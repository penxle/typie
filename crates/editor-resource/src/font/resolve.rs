#[cfg(test)]
use hashbrown::HashMap;

use super::manifest::FontManifest;
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

/// Codepoints the shaper renders invisibly regardless of font support
/// (ZWJ/ZWNJ, variation selectors, bidi controls, …). Must stay in sync with
/// the shaper's default-ignorable set so cluster font matching never rejects a
/// font over a codepoint the shaper would hide anyway.
fn is_default_ignorable(cp: u32) -> bool {
    match cp >> 16 {
        0x00 => matches!(
            cp,
            0x00AD
                | 0x034F
                | 0x061C
                | 0x115F..=0x1160
                | 0x17B4..=0x17B5
                | 0x180B..=0x180E
                | 0x200B..=0x200F
                | 0x202A..=0x202E
                | 0x2060..=0x206F
                | 0x3164
                | 0xFE00..=0xFE0F
                | 0xFEFF
                | 0xFFA0
                | 0xFFF0..=0xFFF8
        ),
        0x01 => matches!(cp, 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A),
        0x0E => matches!(cp, 0xE0000..=0xE0FFF),
        _ => false,
    }
}

impl FontRegistry {
    fn resolve_step_where(
        &self,
        family_id: u16,
        weight: u16,
        covers: impl Fn(&FontManifest) -> Option<u16>,
    ) -> ResolveStep {
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
                if let Some(cid) = covers(manifest) {
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
            if let Some(cid) = covers(manifest) {
                return ResolveStep::Target(ResolvedTarget {
                    family_id: fb_id,
                    weight: fb_w,
                    chunk_id: cid,
                });
            }
        }

        ResolveStep::NotCovered
    }

    pub(crate) fn resolve_one(&self, family_id: u16, weight: u16, cp: u32) -> ResolveStep {
        self.resolve_step_where(family_id, weight, |manifest| manifest.chunk_id(cp))
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

    /// Resolves a whole grapheme cluster to one font: the first family in the
    /// chain covering every significant (non-default-ignorable) codepoint. If
    /// no family covers them all, the cluster follows its base character so it
    /// still maps atomically to a single font.
    pub fn resolve_cluster(&self, family_id: u16, weight: u16, cps: &[u32]) -> Resolution {
        let significant: Vec<u32> = cps
            .iter()
            .copied()
            .filter(|&cp| !is_default_ignorable(cp))
            .collect();

        let Some(&first_significant) = significant.first() else {
            return match cps.first() {
                Some(&cp) => self.resolve(family_id, weight, cp),
                None => Resolution::Missing,
            };
        };
        if cps.len() == 1 {
            return self.resolve(family_id, weight, first_significant);
        }

        let step = self.resolve_step_where(family_id, weight, |manifest| {
            let cid = manifest.chunk_id(first_significant)?;
            significant[1..]
                .iter()
                .all(|&cp| manifest.chunk_id(cp).is_some())
                .then_some(cid)
        });

        match step {
            ResolveStep::Target(t) => {
                let target = Target {
                    family_id: t.family_id,
                    weight: t.weight,
                    chunk_id: t.chunk_id,
                };
                if !self.is_base_loaded(target.family_id, target.weight) {
                    return Resolution::Pending {
                        target,
                        needs_base: true,
                    };
                }
                let manifest = self.manifest(target.family_id, target.weight);
                let all_chunks_loaded = significant.iter().all(|&cp| {
                    manifest.and_then(|m| m.chunk_id(cp)).is_none_or(|cid| {
                        self.is_chunk_loaded(target.family_id, target.weight, cid)
                    })
                });
                if all_chunks_loaded {
                    Resolution::Ready(target)
                } else {
                    Resolution::Pending {
                        target,
                        needs_base: false,
                    }
                }
            }
            ResolveStep::AwaitManifest { family_id, weight } => {
                Resolution::AwaitingManifest { family_id, weight }
            }
            ResolveStep::NotCovered => self.resolve(family_id, weight, first_significant),
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

    #[test]
    fn default_ignorable_classification() {
        for cp in [0x200C, 0x200D, 0xFE0E, 0xFE0F, 0x00AD, 0xE0001, 0x180B] {
            assert!(is_default_ignorable(cp), "U+{cp:04X} must be ignorable");
        }
        for cp in ['a' as u32, '한' as u32, 0x1F636, 0x0301, 0x20E3] {
            assert!(!is_default_ignorable(cp), "U+{cp:04X} must be significant");
        }
    }

    // "E"(emoji 폰트 역할): chunk0=1F636, chunk1=FE0F 커버. base 로드 완료.
    fn cluster_registry(loaded_chunks: Vec<bool>) -> (FontRegistry, u16) {
        let mut reg = FontRegistry::new();
        reg.set_fonts(vec![FontFamily {
            name: "E".into(),
            source: FontFamilySource::Default,
            weights: vec![weight(400, "hE")],
        }]);
        let e = reg.intern_id("E").unwrap();
        reg.set_manifest(
            e,
            400,
            FontManifest::from_coverages(&[
                vec![0x1F32B, 0x1F32B, 0x1F636, 0x1F636],
                vec![0x200D, 0x200D, 0xFE0F, 0xFE0F],
            ]),
        );
        let key = (e, 400u16);
        reg.font_entries.insert(
            key,
            FontEntry {
                data: Arc::new(FontData::new(vec![0u8; 20])),
                split_offset: 8,
            },
        );
        reg.font_versions.insert(key, 0);
        reg.loaded_chunks.insert(key, loaded_chunks);
        (reg, e)
    }

    #[test]
    fn resolve_cluster_ready_ignores_unloaded_ignorable_chunks() {
        // significant(1F636, 1F32B)의 chunk0만 로드되면 Ready — ZWJ/FE0F의
        // chunk1은 per-codepoint 로딩이 이 가족에서 요청하지 않으므로 게이트에서
        // 제외해야 영구 Pending에 빠지지 않는다.
        let (reg, e) = cluster_registry(vec![true, false]);
        let cps = [0x1F636, 0x200D, 0x1F32B, 0xFE0F];
        assert!(matches!(
            reg.resolve_cluster(e, 400, &cps),
            Resolution::Ready(t) if t.family_id == e
        ));
    }

    #[test]
    fn resolve_cluster_pending_when_significant_chunk_unloaded() {
        let (reg, e) = cluster_registry(vec![false, true]);
        let cps = [0x1F636, 0x200D, 0x1F32B, 0xFE0F];
        assert!(matches!(
            reg.resolve_cluster(e, 400, &cps),
            Resolution::Pending {
                needs_base: false,
                ..
            }
        ));
    }

    #[test]
    fn resolve_cluster_uncovered_falls_back_to_base_codepoint() {
        // 유효 codepoint 일부(0x41)가 미커버 → cluster는 base codepoint(1F636)
        // 단독 해석으로 폴백한다.
        let (reg, e) = cluster_registry(vec![true, true]);
        let cps = [0x1F636, 0x41];
        assert!(matches!(
            reg.resolve_cluster(e, 400, &cps),
            Resolution::Ready(t) if t.family_id == e
        ));
    }
}
