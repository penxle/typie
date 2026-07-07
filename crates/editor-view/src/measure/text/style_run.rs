use std::ops::Range;

use editor_resource::{FontRegistry, PLACEHOLDER_WEIGHT, Resolution};

use super::inline::TextRun;

pub(crate) struct StyleRun {
    pub run_index: usize,
    pub byte_range: Range<usize>,
    pub family: u16,
    pub weight: u16,
    pub font_size: f32,
    pub letter_spacing: f32,
    pub line_height: f32,
}

pub(crate) fn resolve_style_runs(
    text: &str,
    runs: &[TextRun],
    font_registry: &mut FontRegistry,
) -> Vec<StyleRun> {
    let mut style_runs: Vec<StyleRun> = Vec::new();

    let placeholder_id = font_registry
        .placeholder_family_id()
        .expect("placeholder family must be registered before resolving style runs");

    for (run_index, run) in runs.iter().enumerate() {
        let requested_family_id = font_registry.intern(&run.style.font_family);
        let weight = run.style.font_weight;

        let run_text = &text[run.byte_range.clone()];
        let mut byte_offset = run.byte_range.start;

        for ch in run_text.chars() {
            let char_byte_end = byte_offset + ch.len_utf8();

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
                    && last.run_index == run_index
                    && last.byte_range.end == byte_offset
            });

            if can_merge {
                style_runs.last_mut().unwrap().byte_range.end = char_byte_end;
            } else {
                style_runs.push(StyleRun {
                    run_index,
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
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, Anchor, Bias, DocLogs, DocView, Modifier, ModifierAttrLog, ModifierAttrOp,
        NodeAttrLog, NodeType, SeqItem, SpanLog, SpanOp, project_document,
    };
    use editor_resource::{
        FontFamily, FontFamilySource, FontRegistry, FontWeight, PLACEHOLDER_WEIGHT,
    };

    use crate::measure::text::inline::collect_text_runs;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        }
    }

    // Root > Paragraph with `children` appended as leaves. Leaf i is Dot(1, 2 + i).
    fn build_logs(children: Vec<SeqItem>) -> DocLogs {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let mut items = vec![(
            p,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, c) in children.into_iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), c));
        }
        logs(&items)
    }

    fn ch(c: char) -> SeqItem {
        SeqItem::Char(c)
    }
    fn leaf(i: u64) -> Dot {
        Dot::new(1, 2 + i)
    }
    fn anc(d: Dot, bias: Bias) -> Anchor {
        Anchor { id: d, bias }
    }

    // Sets a block FontFamily("Arial") on the root so TextRun.style.font_family == "Arial".
    fn with_root_arial(l: &mut DocLogs) {
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(50, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontFamily {
                        value: "Arial".to_string(),
                    },
                },
            )
            .unwrap();
    }

    // Local copy of style_run.rs:94-118 (a private test helper). Full-coverage families.
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
        reg.register_placeholder(&[0u8; 16]);
        let families: Vec<FontFamily> = families
            .iter()
            .map(|(n, w)| family(n, FontFamilySource::Default, w))
            .collect();
        reg.set_fonts(families);
        reg
    }

    // Build the paragraph's TextRuns from a DocLogs.
    // Returns (text, style_runs) given a registry; the DocView must outlive the runs,
    // so we resolve inside this helper while pd/view are alive.
    fn style_runs_of(l: &DocLogs, reg: &mut FontRegistry) -> (String, Vec<StyleRun>, usize) {
        let pd = project_document(l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (text, runs, _tabs) = collect_text_runs(&para);
        let n_runs = runs.len();
        let srs = resolve_style_runs(&text, &runs, reg);
        (text, srs, n_runs)
    }

    #[test]
    fn ready_uses_target_family() {
        let mut l = build_logs(vec![ch('A')]);
        with_root_arial(&mut l);
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let (_t, srs, _n) = style_runs_of(&l, &mut reg);
        assert_eq!(srs.len(), 1);
        assert_eq!(srs[0].family, arial);
        assert_eq!(srs[0].weight, 400);
        assert_eq!(srs[0].run_index, 0);
        assert_eq!(srs[0].byte_range, 0..1);
    }

    #[test]
    fn pending_chunk_only_uses_target_family() {
        // base loaded, chunk 0 not loaded → still the target family.
        let mut l = build_logs(vec![ch('A')]);
        with_root_arial(&mut l);
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 0);
        let (_t, srs, _n) = style_runs_of(&l, &mut reg);
        assert_eq!(srs.len(), 1);
        assert_eq!(srs[0].family, arial);
    }

    #[test]
    fn pending_needs_base_uses_placeholder() {
        // base never loaded → placeholder.
        let mut l = build_logs(vec![ch('A')]);
        with_root_arial(&mut l);
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let placeholder = reg.placeholder_family_id().unwrap();
        let (_t, srs, _n) = style_runs_of(&l, &mut reg);
        assert_eq!(srs.len(), 1);
        assert_eq!(srs[0].family, placeholder);
        assert_eq!(srs[0].weight, PLACEHOLDER_WEIGHT);
    }

    #[test]
    fn missing_uses_placeholder() {
        // Arial covers only 0x0000..=0x00FF; CJK char is outside → placeholder.
        let mut l = build_logs(vec![ch('\u{4E2D}')]);
        with_root_arial(&mut l);
        let mut reg = FontRegistry::new();
        reg.register_placeholder(&[0u8; 16]);
        reg.set_fonts(vec![FontFamily {
            name: "Arial".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
                chunks: vec![vec![0x0000, 0x00FF]],
            }],
        }]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let placeholder = reg.placeholder_family_id().unwrap();
        let (_t, srs, _n) = style_runs_of(&l, &mut reg);
        assert_eq!(srs.len(), 1);
        assert_eq!(srs[0].family, placeholder);
    }

    #[test]
    fn two_text_runs_same_family_split_by_run_index() {
        // "AB" with a Bold span on "B" → two TextRuns (Bold is a synthesis flag, not a
        // shaping field, so both resolve to the SAME Arial 400). The ONLY thing preventing a
        // merge is run_index → exactly two StyleRuns. (B1: forced by a span; without it
        // "AB" would be ONE TextRun → ONE StyleRun.)
        let mut l = build_logs(vec![ch('A'), ch('B')]);
        with_root_arial(&mut l);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(51, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(1), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let (_t, srs, n_runs) = style_runs_of(&l, &mut reg);
        assert_eq!(n_runs, 2, "Bold span splits into two TextRuns");
        assert_eq!(srs.len(), 2);
        assert_eq!(srs[0].run_index, 0);
        assert_eq!(srs[0].byte_range, 0..1);
        assert_eq!(srs[1].run_index, 1);
        assert_eq!(srs[1].byte_range, 1..2);
        assert_eq!(srs[0].family, arial);
        assert_eq!(srs[1].family, arial);
    }

    #[test]
    fn intra_run_font_fallback_splits_same_run_index() {
        // Single TextRun "A中"; Arial covers only Latin → "A"→Arial, "中"→placeholder.
        // Two StyleRuns sharing run_index 0, differing in family, contiguous bytes.
        let mut l = build_logs(vec![ch('A'), ch('\u{4E2D}')]);
        with_root_arial(&mut l);
        let mut reg = FontRegistry::new();
        reg.register_placeholder(&[0u8; 16]);
        reg.set_fonts(vec![FontFamily {
            name: "Arial".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "h".into(),
                chunks: vec![vec![0x0000, 0x00FF]],
            }],
        }]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let placeholder = reg.placeholder_family_id().unwrap();
        let (_t, srs, n_runs) = style_runs_of(&l, &mut reg);
        assert_eq!(n_runs, 1, "no (own,effective) difference → one TextRun");
        assert_eq!(srs.len(), 2);
        assert_eq!(srs[0].run_index, 0);
        assert_eq!(srs[1].run_index, 0);
        assert_eq!(srs[0].family, arial);
        assert_eq!(srs[1].family, placeholder);
        assert_eq!(srs[0].byte_range.end, srs[1].byte_range.start); // contiguous
    }

    #[test]
    fn intra_run_equal_font_merges() {
        // Single TextRun "AB", both Arial → ONE StyleRun byte 0..2.
        let mut l = build_logs(vec![ch('A'), ch('B')]);
        with_root_arial(&mut l);
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let (_t, srs, n_runs) = style_runs_of(&l, &mut reg);
        assert_eq!(n_runs, 1);
        assert_eq!(srs.len(), 1);
        assert_eq!(srs[0].run_index, 0);
        assert_eq!(srs[0].byte_range, 0..2);
    }

    #[test]
    fn within_group_bold_split_at_run_boundary() {
        // "漢字" with ruby "かんじ" over both AND Bold on "漢" → two TextRuns (bold/non-bold)
        // inside ONE ruby group; same Arial family. resolve splits at the run boundary → 2 runs,
        // so no StyleRun spans the two source runs within the group (§2.3/H2 structural invariant).
        let mut l = build_logs(vec![ch('\u{6F22}'), ch('\u{5B57}')]); // 漢 字
        with_root_arial(&mut l);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(52, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::Ruby {
                        text: "かんじ".to_string(),
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(53, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let (_t, srs, n_runs) = style_runs_of(&l, &mut reg);
        assert_eq!(
            n_runs, 2,
            "bold span splits the ruby word into two TextRuns"
        );
        assert_eq!(srs.len(), 2);
        assert_eq!(srs[0].run_index, 0);
        assert_eq!(srs[1].run_index, 1);
    }

    #[test]
    fn cross_ruby_group_runs_never_merge() {
        // "ab" with ruby "x" on "a" and ruby "y" on "b" → two ruby groups AND two TextRuns
        // (distinct own Ruby), both Arial. resolve yields two StyleRuns, neither spanning the
        // two groups — the direct cross-group invariant §2.3 rests on (B3).
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        with_root_arial(&mut l);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(54, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: Modifier::Ruby {
                        text: "x".to_string(),
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(55, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(1), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::Ruby {
                        text: "y".to_string(),
                    },
                },
            )
            .unwrap();
        let mut reg = registry_with_families(&[("Arial", &[400])]);
        let arial = reg.intern_id("Arial").unwrap();
        reg.force_loaded_for_test(arial, 400, 1);
        let (_t, srs, n_runs) = style_runs_of(&l, &mut reg);
        assert_eq!(n_runs, 2, "distinct ruby → two TextRuns");
        assert_eq!(srs.len(), 2);
        assert_eq!(srs[0].run_index, 0);
        assert_eq!(srs[0].byte_range, 0..1);
        assert_eq!(srs[1].run_index, 1);
        assert_eq!(srs[1].byte_range, 1..2);
    }
}
