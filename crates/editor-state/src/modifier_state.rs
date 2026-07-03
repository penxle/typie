use std::collections::BTreeMap;

use editor_common::Tri;
use editor_model::{
    Modifier, ModifierState, ModifierType, NodeType, NodeView, Schema, text_style_default_modifier,
};
use strum::IntoEnumIterator;

use crate::modifier_resolution::caret_modifiers;
use crate::pending_modifier::PendingModifier;
use crate::projected_state::ProjectedState;
use crate::selection::ResolvedSelection;
use crate::selection::Selection;
use crate::traversal;

fn modifier_for_caret_state(
    caret: &BTreeMap<ModifierType, Modifier>,
    textblock: &NodeView,
    ty: ModifierType,
) -> Option<Modifier> {
    if !modifier_applies_at_caret(textblock, ty) {
        return None;
    }
    if let Some(modifier) = caret.get(&ty) {
        return Some(modifier.clone());
    }
    let default = text_style_default_modifier(ty)?;
    (if Schema::modifier_spec(ty).inheritable {
        textblock.effective().get(&ty).cloned()
    } else {
        None
    })
    .or(Some(default))
}

fn modifier_applies_at_caret(textblock: &NodeView, ty: ModifierType) -> bool {
    let target = &Schema::modifier_spec(ty).target;
    let targets = target.rightmost_node_types();
    let block_path = textblock_type_path(textblock);
    if targets.contains(&textblock.node_type()) && target.matches(&block_path) {
        return true;
    }
    targets.contains(&NodeType::Text)
        && Schema::node_spec(textblock.node_type())
            .content
            .matches(NodeType::Text)
}

fn textblock_type_path(textblock: &NodeView) -> Vec<NodeType> {
    let mut path: Vec<_> = textblock.ancestors().map(|n| n.node_type()).collect();
    path.reverse();
    path
}

/// Interns one type-path entry per distinct `(host, node type)` — a handful
/// (blocks × leaf types) — so per-type target applicability is tested once per
/// distinct path, not per node.
fn intern_path(
    entries: &mut Vec<(NodeType, Vec<NodeType>)>,
    index: &mut std::collections::HashMap<(editor_crdt::Dot, NodeType), usize>,
    host: &NodeView,
    ty: NodeType,
    is_leaf: bool,
) -> usize {
    *index.entry((host.id(), ty)).or_insert_with(|| {
        let mut p: Vec<NodeType> = host.ancestors().map(|n| n.node_type()).collect();
        p.reverse();
        if is_leaf {
            p.push(ty);
        }
        entries.push((ty, p));
        entries.len() - 1
    })
}

/// Single pass over the range, aggregating uniform groups: a block fully inside
/// the selection contributes its run segments — one `(effective, leaf count)`
/// group per uniform stretch — instead of one entry per leaf, so a whole-document
/// selection aggregates O(segments), not O(leaves). Boundary blocks keep the
/// exact per-leaf slot filter. Every type's verdict then follows from counts —
/// how many applicable nodes there are, how many carried an explicit value, and
/// whether those explicit values agreed.
pub fn resolve_modifier_state_in_range(rs: &ResolvedSelection) -> ModifierState {
    struct Explicit<'a> {
        value: &'a Modifier,
        conflicting: bool,
        count: usize,
    }

    let mut out = ModifierState::default();
    let blocks = traversal::blocks_in_range(rs);

    let mut entries: Vec<(NodeType, Vec<NodeType>)> = Vec::new();
    let mut index: std::collections::HashMap<(editor_crdt::Dot, NodeType), usize> =
        std::collections::HashMap::new();
    // (path idx, effective, node count, is_leaf) — blocks count 1, leaf groups
    // count a whole uniform stretch.
    let mut groups: Vec<(usize, &BTreeMap<ModifierType, Modifier>, usize, bool)> = Vec::new();

    for b in &blocks {
        let pi = intern_path(&mut entries, &mut index, b, b.node_type(), false);
        groups.push((pi, b.effective(), 1, false));
    }

    for b in &blocks {
        if b.leaf_child_count() == 0 {
            continue;
        }
        // The bulk path pairs run segments with leaf types from the child list;
        // it is sound only when the segments cover exactly the block's leaves.
        let bulk = traversal::contains_subtree(rs, b)
            && b.run_groups().map(|(_, n)| n).sum::<usize>() == b.leaf_child_count();
        if bulk {
            let mut types = b
                .children()
                .filter_map(|c| match c {
                    editor_model::ChildView::Leaf(l) => Some(l.node_type()),
                    editor_model::ChildView::Block(_) => None,
                })
                .peekable();
            for (eff, len) in b.run_groups() {
                let mut remaining = len;
                while remaining > 0 {
                    let Some(ty) = types.next() else {
                        break;
                    };
                    let mut n = 1;
                    while n < remaining && types.peek() == Some(&ty) {
                        types.next();
                        n += 1;
                    }
                    let pi = intern_path(&mut entries, &mut index, b, ty, true);
                    groups.push((pi, eff, n, true));
                    remaining -= n;
                }
            }
        } else {
            for (slot, l) in traversal::leaves_in_block_range(rs, b) {
                let Some(st) = b.leaf_state_at(slot) else {
                    continue;
                };
                let pi = intern_path(&mut entries, &mut index, b, l.node_type(), true);
                groups.push((pi, st.eff, 1, true));
            }
        }
    }

    let apps: BTreeMap<ModifierType, Vec<bool>> = ModifierType::iter()
        .map(|ty| {
            let target = &Schema::modifier_spec(ty).target;
            let targets = target.rightmost_node_types();
            let app: Vec<bool> = entries
                .iter()
                .map(|(nt, path)| targets.contains(nt) && target.matches(path))
                .collect();
            (ty, app)
        })
        .collect();

    let mut path_count = vec![0usize; entries.len()];
    let bold_applicable = &apps[&ModifierType::Bold];
    let mut explicit: BTreeMap<ModifierType, Explicit> = BTreeMap::new();
    let (mut bold_any_applicable, mut bold_all, mut bold_any) = (false, true, false);
    for &(pi, eff, n, is_leaf) in &groups {
        path_count[pi] += n;
        for (ty, m) in eff {
            if !apps.get(ty).is_some_and(|a| a[pi]) {
                continue;
            }
            explicit
                .entry(*ty)
                .and_modify(|e| {
                    e.count += n;
                    if e.value != m {
                        e.conflicting = true;
                    }
                })
                .or_insert(Explicit {
                    value: m,
                    conflicting: false,
                    count: n,
                });
        }
        if bold_applicable[pi] {
            bold_any_applicable = true;
            let bold = is_leaf && map_is_bold(eff);
            if bold {
                bold_any = true;
            } else {
                bold_all = false;
            }
        }
    }

    for ty in ModifierType::iter() {
        let applicable = &apps[&ty];
        let applicable_count: usize = path_count
            .iter()
            .zip(applicable)
            .filter(|(_, a)| **a)
            .map(|(c, _)| c)
            .sum();
        if applicable_count == 0 {
            continue;
        }
        let sparse_absence_is_neutral = matches!(ty, ModifierType::Link);
        let ty_default = text_style_default_modifier(ty);
        let ex = explicit.get(&ty);
        if ex.is_some_and(|e| e.conflicting) {
            out.set_mixed(ty);
            continue;
        }
        let defaulted = applicable_count - ex.map_or(0, |e| e.count);
        match (ex.map(|e| e.value), ty_default) {
            (Some(e), _) if defaulted == 0 => out.set_uniform(e),
            (Some(e), Some(d)) => {
                if *e == d {
                    out.set_uniform(e);
                } else {
                    out.set_mixed(ty);
                }
            }
            (Some(e), None) => {
                if sparse_absence_is_neutral {
                    out.set_uniform(e);
                } else {
                    out.set_mixed(ty);
                }
            }
            (None, Some(d)) => out.set_uniform(&d),
            (None, None) => {}
        }
    }

    out.effective_bold = if !bold_any_applicable {
        Tri::Absent
    } else if bold_all {
        Tri::Uniform { value: () }
    } else if bold_any {
        Tri::Mixed
    } else {
        Tri::Absent
    };
    out
}

