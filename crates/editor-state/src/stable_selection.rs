use std::cmp::Ordering;
use std::sync::Arc;

use editor_macros::ffi;
use editor_model::DocView;
use serde::{Deserialize, Serialize};

use crate::bind::Bind;
use crate::selection::Selection;
use crate::stable_position::{StablePosition, StableResolveCtx};
use crate::state::State;

/// The persisted wire version of a `StableSelection`. Bumped when the anchor
/// encoding changes; a stored envelope carries it so future revisions can tell
/// formats apart. There is no legacy-read path — every consumer is force-updated
/// and server-stored v1 is cleared by a one-shot migration — so the runtime only
/// ever writes and reads the current version.
pub const STABLE_SELECTION_WIRE_VERSION: u32 = 2;

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StableSelection {
    pub version: u32,
    pub anchor: StablePosition,
    pub head: StablePosition,
}

impl StableSelection {
    pub fn capture(sel: &Selection, view: &DocView) -> StableSelection {
        // A non-collapsed range binds its boundaries exclusively so text typed
        // at either edge stays outside. A collapsed caret keeps each endpoint's
        // own affinity so it never splits on a boundary insertion. A raw-equal
        // pair is collapsed by construction — resolving it to learn that costs
        // an `O(parent fan-out)` ancestor index scan on the per-keystroke
        // undo-capture path.
        let anchor_is_start = if sel.anchor == sel.head {
            None
        } else {
            match sel.resolve(view) {
                Some(rs) if !rs.is_collapsed() => Some(rs.anchor() <= rs.head()),
                _ => None,
            }
        };
        let (anchor, head) = match anchor_is_start {
            Some(true) => (
                StablePosition::capture_range_start(&sel.anchor, view),
                StablePosition::capture_range_end(&sel.head, view),
            ),
            Some(false) => (
                StablePosition::capture_range_end(&sel.anchor, view),
                StablePosition::capture_range_start(&sel.head, view),
            ),
            None => (
                StablePosition::capture(&sel.anchor, view),
                StablePosition::capture(&sel.head, view),
            ),
        };
        StableSelection {
            version: STABLE_SELECTION_WIRE_VERSION,
            anchor,
            head,
        }
    }

    pub fn resolve(&self, ctx: &StableResolveCtx) -> Option<Selection> {
        let primary = Selection {
            anchor: self.anchor.resolve(ctx)?,
            head: self.head.resolve(ctx)?,
        };
        if self.anchor == self.head {
            return Some(primary);
        }
        if self.anchor.has_live_inline_child(ctx) && self.head.has_live_inline_child(ctx) {
            return Some(primary);
        }

        if let Some(order) = self.anchor.cmp_inline_boundary(&self.head, ctx)
            && order != Ordering::Equal
        {
            return Some(self.resolve_inline_range(ctx, primary, order));
        }

        let Some(resolved) = primary.resolve(ctx.view()) else {
            return Some(primary);
        };
        Some(match resolved.anchor().cmp(resolved.head()) {
            Ordering::Less => Selection {
                anchor: self.anchor.range_start_position(ctx, primary.anchor),
                head: self.head.range_end_position(ctx, primary.head),
            },
            Ordering::Greater => Selection {
                anchor: self.anchor.range_end_position(ctx, primary.anchor),
                head: self.head.range_start_position(ctx, primary.head),
            },
            Ordering::Equal => primary,
        })
    }

    fn resolve_inline_range(
        &self,
        ctx: &StableResolveCtx,
        primary: Selection,
        order: Ordering,
    ) -> Selection {
        let (start, end, reversed) = match order {
            Ordering::Less => (
                self.anchor
                    .inline_range_boundary_position(ctx, primary.anchor, Bind::Left),
                self.head
                    .inline_range_boundary_position(ctx, primary.head, Bind::Right),
                false,
            ),
            Ordering::Greater => (
                self.head
                    .inline_range_boundary_position(ctx, primary.head, Bind::Left),
                self.anchor
                    .inline_range_boundary_position(ctx, primary.anchor, Bind::Right),
                true,
            ),
            Ordering::Equal => return primary,
        };
        let (Some(start), Some(end)) = (start, end) else {
            return primary;
        };

        let crosses = Selection::new(start, end)
            .resolve(ctx.view())
            .is_some_and(|resolved| resolved.anchor() > resolved.head());
        if crosses {
            return Selection::collapsed(end);
        }
        if reversed {
            Selection::new(end, start)
        } else {
            Selection::new(start, end)
        }
    }
}

