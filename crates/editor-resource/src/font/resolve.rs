use hashbrown::HashMap;

use super::registry::FontRegistry;
use super::weight::match_weight;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodepointMapping {
    pub family_id: u16,
    pub weight: u16,
    pub codepoints: Vec<u32>,
}

/// Sort `weights` by CSS Fonts Level 4 proximity to `target`.
fn weights_by_proximity(weights: &[u16], target: u16) -> Vec<u16> {
    let mut sorted = weights.to_vec();
    sorted.sort_unstable();

    let mut result = Vec::with_capacity(sorted.len());

    if target >= 400 && target <= 500 {
        result.extend(sorted.iter().filter(|&&w| w >= target && w <= 500));
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

pub(crate) fn resolve_codepoint_mappings(
    registry: &FontRegistry,
    family_id: u16,
    weight: u16,
    codepoints: &[u32],
) -> Vec<CodepointMapping> {
    let mut result_map: HashMap<(u16, u16), Vec<u32>> = HashMap::new();
    let mut remaining: Vec<u32> = codepoints.to_vec();

    // Step 1: Primary font - all weights in proximity order
    if let Some(family_name) = registry.resolve_opt(family_id) {
        if let Some(weights) = registry.weights(family_name) {
            let ordered = weights_by_proximity(weights, weight);
            remaining.retain(|&cp| {
                for &w in &ordered {
                    if let Some(manifest) = registry.manifest(family_id, w) {
                        if manifest.has_codepoint(cp) {
                            result_map.entry((family_id, w)).or_default().push(cp);
                            return false;
                        }
                    }
                }
                true
            });
        }
    }

    // Step 2: Fallback chain - 1 weight per family
    for entry in registry.fallback_entries() {
        if remaining.is_empty() {
            break;
        }

        let fallback_weights: Vec<u16> = entry.fonts.iter().map(|f| f.weight).collect();
        let Some(matched_weight) = match_weight(
            &{
                let mut w = fallback_weights.clone();
                w.sort_unstable();
                w
            },
            weight,
        ) else {
            continue;
        };

        let Some(fallback_font) = entry.fonts.iter().find(|f| f.weight == matched_weight) else {
            continue;
        };

        let Some(fallback_family_id) = registry.intern_id(&entry.family_name) else {
            continue;
        };

        remaining.retain(|&cp| {
            if fallback_font.manifest.has_codepoint(cp) {
                result_map
                    .entry((fallback_family_id, matched_weight))
                    .or_default()
                    .push(cp);
                false
            } else {
                true
            }
        });
    }

    result_map
        .into_iter()
        .map(|((fid, w), cps)| CodepointMapping {
            family_id: fid,
            weight: w,
            codepoints: cps,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::font::fallback::{FallbackFont, FallbackFontEntry};
    use crate::font::manifest::FontManifest;

    fn make_manifest(entries: &[(u8, u8, u8)], chunk_count: u16) -> FontManifest {
        let mut chunk_map = vec![0xffu8; 256];
        let mut l2_blocks: BTreeMap<u8, [u8; 256]> = BTreeMap::new();
        for &(hi, lo, chunk_idx) in entries {
            let l2 = l2_blocks.entry(hi).or_insert([0xff; 256]);
            l2[lo as usize] = chunk_idx;
        }
        for (i, (&hi, _)) in l2_blocks.iter().enumerate() {
            chunk_map[hi as usize] = i as u8;
        }
        for (_, block) in &l2_blocks {
            chunk_map.extend_from_slice(block);
        }
        FontManifest::new(chunk_count, chunk_map, vec![])
    }

    fn setup_primary_only() -> (FontRegistry, u16) {
        let mut reg = FontRegistry::new();
        let a = reg.intern("A");

        // A(400) has cp 1 (0x0001)
        reg.add_manifest(a, 400, make_manifest(&[(0x00, 0x01, 0)], 4));
        // A(500) has cp 2 (0x0002)
        reg.add_manifest(a, 500, make_manifest(&[(0x00, 0x02, 1)], 4));
        // A(600) has cp 3 (0x0003)
        reg.add_manifest(a, 600, make_manifest(&[(0x00, 0x03, 2)], 4));

        let mut families = HashMap::default();
        families.insert("A".into(), vec![400, 500, 600]);
        reg.update(families);

        (reg, a)
    }

    fn setup_with_fallbacks() -> (FontRegistry, u16) {
        let (mut reg, a) = setup_primary_only();

        let b_manifest = make_manifest(&[(0x00, 0x04, 0)], 2);
        let c_manifest = make_manifest(&[(0x00, 0x04, 0), (0x00, 0x05, 1)], 2);

        reg.set_fallback_entries(vec![
            FallbackFontEntry {
                family_name: "B".into(),
                fonts: vec![FallbackFont {
                    weight: 300,
                    manifest: b_manifest,
                }],
            },
            FallbackFontEntry {
                family_name: "C".into(),
                fonts: vec![FallbackFont {
                    weight: 400,
                    manifest: c_manifest,
                }],
            },
        ]);

        (reg, a)
    }

    #[test]
    fn primary_exact_weight() {
        let (reg, a) = setup_primary_only();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[1]);
        assert_eq!(
            result,
            vec![CodepointMapping {
                family_id: a,
                weight: 400,
                codepoints: vec![1],
            }]
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
                codepoints: vec![2],
            }]
        );
    }

    #[test]
    fn primary_multiple_weights() {
        let (reg, a) = setup_primary_only();
        let result = resolve_codepoint_mappings(&reg, a, 300, &[1, 2, 3]);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&CodepointMapping {
            family_id: a,
            weight: 400,
            codepoints: vec![1],
        }));
        assert!(result.contains(&CodepointMapping {
            family_id: a,
            weight: 500,
            codepoints: vec![2],
        }));
        assert!(result.contains(&CodepointMapping {
            family_id: a,
            weight: 600,
            codepoints: vec![3],
        }));
    }

    #[test]
    fn primary_miss_falls_to_fallback() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[4]);
        let b = reg.intern_id("B").unwrap();
        assert_eq!(
            result,
            vec![CodepointMapping {
                family_id: b,
                weight: 300,
                codepoints: vec![4],
            }]
        );
    }

    #[test]
    fn fallback_order_preserved() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[4]);
        let b = reg.intern_id("B").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].family_id, b);
        assert_eq!(result[0].weight, 300);
        assert_eq!(result[0].codepoints, vec![4]);
    }

    #[test]
    fn fallback_spread_across_families() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[4, 5]);
        let b = reg.intern_id("B").unwrap();
        let c = reg.intern_id("C").unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&CodepointMapping {
            family_id: b,
            weight: 300,
            codepoints: vec![4],
        }));
        assert!(result.contains(&CodepointMapping {
            family_id: c,
            weight: 400,
            codepoints: vec![5],
        }));
    }

    #[test]
    fn codepoint_in_no_font() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[99]);
        assert!(result.is_empty());
    }

    #[test]
    fn mixed_primary_and_fallback() {
        let (reg, a) = setup_with_fallbacks();
        let result = resolve_codepoint_mappings(&reg, a, 400, &[1, 4]);
        let b = reg.intern_id("B").unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&CodepointMapping {
            family_id: a,
            weight: 400,
            codepoints: vec![1],
        }));
        assert!(result.contains(&CodepointMapping {
            family_id: b,
            weight: 300,
            codepoints: vec![4],
        }));
    }

    #[test]
    fn proximity_order_determines_primary_weight() {
        let (reg, a) = setup_primary_only();
        let result = resolve_codepoint_mappings(&reg, a, 600, &[1]);
        assert_eq!(
            result,
            vec![CodepointMapping {
                family_id: a,
                weight: 400,
                codepoints: vec![1],
            }]
        );
    }
}
