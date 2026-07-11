#[cfg(test)]
use hashbrown::HashMap;

use super::registry::FontRegistry;
use super::resolution::{Resolution, Target};
use super::weight::match_weight;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedTarget {
    pub family_id: u16,
    pub weight: u16,
    pub chunk_id: u16,
}

pub(crate) enum ResolveStep {
    Target(ResolvedTarget),
    AwaitManifest { family_id: u16, weight: u16 },
    NotCovered,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodepointMapping {
    pub family_id: u16,
    pub weight: u16,
    pub chunk_id: u16,
    pub codepoints: Vec<u32>,
}

/// Sort `weights` by CSS Fonts Level 4 proximity to `target`.
fn weights_by_proximity(weights: &[u16], target: u16) -> Vec<u16> {
    let mut sorted = weights.to_vec();
    sorted.sort_unstable();

    let mut result = Vec::with_capacity(sorted.len());

    if (400..=500).contains(&target) {
        result.extend(sorted.iter().filter(|&&w| (target..=500).contains(&w)));
        result.extend(sorted.iter().rev().filter(|&&w| w < target));
        result.extend(sorted.iter().filter(|&&w| w > 500));
    } else if target < 400 {
        result.extend(sorted.iter().rev().filter(|&&w| w <= target));
        result.extend(sorted.iter().filter(|&&w| w > target));
    } else {
        result.extend(sorted.iter().filter(|&&w| w >= target));
        result.extend(sorted.iter().rev().filter(|&&w| w < target));
    }

    result
}

impl FontRegistry {
    pub(crate) fn resolve_one(&self, family_id: u16, weight: u16, cp: u32) -> ResolveStep {
        if let Some(family) = self.family_name_opt(family_id)
            && let Some(ws) = self.weights(family)
        {
            let ordered = weights_by_proximity(ws, weight);
            for w in ordered {
                let Some(manifest) = self.manifest(family_id, w) else {
                    return ResolveStep::AwaitManifest {
                        family_id,
                        weight: w,
                    };
                };
                if let Some(cid) = manifest.chunk_id(cp) {
                    return ResolveStep::Target(ResolvedTarget {
                        family_id,
                        weight: w,
                        chunk_id: cid,
                    });
                }
            }
        }

        for fb_id in self.fallback_family_ids() {
            let Some(fb_name) = self.family_name_opt(fb_id) else {
                continue;
            };
            let Some(ws) = self.weights(fb_name) else {
                continue;
            };
            let Some(fb_w) = match_weight(ws, weight) else {
                continue;
            };
            let Some(manifest) = self.manifest(fb_id, fb_w) else {
                return ResolveStep::AwaitManifest {
                    family_id: fb_id,
                    weight: fb_w,
                };
            };
            if let Some(cid) = manifest.chunk_id(cp) {
                return ResolveStep::Target(ResolvedTarget {
                    family_id: fb_id,
                    weight: fb_w,
                    chunk_id: cid,
                });
            }
        }

        ResolveStep::NotCovered
    }

    #[cfg(test)]
    pub(crate) fn resolve_each_codepoint(
        &self,
        family_id: u16,
        weight: u16,
        codepoints: &[u32],
    ) -> Vec<Option<ResolvedTarget>> {
        codepoints
            .iter()
            .map(|&cp| match self.resolve_one(family_id, weight, cp) {
                ResolveStep::Target(t) => Some(t),
                ResolveStep::AwaitManifest { .. } | ResolveStep::NotCovered => None,
            })
            .collect()
    }