/// Remaps a selection that resolves in `source` into `target` by stable identity.
///
/// `selection` must resolve in `source`; debug builds assert this precondition.
/// `None` means the captured selection could not resolve in `target`.
pub fn remap_selection(selection: Selection, source: &State, target: &State) -> Option<Selection> {
    debug_assert!(
        {
            let source_view = source.view();
            selection.resolve(&source_view).is_some()
        },
        "remap_selection source selection must resolve"
    );
    if Arc::ptr_eq(&source.projected, &target.projected) {
        return Some(selection);
    }
    let stable = StableSelection::capture(&selection, &source.view());
    let target_view = target.view();
    let ctx = StableResolveCtx::from_live(&target_view, target.projected.seq_checkout());
    stable.resolve(&ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Changeset, Dot, InputEvent, ListOp, OpGraph, build_oplog};
    use editor_model::{
        AliasLog, DocLogs, EditOp, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc, SeqItem,
        SpanLog, project_document,
    };

    use crate::Position;

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "remap_selection source selection must resolve")]
    fn remap_selection_requires_a_valid_source() {
        let source = State::empty();
        let target = source.clone();
        let selection = Selection::collapsed(Position::new(Dot::new(9, 9), 0));

        let _ = remap_selection(selection, &source, &target);
    }

    fn block(node_type: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block {
            node_type,
            parents,
            attrs: vec![],
        }
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
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
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

    #[derive(Clone, Copy, Debug)]
    enum DeletedInlineBoundaryCase {
        Start,
        End,
        Full,
    }

    fn inline_state(text: &str) -> (crate::ProjectedState, Dot) {
        let mut state = crate::ProjectedState::empty();
        let paragraph = state.seq().entries[0].dot;
        for (offset, ch) in text.chars().enumerate() {
            state
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: 1 + offset,
                    item: SeqItem::Char(ch),
                }))
                .unwrap();
        }
        (state, paragraph)
    }

    fn directed_inline_selection(
        paragraph: Dot,
        start: usize,
        end: usize,
        reversed: bool,
    ) -> Selection {
        let start = Position::new(paragraph, start);
        let end = Position::new(paragraph, end);
        if reversed {
            Selection::new(end, start)
        } else {
            Selection::new(start, end)
        }
    }

    fn assert_owned_live_resolution(
        stable: &StableSelection,
        state: &crate::ProjectedState,
        expected: Selection,
    ) {
        let view = state.view();
        let owned = StableResolveCtx::new(&view, state.seq());
        let live = StableResolveCtx::from_live(&view, state.seq_checkout());
        assert_eq!(stable.resolve(&owned), Some(expected));
        assert_eq!(stable.resolve(&live), Some(expected));
    }

    fn projected_client(actor: u64, base: &[Changeset<EditOp>]) -> crate::ProjectedState {
        let mut graph = OpGraph::with_actor(actor);
        for changeset in base {
            graph
                .receive_changeset_mut(changeset.clone())
                .expect("shared base applies");
        }
        crate::ProjectedState::from_graph(graph).expect("shared base projects")
    }

    #[test]
    fn deleted_inline_boundary_resolution_matches_owned_and_live() {
        for case in [
            DeletedInlineBoundaryCase::Start,
            DeletedInlineBoundaryCase::End,
            DeletedInlineBoundaryCase::Full,
        ] {
            for reversed in [false, true] {
                let (mut state, paragraph) = inline_state("abcdef");
                let stable = StableSelection::capture(
                    &directed_inline_selection(paragraph, 1, 4, reversed),
                    &state.view(),
                );

                let (expected_start, expected_end) = match case {
                    DeletedInlineBoundaryCase::Start => {
                        state
                            .apply(EditOp::Seq(ListOp::Ins {
                                pos: 3,
                                item: SeqItem::Char('Y'),
                            }))
                            .unwrap();
                        state
                            .apply(EditOp::Seq(ListOp::Del { pos: 2, len: 1 }))
                            .unwrap();
                        state
                            .apply(EditOp::Seq(ListOp::Ins {
                                pos: 2,
                                item: SeqItem::Char('X'),
                            }))
                            .unwrap();
                        (2, 5)
                    }
                    DeletedInlineBoundaryCase::End => {
                        state
                            .apply(EditOp::Seq(ListOp::Ins {
                                pos: 4,
                                item: SeqItem::Char('Y'),
                            }))
                            .unwrap();
                        state
                            .apply(EditOp::Seq(ListOp::Del { pos: 5, len: 2 }))
                            .unwrap();
                        state
                            .apply(EditOp::Seq(ListOp::Ins {
                                pos: 5,
                                item: SeqItem::Char('X'),
                            }))
                            .unwrap();
                        (1, 4)
                    }
                    DeletedInlineBoundaryCase::Full => {
                        state
                            .apply(EditOp::Seq(ListOp::Del { pos: 2, len: 3 }))
                            .unwrap();
                        state
                            .apply(EditOp::Seq(ListOp::Ins {
                                pos: 2,
                                item: SeqItem::Char('X'),
                            }))
                            .unwrap();
                        (1, 1)
                    }
                };
                let expected = directed_inline_selection(
                    paragraph,
                    expected_start,
                    expected_end,
                    reversed && expected_start != expected_end,
                );
                assert_owned_live_resolution(&stable, &state, expected);
            }
        }
    }

    #[test]
    fn collapsed_selection_keeps_primary_resolution_after_its_child_is_deleted() {
        let (mut state, paragraph) = inline_state("abcdef");
        let stable = StableSelection::capture(
            &Selection::collapsed(Position::new(paragraph, 2)),
            &state.view(),
        );
        state
            .apply(EditOp::Seq(ListOp::Del { pos: 2, len: 2 }))
            .unwrap();
        state
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Char('X'),
            }))
            .unwrap();

        let view = state.view();
        for ctx in [
            StableResolveCtx::new(&view, state.seq()),
            StableResolveCtx::from_live(&view, state.seq_checkout()),
        ] {
            let primary = stable.anchor.resolve(&ctx).unwrap();
            assert_eq!(stable.resolve(&ctx), Some(Selection::collapsed(primary)));
        }
    }

    #[test]
    fn missing_canonical_inline_end_candidate_preserves_primary_selection() {
        let (mut state, paragraph) = inline_state("abcdef");
        let stable = StableSelection::capture(
            &directed_inline_selection(paragraph, 0, 3, false),
            &state.view(),
        );
        state
            .apply(EditOp::Seq(ListOp::Del { pos: 1, len: 3 }))
            .unwrap();
        state
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            }))
            .unwrap();

        let view = state.view();
        for ctx in [
            StableResolveCtx::new(&view, state.seq()),
            StableResolveCtx::from_live(&view, state.seq_checkout()),
        ] {
            let primary = Selection {
                anchor: stable.anchor.resolve(&ctx).unwrap(),
                head: stable.head.resolve(&ctx).unwrap(),
            };
            let end = stable
                .head
                .inline_range_boundary_position(&ctx, primary.head, Bind::Right);
            assert_eq!(end, None);
            assert_eq!(stable.resolve(&ctx), Some(primary));
        }
    }

    #[test]
    fn deletion_gap_excludes_an_insertion_the_delete_did_not_observe() {
        let mut base_graph = OpGraph::with_actor(0);
        base_graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: block(NodeType::Paragraph, vec![Dot::ROOT]),
            }))
            .unwrap();
        for (offset, ch) in "abcdef".chars().enumerate() {
            base_graph
                .add_mut(EditOp::Seq(ListOp::Ins {
                    pos: 1 + offset,
                    item: SeqItem::Char(ch),
                }))
                .unwrap();
        }
        base_graph.commit_mut();
        let base = base_graph.changesets_as_vec();
        let paragraph = base[0].ops[0].id;

        for reversed in [false, true] {
            let base_state = projected_client(3, &base);
            let stable = StableSelection::capture(
                &directed_inline_selection(paragraph, 1, 4, reversed),
                &base_state.view(),
            );

            let mut inserter = projected_client(1, &base);
            inserter
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: 4,
                    item: SeqItem::Char('Z'),
                }))
                .unwrap();
            inserter.commit();

            let mut deleter = projected_client(2, &base);
            deleter
                .apply(EditOp::Seq(ListOp::Del { pos: 4, len: 2 }))
                .unwrap();
            deleter.commit();

            let mut merged = projected_client(4, &base);
            for changeset in inserter
                .graph()
                .changesets_as_vec()
                .into_iter()
                .chain(deleter.graph().changesets_as_vec())
                .filter(|changeset| changeset.ops[0].id.actor != 0)
            {
                merged = merged.receive_changeset(changeset).unwrap();
            }

            assert!(
                merged
                    .view()
                    .node(paragraph)
                    .unwrap()
                    .inline_text()
                    .contains('Z')
            );
            assert_owned_live_resolution(
                &stable,
                &merged,
                directed_inline_selection(paragraph, 1, 3, reversed),
            );
        }
    }
}
