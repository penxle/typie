use std::ops::Range;

use editor_model::NodeId;
use editor_resource::FontRegistry;

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

    for run in runs {
        let family_id = font_registry.intern(&run.style.font_family);
        let weight = run.style.font_weight;

        let cp_map = font_registry.codepoint_map(family_id, weight);

        let run_text = &text[run.byte_range.clone()];
        let mut byte_offset = run.byte_range.start;

        for ch in run_text.chars() {
            let char_bytes = ch.len_utf8();
            let char_byte_end = byte_offset + char_bytes;

            let (resolved_family, resolved_weight) = cp_map
                .and_then(|m| m.get(&(ch as u32)).copied())
                .unwrap_or_else(|| {
                    let w = font_registry
                        .nearest_weight(&run.style.font_family, weight)
                        .unwrap_or(weight);
                    (family_id, w)
                });

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
    use editor_resource::FontRegistry;

    use super::*;
    use crate::measure::text::text_run::collect_text_runs;

    fn registry_with_families(families: &[&str]) -> FontRegistry {
        FontRegistry::from_families(families.iter().map(|f| (f.to_string(), vec![400, 700])))
    }

    #[test]
    fn single_run_known_family() {
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("hello") }
            }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        let mut registry = registry_with_families(&["Arial"]);
        let style_runs = resolve_style_runs(&text, &runs, &mut registry);
        assert_eq!(style_runs.len(), 1);
        assert_eq!(registry.resolve(style_runs[0].family), "Arial");
        assert_eq!(style_runs[0].byte_range, 0..5);
        assert_eq!(style_runs[0].font_size, runs[0].style.font_size);
        assert_eq!(style_runs[0].node_id, runs[0].node_id);
    }

    #[test]
    fn unknown_family_uses_fallback() {
        let (doc, p1) = doc! {
            root [font_family("UnknownFont".to_string())] {
                p1: paragraph { text("hello") }
            }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        let mut registry = FontRegistry::new();
        let style_runs = resolve_style_runs(&text, &runs, &mut registry);
        assert_eq!(style_runs.len(), 1);
        assert_eq!(style_runs[0].byte_range, 0..5);
        assert_eq!(registry.resolve(style_runs[0].family), "UnknownFont");
    }

    #[test]
    fn adjacent_runs_same_family_merged() {
        let (doc, p1) = doc! {
            root [font_family("Arial".to_string())] {
                p1: paragraph { text("hello") text(" world") }
            }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);
        let mut registry = registry_with_families(&["Arial"]);
        let style_runs = resolve_style_runs(&text, &runs, &mut registry);
        // Two text nodes have different node_ids, so they won't merge
        assert_eq!(style_runs.len(), 2);
        assert_eq!(style_runs[0].byte_range, 0..5);
        assert_eq!(style_runs[1].byte_range, 5..11);
    }

    #[test]
    fn codepoint_mapping_splits_run() {
        let (doc, p1) = doc! {
            root [font_family("Pretendard".to_string())] {
                p1: paragraph { text("A\u{D55C}B") }
            }
        };
        let node = doc.node(p1).unwrap();
        let (text, runs) = collect_text_runs(&doc, &node);

        let mut registry = registry_with_families(&["Pretendard", "Paperlogy"]);
        let pretendard_id = registry.intern("Pretendard");
        let paperlogy_id = registry.intern("Paperlogy");
        registry.add_codepoint_mapping(pretendard_id, 400, '\u{D55C}' as u32, paperlogy_id, 700);

        let style_runs = resolve_style_runs(&text, &runs, &mut registry);
        assert_eq!(style_runs.len(), 3);
        assert_eq!(registry.resolve(style_runs[0].family), "Pretendard"); // "A"
        assert_eq!(registry.resolve(style_runs[1].family), "Paperlogy"); // "한"
        assert_eq!(style_runs[1].weight, 700);
        assert_eq!(registry.resolve(style_runs[2].family), "Pretendard"); // "B"
    }
}
