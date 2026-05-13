use std::ops::Range;

use editor_model::NodeId;
use editor_resource::{FontRegistry, PLACEHOLDER_WEIGHT, Resolution};

use super::text_run::TextRun;

pub struct StyleRun {
    pub node_id: NodeId,
    pub byte_range: Range<usize>,
    pub family: u16,
    pub weight: u16,
    pub font_size: f32,
    pub letter_spacing: f32,
    pub line_height: f32,
}

pub fn resolve_style_runs(
    text: &str,
    runs: &[TextRun],
    font_registry: &mut FontRegistry,
) -> Vec<StyleRun> {
    let mut style_runs: Vec<StyleRun> = Vec::new();

    let placeholder_id = font_registry
        .placeholder_family_id()
        .expect("placeholder family must be registered before resolving style runs");

    for run in runs {
        let requested_family_id = font_registry.intern(&run.style.font_family);
        let weight = run.style.font_weight;

        let run_text = &text[run.byte_range.clone()];
        let mut byte_offset = run.byte_range.start;

        for ch in run_text.chars() {
            let char_bytes = ch.len_utf8();
            let char_byte_end = byte_offset + char_bytes;

            let (resolved_family, resolved_weight) =
                match font_registry.resolve(requested_family_id, weight, ch as u32) {
                    Resolution::Ready(target) => (target.family_id, target.weight),
                    Resolution::Pending {
                        target,
                        needs_base: false,
                    } => (target.family_id, target.weight),
                    Resolution::Pending {
                        needs_base: true, ..
                    }
                    | Resolution::Missing => (placeholder_id, PLACEHOLDER_WEIGHT),
                };

            let can_merge = style_runs.last().is_some_and(|last: &StyleRun| {
                last.family == resolved_family
                    && last.weight == resolved_weight
                    && last.font_size == run.style.font_size
                    && last.letter_spacing == run.style.letter_spacing
                    && last.line_height == run.style.line_height
                    && last.node_id == run.node_id
                    && last.byte_range.end == byte_offset
            });

            if can_merge {
                style_runs.last_mut().unwrap().byte_range.end = char_byte_end;
            } else {
                style_runs.push(StyleRun {
                    node_id: run.node_id,
                    byte_range: byte_offset..char_byte_end,
                    family: resolved_family,
                    weight: resolved_weight,
                    font_size: run.style.font_size,
                    letter_spacing: run.style.letter_spacing,
                    line_height: run.style.line_height,
                });
            }

            byte_offset = char_byte_end;
        }
    }

    style_runs
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_resource::{
        FontFamily, FontFamilySource, FontRegistry, FontWeight, PLACEHOLDER_WEIGHT,
    };

    use super::*;
    use crate::measure::text::text_run::collect_text_runs_for;

    fn family(name: &str, source: FontFamilySource, weights: &[u16]) -> FontFamily {
        FontFamily {
            name: name.into(),
            source,
            weights: weights
                .iter()
                .map(|&w| FontWeight {
                    value: w,
                    hash: format!("h{}_{}", name, w),
                    chunks: vec![vec![0x0000, 0xFFFF]],
                })
                .collect(),
        }
    }

    fn registry_with_families(families: &[(&str, &[u16])]) -> FontRegistry {
        let mut reg = FontRegistry::new();
        reg.register_placeholder(&vec![0u8; 16]);
        let families: Vec<FontFamily> = families
            .iter()
            .map(|(n, w)| family(n, FontFamilySource::Default, w))
            .collect();
        reg.set_fonts(families);
        reg
    }

    #[test]
    fn ready_uses_target_family() {
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("A") }
            }
        };
        let children: Vec<editor_model::NodeRef<'_>> = doc.node(p1).unwrap().children().collect();
        let (text, runs) = collect_text_runs_for(&children);
        let mut registry = registry_with_families(&[("Arial", &[400])]);
        let arial_id = registry.intern_id("Arial").unwrap();
        registry.force_loaded_for_test(arial_id, 400, 1);

        let style_runs = resolve_style_runs(&text, &runs, &mut registry);

        assert_eq!(style_runs.len(), 1);
        assert_eq!(style_runs[0].family, arial_id);
        assert_eq!(style_runs[0].weight, 400);
    }

    #[test]
    fn pending_chunk_only_uses_target_family() {
        // base loaded but chunk not loaded → still use the target family; renderer blanks at rasterize.
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("A") }
            }
        };
        let children: Vec<editor_model::NodeRef<'_>> = doc.node(p1).unwrap().children().collect();
        let (text, runs) = collect_text_runs_for(&children);
        let mut registry = registry_with_families(&[("Arial", &[400])]);
        let arial_id = registry.intern_id("Arial").unwrap();
        // base loaded, chunk 0 not loaded
        registry.force_loaded_for_test(arial_id, 400, 0);

        let style_runs = resolve_style_runs(&text, &runs, &mut registry);

        assert_eq!(style_runs.len(), 1);
        assert_eq!(style_runs[0].family, arial_id);
    }

    #[test]
    fn pending_needs_base_uses_placeholder() {
        // base not loaded at all → placeholder.
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("A") }
            }
        };
        let children: Vec<editor_model::NodeRef<'_>> = doc.node(p1).unwrap().children().collect();
        let (text, runs) = collect_text_runs_for(&children);
        let mut registry = registry_with_families(&[("Arial", &[400])]);

        let style_runs = resolve_style_runs(&text, &runs, &mut registry);

        let placeholder_id = registry.placeholder_family_id().unwrap();
        assert_eq!(style_runs.len(), 1);
        assert_eq!(style_runs[0].family, placeholder_id);
        assert_eq!(style_runs[0].weight, PLACEHOLDER_WEIGHT);
    }

    #[test]
    fn missing_uses_placeholder() {
        // "Arial" registered, but cp outside its coverage (covered family has range 0x0000..=0x00FF only).
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("\u{4E2D}") } // Chinese char outside our narrow coverage
            }
        };
        let children: Vec<editor_model::NodeRef<'_>> = doc.node(p1).unwrap().children().collect();
        let (text, runs) = collect_text_runs_for(&children);

        // Build a registry where Arial only covers 0x0000..=0x00FF and there is no fallback.
        let mut registry = FontRegistry::new();
        registry.register_placeholder(&vec![0u8; 16]);
        registry.set_fonts(vec![FontFamily {
            name: "Arial".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
                chunks: vec![vec![0x0000, 0x00FF]],
            }],
        }]);
        let arial_id = registry.intern_id("Arial").unwrap();
        registry.force_loaded_for_test(arial_id, 400, 1);

        let style_runs = resolve_style_runs(&text, &runs, &mut registry);

        let placeholder_id = registry.placeholder_family_id().unwrap();
        assert_eq!(style_runs.len(), 1);
        assert_eq!(style_runs[0].family, placeholder_id);
    }

    #[test]
    fn adjacent_runs_same_family_merged() {
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("A") text("B") }
            }
        };
        let children: Vec<editor_model::NodeRef<'_>> = doc.node(p1).unwrap().children().collect();
        let (text, runs) = collect_text_runs_for(&children);
        let mut registry = registry_with_families(&[("Arial", &[400])]);
        let arial_id = registry.intern_id("Arial").unwrap();
        registry.force_loaded_for_test(arial_id, 400, 1);

        let style_runs = resolve_style_runs(&text, &runs, &mut registry);

        // Two text nodes have different node_ids, so they won't merge.
        assert_eq!(style_runs.len(), 2);
        assert_eq!(style_runs[0].byte_range, 0..1);
        assert_eq!(style_runs[1].byte_range, 1..2);
        assert_eq!(style_runs[0].family, arial_id);
        assert_eq!(style_runs[1].family, arial_id);
    }
}