fn is_bold_set<'a>(mut mods: impl Iterator<Item = &'a Modifier>) -> bool {
    mods.any(|m| {
        matches!(m, Modifier::Bold) || matches!(m, Modifier::FontWeight { value } if *value >= 700)
    })
}

fn map_is_bold(eff: &BTreeMap<ModifierType, Modifier>) -> bool {
    matches!(eff.get(&ModifierType::Bold), Some(Modifier::Bold))
        || matches!(
            eff.get(&ModifierType::FontWeight),
            Some(Modifier::FontWeight { value }) if *value >= 700
        )
}

pub fn resolve_modifier_state(
    state: &ProjectedState,
    sel: &Selection,
    pending: &[PendingModifier],
) -> Option<ModifierState> {
    let view = state.view();
    let rs = sel.resolve(&view)?;
    if rs.is_collapsed() {
        let caret = caret_modifiers(state, &sel.head, pending);
        let modifiers = if let Some(node) = view.node(sel.head.node)
            && Schema::node_spec(node.node_type()).is_textblock()
        {
            let mut modifiers = BTreeMap::new();
            for ty in ModifierType::iter() {
                if let Some(modifier) = modifier_for_caret_state(&caret, &node, ty) {
                    modifiers.insert(ty, modifier);
                }
            }
            modifiers
        } else {
            caret
        };
        let mut out = ModifierState::default();
        for m in modifiers.values() {
            out.set_uniform(m);
        }
        if is_bold_set(modifiers.values()) {
            out.effective_bold = Tri::Uniform { value: () };
        }
        Some(out)
    } else {
        Some(resolve_modifier_state_in_range(&rs))
    }
}

#[cfg(test)]
mod tests {
    use editor_common::Tri;
    use editor_crdt::{Dot, ListOp, OpGraph};
    use editor_model::{
        Anchor, Bias, EditOp, Modifier, ModifierAttrOp, ModifierType, NodeType, SeqItem, SpanOp,
    };

