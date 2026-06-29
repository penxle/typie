use std::collections::BTreeMap;

use editor_macros::ffi;
use editor_model::{DocView, Modifier, ModifierType};
use serde::{Deserialize, Serialize};

use crate::Position;
use crate::modifier_resolution::{CaretCtx, resolve_caret_modifiers};
use crate::pending_modifier::PendingModifier;
use crate::projected_state::ProjectedState;
use crate::selection::Selection;
use crate::stable_position::{StablePosition, StableResolveCtx};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StableSelection {
    anchor: StablePosition,
    head: StablePosition,
}

impl StableSelection {
    pub fn capture(sel: &Selection, view: &DocView) -> StableSelection {
        use crate::affinity::Affinity;
        // A non-collapsed range binds its boundaries exclusively so text typed
        // at either edge stays outside: the start edge binds left-exclusive
        // (Downstream → following child) and the end edge right-exclusive
        // (Upstream → preceding child). A collapsed caret keeps each endpoint's
        // own affinity so it never splits on a boundary insertion. The stored
        // affinity is always the endpoint's original (only the binding changes).
        let anchor_is_start = match sel.resolve(view) {
            Some(rs) if !rs.is_collapsed() => Some(rs.anchor() <= rs.head()),
            _ => None,
        };
        let (anchor_aff, head_aff) = match anchor_is_start {
            Some(true) => (Affinity::Downstream, Affinity::Upstream),
            Some(false) => (Affinity::Upstream, Affinity::Downstream),
            None => (sel.anchor.affinity, sel.head.affinity),
        };
        StableSelection {
            anchor: StablePosition::capture_with_bind_affinity(&sel.anchor, anchor_aff, view),
            head: StablePosition::capture_with_bind_affinity(&sel.head, head_aff, view),
        }
    }

    pub fn resolve(&self, ctx: &StableResolveCtx) -> Option<Selection> {
        let anchor = self.anchor.resolve(ctx)?;
        let head = self.head.resolve(ctx)?;
        Some(Selection { anchor, head })
    }
}

/// Effective inline modifiers a caret at `pos` would carry (no pending overrides).
pub fn resolve_effective_modifiers_at(
    state: &crate::state::State,
    pos: &Position,
) -> Vec<Modifier> {
    caret_modifiers(&state.projected, pos, &[])
        .into_values()
        .collect()
}