    pub fn resolve(&self, family_id: u16, weight: u16, cp: u32) -> Resolution {
        let resolved = match self.resolve_one(family_id, weight, cp) {
            ResolveStep::Target(t) => t,
            ResolveStep::AwaitManifest { family_id, weight } => {
                return Resolution::AwaitingManifest { family_id, weight };
            }
            ResolveStep::NotCovered => return Resolution::Missing,
        };
        let target = Target {
            family_id: resolved.family_id,
            weight: resolved.weight,
            chunk_id: resolved.chunk_id,
        };

        if !self.is_base_loaded(target.family_id, target.weight) {
            return Resolution::Pending {
                target,
                needs_base: true,
            };
        }

        if self.is_chunk_loaded(target.family_id, target.weight, target.chunk_id) {
            Resolution::Ready(target)
        } else {
            Resolution::Pending {
                target,
                needs_base: false,
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn resolve_codepoint_mappings(
    registry: &FontRegistry,
    family_id: u16,
    weight: u16,
    codepoints: &[u32],
) -> Vec<CodepointMapping> {
    let targets = registry.resolve_each_codepoint(family_id, weight, codepoints);
    let mut grouped: HashMap<(u16, u16, u16), Vec<u32>> = HashMap::default();
    for (i, target_opt) in targets.iter().enumerate() {
        if let Some(t) = target_opt {
            grouped
                .entry((t.family_id, t.weight, t.chunk_id))
                .or_default()
                .push(codepoints[i]);
        }
    }
    grouped
        .into_iter()
        .map(|((fid, w, cid), cps)| CodepointMapping {
            family_id: fid,
            weight: w,
            chunk_id: cid,
            codepoints: cps,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::config::{FontFamily, FontFamilySource, FontWeight};
    use crate::font::data::FontData;
    use crate::font::manifest::FontManifest;
    use crate::font::registry::FontEntry;
    use std::sync::Arc;

    /// chunk index `id`에 단일 coverage를 갖는 chunks 생성 — 앞 인덱스는 빈 chunk.
    fn chunks_at(id: usize, ranges: &[(u32, u32)]) -> Vec<Vec<u32>> {
        let flat: Vec<u32> = ranges.iter().flat_map(|&(s, e)| [s, e]).collect();
        let mut chunks = vec![Vec::new(); id + 1];
        chunks[id] = flat;
        chunks
    }

    fn weight(value: u16, hash: &str) -> FontWeight {
        FontWeight {
            value,
            hash: hash.to_string(),
        }
    }

    fn setup_primary_only() -> (FontRegistry, u16) {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "A".into(),
            source: FontFamilySource::Default,
            weights: vec![
                weight(400, "h400"),
                weight(500, "h500"),
                weight(600, "h600"),
            ],
        }]);
        let a = reg.intern_id("A").unwrap();
        reg.set_manifest(
            a,
            400,
            FontManifest::from_coverages(&chunks_at(0, &[(0x0001, 0x0001)])),
        );
        reg.set_manifest(
            a,
            500,
            FontManifest::from_coverages(&chunks_at(1, &[(0x0002, 0x0002)])),
        );
        reg.set_manifest(
            a,
            600,
            FontManifest::from_coverages(&chunks_at(2, &[(0x0003, 0x0003)])),
        );
        (reg, a)
    }

    fn setup_with_fallbacks() -> (FontRegistry, u16) {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![
            FontFamily {
                name: "A".into(),
                source: FontFamilySource::Default,
                weights: vec![
                    weight(400, "h400"),
                    weight(500, "h500"),
                    weight(600, "h600"),
                ],
            },
            FontFamily {
                name: "B".into(),
                source: FontFamilySource::Fallback,
                weights: vec![weight(300, "hB300")],
            },
            FontFamily {
                name: "C".into(),
                source: FontFamilySource::Fallback,
                weights: vec![weight(400, "hC400")],
            },
        ]);
        let a = reg.intern_id("A").unwrap();
        let b = reg.intern_id("B").unwrap();
        let c = reg.intern_id("C").unwrap();
        reg.set_manifest(
            a,
            400,
            FontManifest::from_coverages(&chunks_at(0, &[(0x0001, 0x0001)])),
        );
        reg.set_manifest(
            a,
            500,
            FontManifest::from_coverages(&chunks_at(1, &[(0x0002, 0x0002)])),
        );
        reg.set_manifest(
            a,
            600,
            FontManifest::from_coverages(&chunks_at(2, &[(0x0003, 0x0003)])),
        );
        reg.set_manifest(
            b,
            300,
            FontManifest::from_coverages(&chunks_at(0, &[(0x0004, 0x0004)])),
        );
        reg.set_manifest(
            c,
            400,
            FontManifest::from_coverages(&chunks_at(0, &[(0x0004, 0x0004), (0x0005, 0x0005)])),
        );
        (reg, a)
    }

    #[test]
    fn weights_by_proximity_normal_range() {
        // target in [400, 500]: [target..=500] asc, then <target desc, then >500 asc.
        let w = [100, 300, 400, 500, 700, 900];
        assert_eq!(
            weights_by_proximity(&w, 400),
            vec![400, 500, 300, 100, 700, 900]
        );
    }

    #[test]
    fn resolve_one_primary_exact_weight() {
        let (reg, a) = setup_primary_only();
        let ResolveStep::Target(t) = reg.resolve_one(a, 400, 1) else {
            panic!("expected Target");
        };
        assert_eq!(
            t,
            ResolvedTarget {
                family_id: a,
                weight: 400,
                chunk_id: 0,
            }
        );
    }

    #[test]
    fn primary_different_weight() {
        let (reg, a) = setup_primary_only();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[2]);
        assert_eq!(
            result,
            vec![CodepointMapping {
                family_id: a,
                weight: 500,
                chunk_id: 1,
                codepoints: vec![2],
            }]
        );
    }

    #[test]
    fn primary_miss_falls_to_fallback() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[4]);
        assert_eq!(result.len(), 1);
        let b = reg.intern_id("B").unwrap();
        assert_eq!(result[0].family_id, b);
        assert_eq!(result[0].weight, 300);
        assert_eq!(result[0].chunk_id, 0);
        assert_eq!(result[0].codepoints, vec![4]);
    }