    use crate::pending_modifier::PendingModifier;
    use crate::projected_state::ProjectedState;
    use crate::resolve_modifier_state;
    use crate::{Position, selection::Selection};

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block { node_type, parents },
        })
    }

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    fn simple_para_state(chars: &[char]) -> (ProjectedState, Dot, Dot) {
        let mut graph = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let para = graph
            .add_mut(seq_block(0, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        for (i, c) in chars.iter().enumerate() {
            graph.add_mut(seq_char(1 + i, *c)).unwrap();
        }
        let state = ProjectedState::from_graph(graph).unwrap();
        (state, root, para)
    }

    fn fold_title_state() -> (ProjectedState, Dot) {
        let mut graph = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let fold = graph
            .add_mut(seq_block(0, NodeType::Fold, vec![root]))
            .unwrap()
            .id;
        let title = graph
            .add_mut(seq_block(1, NodeType::FoldTitle, vec![root, fold]))
            .unwrap()
            .id;
        let content = graph
            .add_mut(seq_block(2, NodeType::FoldContent, vec![root, fold]))
            .unwrap()
            .id;
        graph
            .add_mut(seq_block(3, NodeType::Paragraph, vec![root, fold, content]))
            .unwrap();
        (ProjectedState::from_graph(graph).unwrap(), title)
    }

    fn sel(anchor: (Dot, usize), head: (Dot, usize)) -> Selection {
        Selection::new(
            Position::new(anchor.0, anchor.1),
            Position::new(head.0, head.1),
        )
    }

    fn collapsed(node: Dot, offset: usize) -> Selection {
        Selection::collapsed(Position::new(node, offset))
    }

    fn first_leaf_dot(state: &ProjectedState, para: Dot) -> Dot {
        let view = state.view();
        let p = view.node(para).unwrap();
        p.children()
            .next()
            .and_then(|c| {
                if let editor_model::ChildView::Leaf(l) = c {
                    Some(l.dot())
                } else {
                    None
                }
            })
            .unwrap()
    }

    // §4.1: collapsed uniform — caret inside a bold run → bold == Uniform; pending applies
    #[test]
    fn test_1_collapsed_uniform_bold() {
        let (mut state, _root, para) = simple_para_state(&['a', 'b']);
        // get the dot of 'a' leaf
        let leaf_a = first_leaf_dot(&state, para);
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();

        // caret at offset 1 (after 'a', which is bold)
        let s = collapsed(para, 1);
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.bold,
            Tri::Uniform { value: () },
            "collapsed caret inside bold run → Uniform bold"
        );
        assert_eq!(
            ms.effective_bold,
            Tri::Uniform { value: () },
            "effective_bold also Uniform"
        );
    }

    // §4.1 with pending: pending italic applies to collapsed caret
    #[test]
    fn test_1b_collapsed_pending_applies() {
        let (state, _root, para) = simple_para_state(&['a']);
        let s = collapsed(para, 1);
        let ms = resolve_modifier_state(
            &state,
            &s,
            &[PendingModifier::Set {
                modifier: Modifier::Italic,
            }],
        )
        .unwrap();
        assert_eq!(ms.italic, Tri::Uniform { value: () });
    }

    #[test]
    fn range_clear_inheritable_value_reports_resolved_default_not_parent() {
        let (mut state, root, para) = simple_para_state(&['a']);
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: root,
                modifier: Modifier::FontWeight { value: 500 },
            }))
            .unwrap();
        let leaf_a = first_leaf_dot(&state, para);
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::FontWeight { value: 700 },
            }))
            .unwrap();
        state
            .apply(EditOp::Span(SpanOp::ClearSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier_type: ModifierType::FontWeight,
            }))
            .unwrap();

        let s = sel((para, 0), (para, 1));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.font_weight,
            Tri::Uniform {
                value: editor_model::FontWeightValue { value: 400 }
            }
        );
        assert_eq!(ms.effective_bold, Tri::Absent);
    }

    #[test]
    fn range_clear_inheritable_font_family_reports_pretendard_default() {
        let (mut state, root, para) = simple_para_state(&['a']);
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: root,
                modifier: Modifier::FontFamily {
                    value: "Pretendard".to_string(),
                },
            }))
            .unwrap();
        let leaf_a = first_leaf_dot(&state, para);
        state
            .apply(EditOp::Span(SpanOp::RemoveSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier_type: ModifierType::FontFamily,
            }))
            .unwrap();

        let s = sel((para, 0), (para, 1));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.font_family,
            Tri::Uniform {
                value: editor_model::FontFamilyValue {
                    value: "Pretendard".to_string()
                }
            }
        );
    }

    #[test]
    fn collapsed_pending_unset_inheritable_value_reports_inserted_resolved_value() {
        let (mut state, root, para) = simple_para_state(&['a']);
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: root,
                modifier: Modifier::FontWeight { value: 500 },
            }))
            .unwrap();
        let leaf_a = {
            let view = state.view();
            let p = view.node(para).unwrap();
            p.children()
                .next()
                .and_then(|c| {
                    if let editor_model::ChildView::Leaf(l) = c {
                        Some(l.dot())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::FontWeight { value: 700 },
            }))
            .unwrap();

        let s = collapsed(para, 1);
        let ms = resolve_modifier_state(
            &state,
            &s,
            &[PendingModifier::Unset {
                ty: ModifierType::FontWeight,
            }],
        )
        .unwrap();
        assert_eq!(
            ms.font_weight,
            Tri::Uniform {
                value: editor_model::FontWeightValue { value: 500 }
            }
        );
        assert_eq!(ms.effective_bold, Tri::Absent);
    }

    #[test]
    fn collapsed_fold_title_does_not_report_line_height_default() {
        let (state, title) = fold_title_state();
        let s = collapsed(title, 0);
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.line_height, Tri::Absent);
        assert_eq!(
            ms.font_weight,
            Tri::Uniform {
                value: editor_model::FontWeightValue { value: 500 }
            }
        );
    }

    // §4.2: collapsed inherits — caret in para under root[font_size] → font_size Uniform
    #[test]
    fn test_2_collapsed_inherits_font_size() {
        let (mut state, root, para) = simple_para_state(&['x']);
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: root,
                modifier: Modifier::FontSize { value: 1600 },
            }))
            .unwrap();
        let s = collapsed(para, 1);
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.font_size,
            Tri::Uniform {
                value: editor_model::FontSizeValue { value: 1600 }
            }
        );
    }

    // §4.3: range uniform — selection fully inside one bold run
    #[test]
    fn test_3_range_uniform_bold() {
        let (mut state, _root, para) = simple_para_state(&['a', 'b', 'c']);
        let (leaf_a, leaf_b, leaf_c) = {
            let view = state.view();
            let p = view.node(para).unwrap();
            let mut it = p.children().filter_map(|c| {
                if let editor_model::ChildView::Leaf(l) = c {
                    Some(l.dot())
                } else {
                    None
                }
            });
            (it.next().unwrap(), it.next().unwrap(), it.next().unwrap())
        };
        // span all three chars with Bold
        for (&start_id, &end_id) in [(&leaf_a, &leaf_a), (&leaf_b, &leaf_b), (&leaf_c, &leaf_c)] {
            state
                .apply(EditOp::Span(SpanOp::AddSpan {
                    start: Anchor {
                        id: start_id,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: end_id,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                }))
                .unwrap();
        }
        let s = sel((para, 0), (para, 3));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.bold, Tri::Uniform { value: () });
        assert_eq!(ms.effective_bold, Tri::Uniform { value: () });
    }

    // §4.3b: selecting EXACTLY a bold run [hello|world|hi], offsets [5,10) covering
    // only "world", must report Uniform bold — the `to`-boundary leaf (first char
    // of "hi") must not be over-collected.
    #[test]
    fn test_3b_range_uniform_bold_exact_span() {
        let chars: Vec<char> = "helloworldhi".chars().collect();
        let (mut state, _root, para) = simple_para_state(&chars);
        let leaves: Vec<Dot> = {
            let view = state.view();
            let p = view.node(para).unwrap();
            p.children()
                .filter_map(|c| {
                    if let editor_model::ChildView::Leaf(l) = c {
                        Some(l.dot())
                    } else {
                        None
                    }
                })
                .collect()
        };
        // bold over "world" = leaf indices 5..=9
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaves[5],
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaves[9],
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();

        let exact = sel((para, 5), (para, 10));
        let ms = resolve_modifier_state(&state, &exact, &[]).unwrap();
        assert_eq!(
            ms.bold,
            Tri::Uniform { value: () },
            "selecting exactly the bold run [5,10) must be Uniform, not Mixed"
        );
        assert_eq!(ms.effective_bold, Tri::Uniform { value: () });

        // reducing the end by one char was the user's workaround — also Uniform
        let shorter = sel((para, 5), (para, 9));
        assert_eq!(
            resolve_modifier_state(&state, &shorter, &[]).unwrap().bold,
            Tri::Uniform { value: () }
        );
    }

    // §4.4: range mixed — selection spanning bold + non-bold chars
    #[test]
    fn test_4_range_mixed_bold() {
        let (mut state, _root, para) = simple_para_state(&['a', 'b']);
        let leaf_a = {
            let view = state.view();
            let p = view.node(para).unwrap();
            p.children()
                .next()
                .and_then(|c| {
                    if let editor_model::ChildView::Leaf(l) = c {
                        Some(l.dot())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        // only 'a' is bold
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();

        let s = sel((para, 0), (para, 2));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.bold, Tri::Mixed, "bold+plain in range → Mixed");
        assert_eq!(ms.effective_bold, Tri::Mixed, "effective_bold Mixed");
    }

    // §4.5: range absent — no bold at all
    #[test]
    fn test_5_range_absent_bold() {
        let (state, _root, para) = simple_para_state(&['a', 'b']);
        let s = sel((para, 0), (para, 2));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.bold, Tri::Absent);
        assert_eq!(ms.effective_bold, Tri::Absent);
    }

    // §4.6: Link sparse-neutral — [link "A"][plain][link "A"] → Uniform
    #[test]
    fn test_6_link_sparse_neutral_uniform() {
        let (mut state, _root, para) = simple_para_state(&['a', 'b', 'c']);
        let (leaf_a, _leaf_b, leaf_c) = {
            let view = state.view();
            let p = view.node(para).unwrap();
            let mut it = p.children().filter_map(|c| {
                if let editor_model::ChildView::Leaf(l) = c {
                    Some(l.dot())
                } else {
                    None
                }
            });
            (it.next().unwrap(), it.next().unwrap(), it.next().unwrap())
        };
        let link_a = Modifier::Link {
            href: "https://a.example".to_string(),
        };
        for id in [leaf_a, leaf_c] {
            state
                .apply(EditOp::Span(SpanOp::AddSpan {
                    start: Anchor {
                        id,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id,
                        bias: Bias::After,
                    },
                    modifier: link_a.clone(),
                }))
                .unwrap();
        }
        let s = sel((para, 0), (para, 3));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        // plain 'b' in the middle is sparse-absent, so Link stays Uniform
        assert_eq!(
            ms.link,
            Tri::Uniform {
                value: editor_model::LinkValue {
                    href: "https://a.example".to_string()
                }
            }
        );
    }

    // §4.6b: differing hrefs → Mixed
    #[test]
    fn test_6b_link_differing_hrefs_mixed() {
        let (mut state, _root, para) = simple_para_state(&['a', 'b']);
        let (leaf_a, leaf_b) = {
            let view = state.view();
            let p = view.node(para).unwrap();
            let mut it = p.children().filter_map(|c| {
                if let editor_model::ChildView::Leaf(l) = c {
                    Some(l.dot())
                } else {
                    None
                }
            });
            (it.next().unwrap(), it.next().unwrap())
        };
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::Link {
                    href: "https://a.example".to_string(),
                },
            }))
            .unwrap();
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_b,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_b,
                    bias: Bias::After,
                },
                modifier: Modifier::Link {
                    href: "https://b.example".to_string(),
                },
            }))
            .unwrap();
        let s = sel((para, 0), (para, 2));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.link, Tri::Mixed);
    }

    // §4.7: block-context modifier — selection inside one paragraph aggregates Root-context BlockGap
    #[test]
    fn test_7_block_context_modifier_root() {
        let (mut state, root, para) = simple_para_state(&['x']);
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: root,
                modifier: Modifier::BlockGap { value: 8 },
            }))
            .unwrap();
        let s = sel((para, 0), (para, 1));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.block_gap,
            Tri::Uniform {
                value: editor_model::BlockGapValue { value: 8 }
            }
        );
    }

    // §4.8: effective_bold — all-bold → Uniform; FontWeight 700 counts
    #[test]
    fn test_8_effective_bold_font_weight_700() {
        let (mut state, _root, para) = simple_para_state(&['a']);
        let leaf_a = {
            let view = state.view();
            let p = view.node(para).unwrap();
            p.children()
                .next()
                .and_then(|c| {
                    if let editor_model::ChildView::Leaf(l) = c {
                        Some(l.dot())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::FontWeight { value: 700 },
            }))
            .unwrap();
        let s = sel((para, 0), (para, 1));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.effective_bold,
            Tri::Uniform { value: () },
            "FontWeight 700 counts as bold for effective_bold"
        );
    }

    #[test]
    fn effective_bold_counts_inherited_bold() {
        let (mut state, _root, para) = simple_para_state(&['a']);
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::Bold,
            }))
            .unwrap();

        let s = sel((para, 0), (para, 1));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.bold, Tri::Uniform { value: () });
        assert_eq!(ms.effective_bold, Tri::Uniform { value: () });
    }

    // §4.8b: some-bold → Mixed effective_bold
    #[test]
    fn test_8b_effective_bold_mixed() {
        let (mut state, _root, para) = simple_para_state(&['a', 'b']);
        let leaf_a = {
            let view = state.view();
            let p = view.node(para).unwrap();
            p.children()
                .next()
                .and_then(|c| {
                    if let editor_model::ChildView::Leaf(l) = c {
                        Some(l.dot())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();
        let s = sel((para, 0), (para, 2));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(ms.effective_bold, Tri::Mixed);
    }

    // §4.9: marker modifiers seed an empty paragraph caret but do not restyle
    // existing text in a non-empty paragraph.
    #[test]
    fn test_9_marker_bold_applies_to_empty_caret_not_existing_range() {
        use editor_crdt::LwwRegOp;
        use editor_model::{EditOp, Marker, NodeLwwOp};

        // Part A: empty paragraph with marker — collapsed caret picks it up
        let (mut state_empty, _root_e, para_e) = simple_para_state(&[]);
        state_empty
            .apply(EditOp::NodeMarker(NodeLwwOp {
                target: para_e,
                op: LwwRegOp::Set {
                    value: Some(Marker {
                        modifiers: vec![Modifier::Bold],
                        style: None,
                    }),
                },
            }))
            .unwrap();
        let s_collapsed_empty = collapsed(para_e, 0);
        let ms_empty = resolve_modifier_state(&state_empty, &s_collapsed_empty, &[]).unwrap();
        assert_eq!(
            ms_empty.effective_bold,
            Tri::Uniform { value: () },
            "collapsed caret in empty paragraph with marker Bold → effective_bold Uniform"
        );

        // Part B: paragraph with 'a' + marker — marker is not part of the leaf's rendered style.
        let (mut state, _root, para) = simple_para_state(&['a']);
        state
            .apply(EditOp::NodeMarker(NodeLwwOp {
                target: para,
                op: LwwRegOp::Set {
                    value: Some(Marker {
                        modifiers: vec![Modifier::Bold],
                        style: None,
                    }),
                },
            }))
            .unwrap();
        let s_range = sel((para, 0), (para, 1));
        let ms_range = resolve_modifier_state(&state, &s_range, &[]).unwrap();
        assert_eq!(
            ms_range.effective_bold,
            Tri::Absent,
            "range over existing text is not bolded by the paragraph marker"
        );
    }

    // §4.10: over-collection guard — selection inside p1, image is NOT collected
    #[test]
    fn test_10_over_collection_guard() {
        use editor_model::AtomLeaf;

        let mut graph = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let p1 = graph
            .add_mut(seq_block(0, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        graph.add_mut(seq_char(1, 'a')).unwrap();
        graph.add_mut(seq_char(2, 'b')).unwrap();
        graph.add_mut(seq_char(3, 'c')).unwrap();
        let img_node = match NodeType::Image.into_node() {
            editor_model::Node::Image(n) => n,
            _ => unreachable!(),
        };
        let image_dot = graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root],
                },
            }))
            .unwrap()
            .id;
        let p2 = graph
            .add_mut(seq_block(5, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        graph.add_mut(seq_char(6, 'x')).unwrap();
        let mut state = ProjectedState::from_graph(graph).unwrap();

        // Set alignment on image — if image is over-collected, alignment would appear
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: image_dot,
                modifier: Modifier::Alignment {
                    value: editor_model::Alignment::Center,
                },
            }))
            .unwrap();

        // Selection wholly inside p1 — image must NOT pollute alignment
        let s = sel((p1, 0), (p1, 3));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        // alignment should be Absent (image not collected)
        assert_eq!(
            ms.alignment,
            Tri::Absent,
            "image alignment must not appear in a selection inside p1 only"
        );
        let _ = (p2, image_dot);
    }

    // §4.11: block-atom alignment — selection across p1, image[align], p2
    #[test]
    fn test_11_block_atom_alignment_ungated() {
        use editor_model::AtomLeaf;

        let mut graph = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let p1 = graph
            .add_mut(seq_block(0, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        graph.add_mut(seq_char(1, 'a')).unwrap();
        let img_node = match NodeType::Image.into_node() {
            editor_model::Node::Image(n) => n,
            _ => unreachable!(),
        };
        let image_dot = graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root],
                },
            }))
            .unwrap()
            .id;
        let p2 = graph
            .add_mut(seq_block(3, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        graph.add_mut(seq_char(4, 'b')).unwrap();
        let mut state = ProjectedState::from_graph(graph).unwrap();

        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: image_dot,
                modifier: Modifier::Alignment {
                    value: editor_model::Alignment::Center,
                },
            }))
            .unwrap();

        let s = sel((p1, 0), (p2, 1));
        let ms = resolve_modifier_state(&state, &s, &[]).unwrap();
        assert_eq!(
            ms.alignment,
            Tri::Mixed,
            "selection spanning p1(no alignment) + image(Center) + p2(no alignment) → Mixed"
        );
    }

    // §4.12: collapsed vs range parity — same modifiers at same spot
    #[test]
    fn test_12_collapsed_vs_range_parity() {
        let (mut state, _root, para) = simple_para_state(&['a']);
        let leaf_a = {
            let view = state.view();
            let p = view.node(para).unwrap();
            p.children()
                .next()
                .and_then(|c| {
                    if let editor_model::ChildView::Leaf(l) = c {
                        Some(l.dot())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf_a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf_a,
                    bias: Bias::After,
                },
                modifier: Modifier::Italic,
            }))
            .unwrap();

        let s_collapsed = collapsed(para, 1);
        let s_range = sel((para, 0), (para, 1));
        let ms_c = resolve_modifier_state(&state, &s_collapsed, &[]).unwrap();
        let ms_r = resolve_modifier_state(&state, &s_range, &[]).unwrap();
        assert_eq!(
            ms_c.italic, ms_r.italic,
            "collapsed and range agree on italic at same spot"
        );
    }

    // §4.13: proptest — never panics; range is a function of covered leaves
    proptest::proptest! {
        #[test]
        fn test_13_proptest_never_panics(
            a_off in 0usize..=2,
            h_off in 0usize..=2,
        ) {
            let (state, _root, para) = simple_para_state(&['a', 'b']);
            let s = Selection::new(
                Position::new(para, a_off.min(2)),
                Position::new(para, h_off.min(2)),
            );
            // must not panic
            let _ = resolve_modifier_state(&state, &s, &[]);
        }
    }

    // Reference: the per-leaf implementation the run-group aggregation replaced.
    // Kept verbatim so the equivalence proptest below pins the rewrite.
    fn reference_in_range(
        rs: &crate::selection::ResolvedSelection,
        leaf_map: &hashbrown::HashMap<Dot, crate::projected_state::LeafTruth>,
    ) -> editor_model::ModifierState {
        use std::collections::BTreeMap;

        use editor_model::{LeafView, Modifier, ModifierState, NodeType, NodeView, Schema};
        use strum::IntoEnumIterator;

        use crate::modifier_state::map_is_bold;
        use crate::selection::ResolvedSelection;
        use crate::traversal;

        enum RangeNode<'a> {
            Block(NodeView<'a>),
            Leaf(LeafView<'a>),
        }

        impl<'a> RangeNode<'a> {
            fn type_path(&self) -> Vec<NodeType> {
                match self {
                    RangeNode::Block(b) => {
                        let mut p: Vec<_> = b.ancestors().map(|n| n.node_type()).collect();
                        p.reverse();
                        p
                    }
                    RangeNode::Leaf(l) => {
                        let host = l.parent().expect("a collected leaf has a live host block");
                        let mut p: Vec<_> = host.ancestors().map(|n| n.node_type()).collect();
                        p.reverse();
                        p.push(l.node_type());
                        p
                    }
                }
            }
        }

        fn range_nodes<'a>(rs: &ResolvedSelection<'a>) -> Vec<RangeNode<'a>> {
            let blocks = traversal::blocks_in_range(rs);
            let mut out: Vec<RangeNode> = blocks.iter().copied().map(RangeNode::Block).collect();
            for b in &blocks {
                out.extend(
                    traversal::leaves_in_block_range(rs, b)
                        .into_iter()
                        .map(|(_, l)| RangeNode::Leaf(l)),
                );
            }
            out
        }

        struct RangePathTable {
            node_path: Vec<usize>,
            entries: Vec<(NodeType, Vec<NodeType>)>,
        }

        fn build_range_path_table(nodes: &[RangeNode]) -> RangePathTable {
            let mut entries: Vec<(NodeType, Vec<NodeType>)> = Vec::new();
            let mut index: std::collections::HashMap<(editor_crdt::Dot, NodeType), usize> =
                std::collections::HashMap::new();
            let mut node_path = Vec::with_capacity(nodes.len());
            for n in nodes {
                let key = match n {
                    RangeNode::Block(b) => (b.id(), b.node_type()),
                    RangeNode::Leaf(l) => (
                        l.parent()
                            .expect("a collected leaf has a live host block")
                            .id(),
                        l.node_type(),
                    ),
                };
                let idx = *index.entry(key).or_insert_with(|| {
                    entries.push((key.1, n.type_path()));
                    entries.len() - 1
                });
                node_path.push(idx);
            }
            RangePathTable { node_path, entries }
        }

        fn applicability(table: &RangePathTable, ty: ModifierType) -> Vec<bool> {
            let target = &Schema::modifier_spec(ty).target;
            let targets = target.rightmost_node_types();
            table
                .entries
                .iter()
                .map(|(nt, path)| targets.contains(nt) && target.matches(path))
                .collect()
        }

        struct Explicit<'a> {
            value: &'a Modifier,
            conflicting: bool,
            count: usize,
        }

        let mut out = ModifierState::default();
        let nodes = range_nodes(rs);
        let table = build_range_path_table(&nodes);

        // Effective map per node: blocks from `block_effective` (authoritative),
        // leaves from the log-derived truth — the reference must not read the old
        // per-leaf maps, which span ops no longer maintain.
        let effectives: Vec<&BTreeMap<ModifierType, Modifier>> = nodes
            .iter()
            .map(|n| match n {
                RangeNode::Block(b) => b.effective(),
                RangeNode::Leaf(l) => {
                    &*leaf_map
                        .get(&l.dot())
                        .expect("covered leaf in log-derived map")
                        .eff
                }
            })
            .collect();

        let apps: BTreeMap<ModifierType, Vec<bool>> = ModifierType::iter()
            .map(|ty| (ty, applicability(&table, ty)))
            .collect();

        let mut path_count = vec![0usize; table.entries.len()];
        for &pi in &table.node_path {
            path_count[pi] += 1;
        }

        let bold_applicable = &apps[&ModifierType::Bold];
        let mut explicit: BTreeMap<ModifierType, Explicit> = BTreeMap::new();
        let (mut bold_any_applicable, mut bold_all, mut bold_any) = (false, true, false);
        for (idx, (n, &pi)) in nodes.iter().zip(&table.node_path).enumerate() {
            let eff = effectives[idx];
            for (ty, m) in eff {
                if !apps.get(ty).is_some_and(|a| a[pi]) {
                    continue;
                }
                explicit
                    .entry(*ty)
                    .and_modify(|e| {
                        e.count += 1;
                        if e.value != m {
                            e.conflicting = true;
                        }
                    })
                    .or_insert(Explicit {
                        value: m,
                        conflicting: false,
                        count: 1,
                    });
            }
            if bold_applicable[pi] {
                bold_any_applicable = true;
                let bold = match n {
                    RangeNode::Leaf(_) => map_is_bold(eff),
                    RangeNode::Block(_) => false,
                };
                if bold {
                    bold_any = true;
                } else {
                    bold_all = false;
                }
            }
        }

        for ty in ModifierType::iter() {
            let applicable = &apps[&ty];
            let applicable_count: usize = path_count
                .iter()
                .zip(applicable)
                .filter(|(_, a)| **a)
                .map(|(c, _)| c)
                .sum();
            if applicable_count == 0 {
                continue;
            }
            let sparse_absence_is_neutral = matches!(ty, ModifierType::Link);
            let ty_default = editor_model::text_style_default_modifier(ty);
            let ex = explicit.get(&ty);
            if ex.is_some_and(|e| e.conflicting) {
                out.set_mixed(ty);
                continue;
            }
            let defaulted = applicable_count - ex.map_or(0, |e| e.count);
            match (ex.map(|e| e.value), ty_default) {
                (Some(e), _) if defaulted == 0 => out.set_uniform(e),
                (Some(e), Some(d)) => {
                    if *e == d {
                        out.set_uniform(e);
                    } else {
                        out.set_mixed(ty);
                    }
                }
                (Some(e), None) => {
                    if sparse_absence_is_neutral {
                        out.set_uniform(e);
                    } else {
                        out.set_mixed(ty);
                    }
                }
                (None, Some(d)) => out.set_uniform(&d),
                (None, None) => {}
            }
        }

        out.effective_bold = if !bold_any_applicable {
            Tri::Absent
        } else if bold_all {
            Tri::Uniform { value: () }
        } else if bold_any {
            Tri::Mixed
        } else {
            Tri::Absent
        };
        out
    }

    fn arb_ms_action() -> impl proptest::strategy::Strategy<Value = (u8, u8, u8, u8)> {
        use proptest::prelude::*;
        (0u8..12, any::<u8>(), any::<u8>(), any::<u8>())
    }

    // The run-group aggregation must agree with the per-leaf reference on
    // arbitrary docs (multiple paragraphs, atoms, block atoms, styles, spans of
    // every family, block modifiers) and arbitrary selections (char-level,
    // block-level, cross-paragraph, whole-doc).
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn in_range_matches_per_leaf_reference(
            actions in proptest::collection::vec(arb_ms_action(), 0..20),
            sel_pick in (proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>()),
        ) {
            use editor_model::{AtomLeaf, EditOp, SeqItem, SpanOp, Anchor, Bias};
            use editor_crdt::{ListOp, LwwRegOp, OrMapOp, OrSetOp};

            let mut state = crate::projected_state::ProjectedState::empty();
            state.apply(EditOp::Style(editor_model::StyleRegOp {
                style_id: "s".to_string(),
                op: editor_model::StyleOp::Presence(OrMapOp::Set { key: "s".to_string(), value: () }),
            })).unwrap();
            state.apply(EditOp::Style(editor_model::StyleRegOp {
                style_id: "s".to_string(),
                op: editor_model::StyleOp::Modifiers(OrSetOp::Add { elem: Modifier::FontSize { value: 1600 } }),
            })).unwrap();
            let mut live: Vec<Dot> = Vec::new();
            let mut count = 1usize;
            for i in 0..24usize {
                let d = state.apply(EditOp::Seq(ListOp::Ins {
                    pos: count,
                    item: SeqItem::Char(char::from(b'a' + (i % 26) as u8)),
                })).unwrap().id;
                live.push(d);
                count += 1;
            }
            for (kind, a, b, bias) in actions {
                let pick = |i: u8, v: &[Dot]| v[(i as usize) % v.len()];
                let bias_s = if bias & 1 == 0 { Bias::Before } else { Bias::After };
                let bias_e = if bias & 2 == 0 { Bias::Before } else { Bias::After };
                let m = match bias % 6 {
                    0 => Modifier::Bold,
                    1 => Modifier::Italic,
                    2 => Modifier::FontWeight { value: 700 },
                    3 => Modifier::FontSize { value: 1400 },
                    4 => Modifier::Link { href: "https://a.example".to_string() },
                    _ => Modifier::Link { href: "https://b.example".to_string() },
                };
                match kind {
                    0..=1 => {
                        let pos = 1 + (a as usize) % count;
                        let d = state.apply(EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::Char('z'),
                        })).unwrap().id;
                        live.push(d);
                        count += 1;
                    }
                    2 => {
                        let pos = 1 + (a as usize) % count;
                        state.apply(EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::Atom(AtomLeaf::HardBreak),
                        })).unwrap();
                        count += 1;
                    }
                    3 => {
                        let pos = 1 + (a as usize) % count;
                        state.apply(EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::Block { node_type: NodeType::Paragraph, parents: vec![Dot::ROOT] },
                        })).unwrap();
                        count += 1;
                    }
                    4 => {
                        let pos = 1 + (a as usize) % count;
                        let img = match NodeType::Image.into_node() {
                            editor_model::Node::Image(n) => n,
                            _ => unreachable!(),
                        };
                        state.apply(EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::BlockAtom { leaf: AtomLeaf::Image { node: img }, parents: vec![Dot::ROOT] },
                        })).unwrap();
                        count += 1;
                    }
                    5 if !live.is_empty() => {
                        state.apply(EditOp::NodeStyle(editor_model::NodeLwwOp {
                            target: pick(a, &live),
                            op: LwwRegOp::Set { value: Some("s".to_string()) },
                        })).unwrap();
                    }
                    6 => {
                        state.apply(EditOp::BlockModifier(editor_model::ModifierAttrOp::SetModifier {
                            target: Dot::ROOT,
                            modifier: match bias % 3 {
                                0 => Modifier::FontWeight { value: 700 },
                                1 => Modifier::FontSize { value: 1200 },
                                _ => Modifier::BlockGap { value: 8 },
                            },
                        })).unwrap();
                    }
                    7..=9 if !live.is_empty() => {
                        state.apply(EditOp::Span(SpanOp::AddSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier: m,
                        })).unwrap();
                    }
                    10 if !live.is_empty() => {
                        state.apply(EditOp::Span(SpanOp::RemoveSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier_type: m.as_type(),
                        })).unwrap();
                    }
                    11 if !live.is_empty() => {
                        state.apply(EditOp::Span(SpanOp::ClearSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier_type: m.as_type(),
                        })).unwrap();
                    }
                    _ => {}
                }
            }

            let view = state.view();
            let mut blocks: Vec<(Dot, usize)> = Vec::new();
            if let Some(root) = view.root() {
                blocks.push((root.id(), root.child_count()));
                for c in root.descendants() {
                    if let editor_model::ChildView::Block(bl) = c {
                        blocks.push((bl.id(), bl.child_count()));
                    }
                }
            }
            proptest::prop_assume!(!blocks.is_empty());
            let (s0, s1, s2, s3) = sel_pick;
            let (b1, c1) = blocks[(s0 as usize) % blocks.len()];
            let (b2, c2) = blocks[(s1 as usize) % blocks.len()];
            let sel = Selection::new(
                Position::new(b1, (s2 as usize) % (c1 + 1)),
                Position::new(b2, (s3 as usize) % (c2 + 1)),
            );
            if let Some(rs) = sel.resolve(&view)
                && !rs.is_collapsed()
            {
                let leaf_map = state.log_derived_leaf_map();
                let got = crate::modifier_state::resolve_modifier_state_in_range(&rs);
                let want = reference_in_range(&rs, &leaf_map);
                proptest::prop_assert_eq!(got, want);

                // leaf_groups_in_range must expand to exactly the per-leaf
                // scan: same leaves in the same order, group bounds anchored on
                // the right dots, and each group's effective agreeing with
                // every covered leaf's own map.
                let dots = crate::inline_leaf_dots_in_range(
                    &view,
                    &rs.from().position(),
                    &rs.to().position(),
                );
                let groups = crate::traversal::leaf_groups_in_range(&rs);
                let total: usize = groups.iter().map(|g| g.count).sum();
                proptest::prop_assert_eq!(total, dots.len());
                let mut i = 0;
                for g in &groups {
                    proptest::prop_assert_eq!(g.first, dots[i]);
                    proptest::prop_assert_eq!(g.last, dots[i + g.count - 1]);
                    for &dot in &dots[i..i + g.count] {
                        let truth = leaf_map.get(&dot).expect("covered leaf resolves");
                        proptest::prop_assert_eq!(&*truth.eff, g.effective);
                        proptest::prop_assert_eq!(truth.leaf_type, g.leaf_type);
                        proptest::prop_assert_eq!(&*truth.own, g.own);
                        proptest::prop_assert_eq!(truth.style.as_ref(), g.style);
                    }
                    i += g.count;
                }
            }
        }
    }
}