pub(crate) fn caret_modifiers(
    state: &ProjectedState,
    pos: &Position,
    pending: &[PendingModifier],
) -> BTreeMap<ModifierType, Modifier> {
    let view = state.view();
    let ctx = CaretCtx {
        view: &view,
        doc: state.projected(),
        block_modifiers: state.block_modifiers(),
    };
    resolve_caret_modifiers(pos, &ctx, pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        Anchor, Bias, DocLogs, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
        ProjectedDoc, SeqItem, SpanLog, StyleLog, project_document,
    };

    use crate::Position;
    use crate::projected_state::ProjectedState;

    fn block(node_type: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block { node_type, parents }
    }

    fn ins_only(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
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
        ev
    }

    fn doclogs(ev: &[InputEvent<SeqItem>]) -> DocLogs {
        DocLogs {
            seq: build_oplog(ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn para_with_chars(chars: &[char]) -> (ProjectedDoc, DocLogs, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(para, block(NodeType::Paragraph, vec![root]))];
        for (i, c) in chars.iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(*c)));
        }
        let ev = ins_only(&items);
        let logs = doclogs(&ev);
        let pd = project_document(&logs).unwrap();
        (pd, logs, root, para)
    }

    fn state_with_chars(chars: &[char]) -> (ProjectedState, Dot, Dot) {
        use editor_crdt::OpGraph;
        use editor_model::EditOp;
        let mut graph = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let para = graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: block(NodeType::Paragraph, vec![root]),
            }))
            .unwrap()
            .id;
        for (i, c) in chars.iter().enumerate() {
            graph
                .add_mut(EditOp::Seq(ListOp::Ins {
                    pos: 1 + i,
                    item: SeqItem::Char(*c),
                }))
                .unwrap();
        }
        let state = ProjectedState::from_graph(graph).unwrap();
        (state, root, para)
    }

    // Test 1: StableSelection round-trip (unchanged doc)
    #[test]
    fn test_1_round_trip_unchanged_doc() {
        let (pd, logs, _root, para) = para_with_chars(&['a', 'b', 'c']);
        let view = DocView::new(&pd);
        let anchor = Position::new(para, 0);
        let head = Position::new(para, 2);
        let sel = Selection::new(anchor, head);
        let ss = StableSelection::capture(&sel, &view);
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let resolved = ss.resolve(&ctx).unwrap();
        assert_eq!(resolved.anchor, anchor);
        assert_eq!(resolved.head, head);
    }

    // Test 2: StableSelection survives a delete
    #[test]
    fn test_2_survives_delete() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let pre_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        let pre_logs = doclogs(&ins_only(&pre_items));
        let pre_pd = project_document(&pre_logs).unwrap();
        let pre_view = DocView::new(&pre_pd);

        let anchor = Position::new(para, 0);
        let head = Position::new(para, 3);
        let sel = Selection::new(anchor, head);
        let ss = StableSelection::capture(&sel, &pre_view);

        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 5),
            parents: vec![Dot::new(1, 4)],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        let post_logs = doclogs(&post_ev);
        let post_pd = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post_pd);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);

        let resolved = ss.resolve(&ctx).unwrap();
        let host = post_view.node(resolved.anchor.node).unwrap();
        assert!(resolved.anchor.offset <= host.children().count());
        let host = post_view.node(resolved.head.node).unwrap();
        assert!(resolved.head.offset <= host.children().count());
    }

    // Test 4: resolve None on dead host chain (doc A capture / doc B resolve)
    #[test]
    fn test_4_unknown_dead_chain_returns_none() {
        // Doc A: actor 1, specific items
        let root_a = Dot::ROOT;
        let para_a = Dot::new(1, 1);
        let items_a = vec![
            (para_a, block(NodeType::Paragraph, vec![root_a])),
            (Dot::new(1, 2), SeqItem::Char('x')),
        ];
        let logs_a = doclogs(&ins_only(&items_a));
        let pd_a = project_document(&logs_a).unwrap();
        let view_a = DocView::new(&pd_a);

        let sel_a = Selection::new(Position::new(para_a, 0), Position::new(para_a, 1));
        let ss = StableSelection::capture(&sel_a, &view_a);

        // Doc B: actor 2, none of A's CONTENT blocks survive. With the canonical
        // implicit root (Dot::ROOT shared by every document), the only surviving
        // chain element is the root, so resolve falls back to the root rather than
        // returning None.
        let root_b = Dot::ROOT;
        let para_b = Dot::new(2, 1);
        let items_b = vec![
            (para_b, block(NodeType::Paragraph, vec![root_b])),
            (Dot::new(2, 2), SeqItem::Char('y')),
        ];
        let logs_b = doclogs(&ins_only(&items_b));
        let pd_b = project_document(&logs_b).unwrap();
        let view_b = DocView::new(&pd_b);
        let ctx_b = StableResolveCtx::new(&view_b, &logs_b.seq);

        let resolved = ss
            .resolve(&ctx_b)
            .expect("falls back to the canonical root");
        assert_eq!(resolved.anchor.node, Dot::ROOT);
        assert_eq!(resolved.head.node, Dot::ROOT);
    }

    // Test 5: collapsed caret modifiers — bolded char has Bold
    #[test]
    fn test_5_collapsed_caret_modifiers_bold() {
        use editor_model::{EditOp, SpanOp};
        let (mut state, _root, para) = state_with_chars(&['a', 'b']);
        let leaf_a = state
            .view()
            .node(para)
            .unwrap()
            .children()
            .next()
            .and_then(|c| {
                if let editor_model::ChildView::Leaf(l) = c {
                    Some(l.dot())
                } else {
                    None
                }
            })
            .unwrap();
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

        let pos = Position::new(para, 1);
        let result = caret_modifiers(&state, &pos, &[]);
        assert_eq!(result.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    // Test 5b: empty paragraph with marker surfaces modifiers
    #[test]
    fn test_5b_empty_para_marker_modifiers() {
        use editor_crdt::{LwwRegOp, OpGraph};
        use editor_model::{EditOp, Marker, NodeLwwOp};
        let mut graph = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let para = graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: block(NodeType::Paragraph, vec![root]),
            }))
            .unwrap()
            .id;
        let mut state = ProjectedState::from_graph(graph).unwrap();
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

        let pos = Position::new(para, 0);
        let result = caret_modifiers(&state, &pos, &[]);
        assert_eq!(result.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    // Test 5c: pending overlay applies
    #[test]
    fn test_5c_pending_overlay() {
        let (state, _root, para) = state_with_chars(&[]);
        let pos = Position::new(para, 0);
        let result = caret_modifiers(
            &state,
            &pos,
            &[PendingModifier::Set {
                modifier: Modifier::Italic,
            }],
        );
        assert_eq!(result.get(&ModifierType::Italic), Some(&Modifier::Italic));
    }

    // Test 6: direction preserved — head-before-anchor stays that way after resolve
    #[test]
    fn test_6_direction_preserved() {
        let (pd, logs, _root, para) = para_with_chars(&['a', 'b', 'c']);
        let view = DocView::new(&pd);
        // anchor at offset 3 (end), head at offset 0 (start) — head before anchor
        let anchor = Position::new(para, 3);
        let head = Position::new(para, 0);
        let sel = Selection::new(anchor, head);
        let ss = StableSelection::capture(&sel, &view);
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let resolved = ss.resolve(&ctx).unwrap();
        assert_eq!(resolved.anchor, anchor);
        assert_eq!(resolved.head, head);
    }

    // Test 3: StableSelection survives Undel — Del then Undel restores the original selection
    #[test]
    fn test_3_survives_undel() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let b = Dot::new(1, 3);
        let base_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        let pre_logs = doclogs(&ins_only(&base_items));
        let pre_pd = project_document(&pre_logs).unwrap();
        let pre_view = DocView::new(&pre_pd);

        let anchor = Position::new(para, 1);
        let head = Position::new(para, 2);
        let sel = Selection::new(anchor, head);
        let ss = StableSelection::capture(&sel, &pre_view);

        let del_op = Dot::new(1, 5);
        let mut ev = ins_only(&base_items);
        ev.push(InputEvent {
            id: del_op,
            parents: vec![Dot::new(1, 4)],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        ev.push(InputEvent {
            id: Dot::new(1, 6),
            parents: vec![del_op],
            op: ListOp::Undel { del: del_op },
        });
        let post_logs = doclogs(&ev);
        let post_pd = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post_pd);
        assert!(post_view.leaf(b).is_some(), "Undel must restore 'b' live");
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);

        let resolved = ss.resolve(&ctx).unwrap();
        assert_eq!(resolved.anchor, anchor);
        assert_eq!(resolved.head, head);
    }

    // Test 7: proptest — random small docs + random selections + single delete: resolve never panics,
    // resolved endpoints in-range, Undel restores exactly.
    fn arb_para_chars() -> impl proptest::strategy::Strategy<Value = Vec<char>> {
        use proptest::prelude::*;
        proptest::collection::vec(prop::sample::select(vec!['a', 'b', 'c', 'd']), 1..5)
    }

    proptest::proptest! {
        #[test]
        fn test_7_proptest_resolve_survives_delete(chars in arb_para_chars(), del_idx in 0usize..4) {
            let root = Dot::ROOT;
            let para = Dot::new(1, 1);
            let mut items = vec![
                (para, block(NodeType::Paragraph, vec![root])),
            ];
            for (i, c) in chars.iter().enumerate() {
                items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(*c)));
            }
            let pre_logs = doclogs(&ins_only(&items));
            let pre_pd = project_document(&pre_logs).unwrap();
            let pre_view = DocView::new(&pre_pd);

            let n = chars.len();
            let a_off = 0;
            let h_off = n;
            let anchor = Position::new(para, a_off);
            let head = Position::new(para, h_off);
            let sel = Selection::new(anchor, head);
            let ss = StableSelection::capture(&sel, &pre_view);

            let del_pos = 1 + (del_idx % n);
            let del_op_dot = Dot::new(1, 100);
            let last = items.last().unwrap().0;
            let mut post_ev = ins_only(&items);
            post_ev.push(InputEvent {
                id: del_op_dot,
                parents: vec![last],
                op: ListOp::Del { pos: del_pos, len: 1 },
            });
            let post_logs = doclogs(&post_ev);
            let post_pd = project_document(&post_logs).unwrap();
            let post_view = DocView::new(&post_pd);
            let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);

            let resolved = ss.resolve(&ctx).expect("resolve must return Some after delete");
            let host_a = post_view.node(resolved.anchor.node).expect("anchor host exists");
            proptest::prop_assert!(resolved.anchor.offset <= host_a.children().count());
            let host_h = post_view.node(resolved.head.node).expect("head host exists");
            proptest::prop_assert!(resolved.head.offset <= host_h.children().count());

            // Undel restores exactly
            let mut undel_ev = post_ev;
            undel_ev.push(InputEvent {
                id: Dot::new(1, 101),
                parents: vec![del_op_dot],
                op: ListOp::Undel { del: del_op_dot },
            });
            let undel_logs = doclogs(&undel_ev);
            let undel_pd = project_document(&undel_logs).unwrap();
            let undel_view = DocView::new(&undel_pd);
            let undel_ctx = StableResolveCtx::new(&undel_view, &undel_logs.seq);
            let restored = ss.resolve(&undel_ctx).expect("resolve after undel");
            proptest::prop_assert_eq!(restored.anchor, anchor);
            proptest::prop_assert_eq!(restored.head, head);
        }
    }
}