    #[test]
    fn codepoint_in_no_font() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[99]);
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_missing_when_no_font_covers_cp() {
        let (reg, a) = setup_primary_only();
        assert_eq!(reg.resolve(a, 400, 0x9999), Resolution::Missing);
    }

    #[test]
    fn resolve_pending_needs_base_before_base_load() {
        let (reg, a) = setup_primary_only();

        let Resolution::Pending { target, needs_base } = reg.resolve(a, 400, 0x0001) else {
            panic!("expected Pending");
        };
        assert_eq!(
            target,
            Target {
                family_id: a,
                weight: 400,
                chunk_id: 0,
            }
        );
        assert!(needs_base, "base is not loaded yet");
    }

    #[test]
    fn resolve_pending_needs_chunk_after_base_only() {
        let (mut reg, a) = setup_primary_only();
        let key = (a, 400u16);
        reg.font_entries.insert(
            key,
            FontEntry {
                data: Arc::new(FontData::new(vec![0u8; 20])),
                split_offset: 8,
            },
        );
        reg.font_versions.insert(key, 0);

        let Resolution::Pending { target, needs_base } = reg.resolve(a, 400, 0x0001) else {
            panic!("expected Pending");
        };
        assert_eq!(
            target,
            Target {
                family_id: a,
                weight: 400,
                chunk_id: 0,
            }
        );
        assert!(!needs_base, "base is loaded; only chunk missing");
    }

    #[test]
    fn resolve_ready_after_base_and_chunk() {
        let (mut reg, a) = setup_primary_only();
        let key = (a, 400u16);
        reg.font_entries.insert(
            key,
            FontEntry {
                data: Arc::new(FontData::new(vec![0u8; 20])),
                split_offset: 8,
            },
        );
        reg.font_versions.insert(key, 0);
        reg.loaded_chunks.insert(key, vec![true]);

        let Resolution::Ready(target) = reg.resolve(a, 400, 0x0001) else {
            panic!("expected Ready");
        };
        assert_eq!(
            target,
            Target {
                family_id: a,
                weight: 400,
                chunk_id: 0,
            }
        );
    }

    #[test]
    fn resolve_awaits_manifest_before_deciding_coverage() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "A".into(),
            source: FontFamilySource::Default,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h400".into(),
                },
                FontWeight {
                    value: 700,
                    hash: "h700".into(),
                },
            ],
        }]);
        let a = reg.intern_id("A").unwrap();

        assert_eq!(
            reg.resolve(a, 400, 0x41),
            Resolution::AwaitingManifest {
                family_id: a,
                weight: 400
            },
            "best-proximity weight의 manifest부터 요구한다"
        );

        reg.set_manifest(a, 400, FontManifest::from_coverages(&[vec![0x50, 0x50]]));
        assert_eq!(
            reg.resolve(a, 400, 0x41),
            Resolution::AwaitingManifest {
                family_id: a,
                weight: 700
            },
            "400이 미커버로 판명되면 다음 후보 700의 manifest를 요구한다"
        );

        reg.set_manifest(a, 700, FontManifest::from_coverages(&[vec![0x41, 0x41]]));
        assert!(matches!(
            reg.resolve(a, 400, 0x41),
            Resolution::Pending { target, needs_base: true } if target.weight == 700
        ));
    }

    #[test]
    fn resolve_awaits_fallback_manifest_only_after_primary_exhausted() {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![
            FontFamily {
                name: "A".into(),
                source: FontFamilySource::Default,
                weights: vec![FontWeight {
                    value: 400,
                    hash: "hA".into(),
                }],
            },
            FontFamily {
                name: "FB".into(),
                source: FontFamilySource::Fallback,
                weights: vec![FontWeight {
                    value: 400,
                    hash: "hFB".into(),
                }],
            },
        ]);
        let a = reg.intern_id("A").unwrap();
        let fb = reg.intern_id("FB").unwrap();
        reg.set_manifest(a, 400, FontManifest::from_coverages(&[vec![0x50, 0x50]]));

        assert_eq!(
            reg.resolve(a, 400, 0x41),
            Resolution::AwaitingManifest {
                family_id: fb,
                weight: 400
            }
        );

        reg.set_manifest(fb, 400, FontManifest::from_coverages(&[vec![0x60, 0x60]]));
        assert_eq!(reg.resolve(a, 400, 0x41), Resolution::Missing);
    }
}
