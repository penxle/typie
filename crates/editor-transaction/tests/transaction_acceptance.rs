use editor_common::time::{Duration, Instant};
use editor_crdt::Dot;
use editor_macros::state;
use editor_model::{
    AtomLeaf, CalloutVariant, ChildView, EditOp, LayoutMode, Modifier, ModifierType, Node,
    NodeAttrOp, NodeType, NodeView, PlainCalloutNode, PlainNode, PlainParagraphNode, PlainRootNode,
    Subtree,
};
use editor_state::undo::{RecordMerge, TransientState, UndoEntry, UndoHistory};
use editor_state::{
    Affinity, Position, Selection, StablePosition, StableResolveCtx, StableSelection, State,
};
use editor_transaction::{Step, StepError, Transaction};
use proptest::prelude::*;

fn block_text(state: &State, elem: &Dot) -> String {
    state
        .view()
        .node(*elem)
        .map(|n| n.inline_text())
        .unwrap_or_default()
}

fn root_id(state: &State) -> Dot {
    state.view().root().unwrap().id()
}

fn root_blocks(state: &State) -> Vec<(NodeType, String)> {
    state
        .view()
        .root()
        .unwrap()
        .child_blocks()
        .map(|b| (b.node_type(), b.inline_text()))
        .collect()
}

fn root_child_labels(state: &State) -> Vec<String> {
    state
        .view()
        .root()
        .unwrap()
        .children()
        .map(|child| match child {
            ChildView::Block(block) => block.inline_text(),
            ChildView::Leaf(leaf) => format!("{:?}", leaf.node_type()),
        })
        .collect()
}

fn snapshot(state: &State) -> Vec<(usize, NodeType, String)> {
    fn walk(nv: &NodeView, depth: usize, out: &mut Vec<(usize, NodeType, String)>) {
        out.push((depth, nv.node_type(), nv.inline_text()));
        for b in nv.child_blocks() {
            walk(&b, depth + 1, out);
        }
    }
    let view = state.view();
    let mut out = Vec::new();
    if let Some(root) = view.root() {
        walk(&root, 0, &mut out);
    }
    out
}

mod proptests {
    use super::*;

    proptest! {
        #[test]
        fn step_fail_rollback_preserves_prior_mutations(
            text_a in "[a-z]{1,5}",
            text_b in "[a-z]{1,5}",
        ) {
            let (state, p1) = state! {
                doc { root { p1: paragraph { text("") } } }
                selection: (p1, 0)
            };

            let mut tr = Transaction::new(&state);
            tr.insert_text(p1, 0, &text_a).unwrap();
            tr.insert_text(p1, 0, &text_b).unwrap();
            let after = block_text(tr.state(), &p1);

            let invalid_offset = after.chars().count() + 100;
            let result = tr.insert_text(p1, invalid_offset, "x");
            prop_assert!(result.is_err());

            prop_assert_eq!(block_text(tr.state(), &p1), after);
        }
    }

    proptest! {
        #[test]
        fn split_paragraph_preserves_chars(
            text in "[a-z]{1,15}",
            split_at in 0usize..15,
        ) {
            let chars: Vec<char> = text.chars().collect();
            let split_at = split_at.min(chars.len());

            let (state, p1) = state! {
                doc { root { p1: paragraph { text("placeholder") } } }
                selection: (p1, 0)
            };

            let mut tr = Transaction::new(&state);
            tr.remove_text(p1, 0, "placeholder".chars().count()).unwrap();
            tr.insert_text(p1, 0, &text).unwrap();
            let (state, _, _, _, _) = tr.commit();

            let mut tr = Transaction::new(&state);
            tr.split_node(p1, split_at).unwrap();
            let (split_state, _, _, _, _) = tr.commit();

            let blocks = root_blocks(&split_state);
            prop_assert_eq!(blocks.len(), 2);
            let expected_first: String = text.chars().take(split_at).collect();
            let expected_rest: String = text.chars().skip(split_at).collect();
            prop_assert_eq!(&blocks[0].1, &expected_first);
            prop_assert_eq!(&blocks[1].1, &expected_rest);
        }

        #[test]
        fn merge_paragraphs_concatenates(
            text_a in "[a-z]{0,10}",
            text_b in "[a-z]{0,10}",
        ) {
            let (state, p1, p2) = state! {
                doc {
                    root {
                        p1: paragraph { text("placeholder_a") }
                        p2: paragraph { text("placeholder_b") }
                    }
                }
                selection: (p1, 0)
            };

            let mut tr = Transaction::new(&state);
            tr.remove_text(p1, 0, "placeholder_a".chars().count()).unwrap();
            tr.insert_text(p1, 0, &text_a).unwrap();
            tr.remove_text(p2, 0, "placeholder_b".chars().count()).unwrap();
            tr.insert_text(p2, 0, &text_b).unwrap();
            let (state, ..) = tr.commit();

            let mut tr = Transaction::new(&state);
            tr.merge_node(p1).unwrap();
            let (merged, ..) = tr.commit();

            prop_assert_eq!(block_text(&merged, &p1), format!("{}{}", text_a, text_b));
            prop_assert_eq!(root_blocks(&merged).len(), 1);
        }

        #[test]
        fn move_paragraph_reorders(_seed in 0usize..2) {
            let (state, ..) = state! {
                doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
                selection: (p1, 0)
            };
            let root = root_id(&state);
            let p2_first = state.view().root().unwrap().child_blocks().nth(1).unwrap().id();

            let step = Step::MoveNode {
                block: p2_first,
                old_parent: root,
                old_index: 1,
                new_parent: root,
                new_index: 0,
            };
            let moved = step.apply(&state).unwrap().state;
            let texts: Vec<String> = root_blocks(&moved).into_iter().map(|(_, t)| t).collect();
            prop_assert_eq!(texts, vec!["b".to_string(), "a".to_string()]);
        }

        #[test]
        fn insert_subtree_appears_in_parent(index in 0usize..2) {
            let (state, ..) = state! {
                doc { root { p1: paragraph } }
                selection: (p1, 0)
            };
            let root = root_id(&state);
            let subtree = Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));

            let mut tr = Transaction::new(&state);
            tr.insert_subtree(root, index, subtree).unwrap();
            let (inserted, _, _, _, _) = tr.commit();

            prop_assert_eq!(root_blocks(&inserted).len(), 2);
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum AnchorCmd {
        Move,
        Undo,
        Redo,
        Insert,
    }

    fn arb_anchor_cmd() -> impl Strategy<Value = AnchorCmd> {
        prop_oneof![
            Just(AnchorCmd::Move),
            Just(AnchorCmd::Undo),
            Just(AnchorCmd::Redo),
            Just(AnchorCmd::Insert),
        ]
    }

    fn record_if_any(history: &mut UndoHistory, recorded: Vec<editor_state::undo::RecordedOp>) {
        if !recorded.is_empty() {
            history.record(
                UndoEntry {
                    ops: recorded,
                    tag: None,
                    transient: TransientState::default(),
                    merge: RecordMerge::Isolated,
                },
                Instant::now(),
            );
        }
    }

    proptest! {
        #[test]
        fn stable_position_tracks_marker_through_move_undo_redo_sequence(
            cmds in prop::collection::vec(arb_anchor_cmd(), 1..12),
        ) {
            const MARKER: char = '#';
            let (initial, p1, p2) = state! {
                doc {
                    root {
                        p1: paragraph { text("xx#yy") }
                        p2: paragraph { text("") }
                    }
                }
                selection: (p1, 0)
            };
            let view0 = initial.view();
            let marker_idx0 = "xx#yy".chars().position(|c| c == MARKER).unwrap();
            let captured = StablePosition::capture(
                &Position {
                    node: p1,
                    offset: marker_idx0 + 1,
                    affinity: Affinity::Upstream,
                },
                &view0,
            );

            let mut state = initial;
            let mut history = UndoHistory::new(Duration::from_millis(0));

            for cmd in cmds {
                match cmd {
                    AnchorCmd::Move => {
                        let root = state.view().root().unwrap().id();
                        let (marker_block, old_index) = {
                            let view = state.view();
                            let root_v = view.root().unwrap();
                            let idx = root_v
                                .child_blocks()
                                .position(|b| b.inline_text().contains(MARKER))
                                .expect("marker paragraph must be a root child");
                            let block = root_v.child_blocks().nth(idx).unwrap().id();
                            (block, idx)
                        };
                        let new_index = 1 - old_index;
                        let mut tr = Transaction::new(&state);
                        if tr.move_node(marker_block, root, new_index).is_ok() {
                            let (next, _, recorded, _, _) = tr.commit();
                            record_if_any(&mut history, recorded);
                            state = next;
                        }
                    }
                    AnchorCmd::Undo => {
                        history.undo(state.projected_mut(), TransientState::default());
                    }
                    AnchorCmd::Redo => {
                        history.redo(state.projected_mut(), TransientState::default());
                    }
                    AnchorCmd::Insert => {
                        let len = state.view().node(p2).unwrap().child_count();
                        let mut tr = Transaction::new(&state);
                        if tr.insert_text(p2, len, "z").is_ok() {
                            let (next, _, recorded, _, _) = tr.commit();
                            record_if_any(&mut history, recorded);
                            state = next;
                        }
                    }
                }
            }

            let view = state.view();
            let ctx = StableResolveCtx::new(&view, state.projected.seq());
            let resolved = captured.resolve(&ctx).expect("captured anchor must still resolve");

            let oracle_block = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|b| b.inline_text().contains(MARKER))
                .expect("marker must survive the whole sequence");
            let oracle_text = oracle_block.inline_text();
            let oracle_idx = oracle_text.chars().position(|c| c == MARKER).unwrap();

            prop_assert_eq!(resolved.node, oracle_block.id());
            prop_assert_eq!(resolved.offset, oracle_idx + 1);
        }
    }

    proptest! {
        #[test]
        fn insert_text_inverse_round_trip(text in "[a-z]{1,10}") {
            let (state, p1) = state! {
                doc { root { p1: paragraph { text("") } } }
                selection: (p1, 0)
            };
            let before = snapshot(&state);
            let step = Step::InsertText { block: p1, offset: 0, text: text.clone() };
            let after = step.apply(&state).unwrap().state;
            let restored = step.inverse().apply(&after).unwrap().state;
            prop_assert_eq!(snapshot(&restored), before);
        }

        #[test]
        fn remove_text_inverse_round_trip(text in "[a-z]{1,10}") {
            let (state, p1) = state! {
                doc { root { p1: paragraph { text("placeholder") } } }
                selection: (p1, 0)
            };
            let mut tr = Transaction::new(&state);
            tr.remove_text(p1, 0, "placeholder".chars().count()).unwrap();
            tr.insert_text(p1, 0, &text).unwrap();
            let (state, _, _, _, _) = tr.commit();

            let before = snapshot(&state);
            let step = Step::RemoveText { block: p1, offset: 0, text: text.clone() };
            let after = step.apply(&state).unwrap().state;
            let restored = step.inverse().apply(&after).unwrap().state;
            prop_assert_eq!(snapshot(&restored), before);
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn add_modifier_twice_dispatches_once() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.add_modifier(p1, Modifier::Bold).unwrap();
        tr.add_modifier(p1, Modifier::Bold).unwrap();
        let (next, ..) = tr.commit();
        assert_eq!(
            next.view()
                .node(p1)
                .unwrap()
                .block_modifier(ModifierType::Bold),
            Some(&Modifier::Bold)
        );
    }

    #[test]
    fn split_then_merge_via_transaction() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.split_node(p1, 3).unwrap();
        let (split_state, ..) = tr.commit();
        assert_eq!(root_blocks(&split_state).len(), 2);
    }

    #[test]
    fn range_in_second_half_survives_split_precisely() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("abcd") } } }
            selection: (p1, 0)
        };

        let view_before = state.view();
        let sel_c_to_d = Selection::new(Position::new(p1, 2), Position::new(p1, 4));
        let captured = StableSelection::capture(&sel_c_to_d, &view_before);

        let mut tr = Transaction::new(&state);
        tr.split_node(p1, 2).unwrap();
        let (after, ..) = tr.commit();

        let new_second_half_block = after
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .nth(1)
            .unwrap()
            .id();

        let view_after = after.view();
        let ctx_after = StableResolveCtx::new(&view_after, after.projected.seq());
        let resolved = captured.resolve(&ctx_after).unwrap();

        assert_eq!(resolved.anchor.node, new_second_half_block);
        assert_eq!(resolved.anchor.offset, 0);
        assert_eq!(resolved.head.node, new_second_half_block);
        assert_eq!(resolved.head.offset, 2);
    }

    #[test]
    fn range_in_merged_block_survives_merge_precisely() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("ab") }
                    p2: paragraph { text("cd") }
                }
            }
            selection: (p1, 0)
        };

        let view_before = state.view();
        let sel_in_p2 = Selection::new(Position::new(p2, 0), Position::new(p2, 2));
        let captured = StableSelection::capture(&sel_in_p2, &view_before);

        let mut tr = Transaction::new(&state);
        tr.merge_node(p1).unwrap();
        let (after, ..) = tr.commit();

        let view_after = after.view();
        let ctx_after = StableResolveCtx::new(&view_after, after.projected.seq());
        let resolved = captured.resolve(&ctx_after).unwrap();

        assert_eq!(resolved.anchor.node, p1);
        assert_eq!(resolved.anchor.offset, 2);
        assert_eq!(resolved.head.node, p1);
        assert_eq!(resolved.head.offset, 4);
    }

    #[test]
    fn remove_subtree_removes_from_doc() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("x") } paragraph } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.remove_subtree(p1).unwrap();
        let (removed, ..) = tr.commit();

        assert_eq!(root_blocks(&removed).len(), 1);
    }

    #[test]
    fn add_modifier_inverse_round_trip() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let before = snapshot(&state);
        let step = Step::AddModifier {
            block: p1,
            modifier: Modifier::Bold,
        };
        let after = step.apply(&state).unwrap().state;
        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn split_node_inverse_round_trip() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let before = snapshot(&state);
        let step = Step::SplitNode {
            block: p1,
            offset: 3,
        };
        let after = step.apply(&state).unwrap().state;
        assert_eq!(root_blocks(&after).len(), 2);
        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn merge_node_inverse_round_trip() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } paragraph { text("world") } } }
            selection: (p1, 0)
        };
        let before = snapshot(&state);
        // offset 5 = child count of survivor (p1) before merge.
        let step = Step::MergeNode {
            block: p1,
            offset: 5,
        };
        let after = step.apply(&state).unwrap().state;
        assert_eq!(root_blocks(&after).len(), 1);
        assert_eq!(block_text(&after, &p1), "helloworld");
        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn remove_modifier_inverse_round_trip() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.add_modifier(p1, Modifier::Bold).unwrap();
        let (state, ..) = tr.commit();

        let before = snapshot(&state);
        let bold_before = state
            .view()
            .node(p1)
            .unwrap()
            .block_modifier(ModifierType::Bold)
            .cloned();
        let step = Step::RemoveModifier {
            block: p1,
            modifier: Modifier::Bold,
        };
        let after = step.apply(&state).unwrap().state;
        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
        assert_eq!(
            restored
                .view()
                .node(p1)
                .unwrap()
                .block_modifier(ModifierType::Bold)
                .cloned(),
            bold_before
        );
    }

    #[test]
    fn set_node_inverse_round_trip() {
        let (state, c1) = state! {
            doc { root { c1: callout { paragraph { text("x") } } } }
            selection: (c1, 0)
        };
        let new_node = PlainNode::Callout(PlainCalloutNode {
            variant: CalloutVariant::Warning,
        });
        let old_node = state.view().node(c1).unwrap().node().to_plain();
        let step = Step::SetNode {
            block: c1,
            old_node,
            new_node,
        };
        let after = step.apply(&state).unwrap().state;
        if let Node::Callout(n) = after.view().node(c1).unwrap().node() {
            assert_eq!(*n.variant.get(), CalloutVariant::Warning);
        } else {
            panic!("expected callout");
        }
        let restored = step.inverse().apply(&after).unwrap().state;
        if let Node::Callout(n) = restored.view().node(c1).unwrap().node() {
            assert_eq!(*n.variant.get(), CalloutVariant::Info);
        } else {
            panic!("expected callout");
        }
    }

    #[test]
    fn set_node_on_root_applies_layout_mode() {
        // Layout-mode changes target the implicit root (Dot::ROOT), which is synthetic.
        // set_node must accept it as a NodeAttr target, not narrow via as_op_dot.
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("x") } } }
            selection: (p1, 0)
        };
        let root = root_id(&state);
        let old_node = state.view().node(root).unwrap().node().to_plain();
        let new_node = PlainNode::Root(PlainRootNode {
            layout_mode: LayoutMode::Paginated {
                page_width: 400,
                page_height: 600,
                page_margin_top: 20,
                page_margin_bottom: 20,
                page_margin_left: 20,
                page_margin_right: 20,
            },
        });
        let step = Step::SetNode {
            block: root,
            old_node,
            new_node,
        };
        let after = step.apply(&state).unwrap().state;
        assert!(matches!(
            after.view().node(root).unwrap().node().to_plain(),
            PlainNode::Root(PlainRootNode {
                layout_mode: LayoutMode::Paginated { .. }
            })
        ));
    }

    #[test]
    fn replace_block_type_inverse_round_trip() {
        let (state, list) = state! {
            doc {
                root {
                    list: ordered_list {
                        list_item { paragraph { text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (list, 0)
        };
        let before = snapshot(&state);
        let step = Step::ReplaceBlockType {
            block: list,
            old_type: NodeType::OrderedList,
            new_type: NodeType::BulletList,
        };
        let after = step.apply(&state).unwrap().state;
        let view = after.view();
        let replaced = view
            .alias_classes()
            .resolve_with(list, |dot| view.node(dot).is_some());
        assert_eq!(
            view.node(replaced).unwrap().node_type(),
            NodeType::BulletList
        );

        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn move_node_reorder_and_back() {
        let (state, ..) = state! {
            doc { root { paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: (p2, 0)
        };
        let before = snapshot(&state);
        let root = root_id(&state);
        let p2_elem = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .nth(1)
            .unwrap()
            .id();

        let step = Step::MoveNode {
            block: p2_elem,
            old_parent: root,
            old_index: 1,
            new_parent: root,
            new_index: 0,
        };
        let moved = step.apply(&state).unwrap().state;
        let texts: Vec<String> = root_blocks(&moved).into_iter().map(|(_, t)| t).collect();
        assert_eq!(texts, vec!["b".to_string(), "a".to_string()]);

        // Reverse manually: the "b" block now sits at index 0 with a fresh dot.
        let b_elem = moved
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .id();
        let back = Step::MoveNode {
            block: b_elem,
            old_parent: root,
            old_index: 0,
            new_parent: root,
            new_index: 1,
        };
        let restored = back.apply(&moved).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn move_node_uses_full_child_slot_index() {
        let (state, _p1, p2) = state! {
            doc {
                root {
                    image
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        let root = root_id(&state);
        let mut tr = Transaction::new(&state);
        tr.move_node(p2, root, 1).unwrap();
        let (moved, ..) = tr.commit();

        assert_eq!(
            root_child_labels(&moved),
            vec!["Image".to_string(), "b".to_string(), "a".to_string()]
        );
    }

    #[test]
    fn insert_subtree_inverse_round_trip() {
        let (state, ..) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let before = snapshot(&state);
        let root = root_id(&state);
        let subtree = Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));
        let step = Step::InsertSubtree {
            parent: root,
            index: 1,
            subtree,
        };
        let after = step.apply(&state).unwrap().state;
        assert_eq!(root_blocks(&after).len(), 2);
        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn remove_subtree_inverse_round_trip() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("keep") } paragraph } }
            selection: (p1, 0)
        };
        let before = snapshot(&state);
        let root = root_id(&state);
        let captured = capture(&state, &p1);
        let step = Step::RemoveSubtree {
            parent: root,
            index: 0,
            subtree: captured,
        };
        let after = step.apply(&state).unwrap().state;
        assert_eq!(root_blocks(&after).len(), 1);
        let restored = step.inverse().apply(&after).unwrap().state;
        assert_eq!(snapshot(&restored), before);
    }

    #[test]
    fn insert_subtree_carries_init_attrs_in_single_op() {
        let (state, ..) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let root = root_id(&state);
        let subtree = Subtree::leaf(PlainNode::Callout(PlainCalloutNode {
            variant: CalloutVariant::Warning,
        }));

        let mut tr = Transaction::new(&state);
        tr.insert_subtree(root, 1, subtree).unwrap();
        let (inserted, _, _, _, _) = tr.commit();

        let view = inserted.view();
        let callout = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|b| b.node_type() == NodeType::Callout)
            .expect("callout inserted");
        let callout_dot = callout.dot().unwrap();
        let PlainNode::Callout(plain) = callout.node().to_plain() else {
            panic!("callout plain node expected");
        };
        assert_eq!(
            plain.variant,
            CalloutVariant::Warning,
            "init attrs가 투영에 시딩돼야 한다"
        );

        let callout_attr_ops = inserted
            .graph()
            .ordered_ops()
            .expect("storage order is ancestry-first")
            .iter()
            .filter(|op| {
                matches!(&op.payload, EditOp::NodeAttr(NodeAttrOp { target, .. }) if *target == callout_dot)
            })
            .count();
        assert_eq!(
            callout_attr_ops, 0,
            "삽입 블록의 init은 단일 op — 이 블록을 향한 NodeAttrOp 방출 금지"
        );
    }

    #[test]
    fn insert_root_subtree_is_rejected() {
        let (state, ..) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let root = root_id(&state);
        let subtree = Subtree::leaf(PlainNode::Root(PlainRootNode::default()));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(root, 1, subtree);
        assert!(
            matches!(result, Err(StepError::RootSubtree)),
            "root subtree 삽입은 정확히 StepError::RootSubtree로 거부돼야 한다"
        );
    }

    fn capture(state: &State, block: &Dot) -> Subtree {
        // Build a Subtree mirroring the block: a paragraph with its text.
        let view = state.view();
        let nv = view.node(*block).unwrap();
        Subtree::leaf(nv.node().to_plain()).with_children(vec![Subtree::leaf(PlainNode::Text(
            editor_model::PlainTextNode {
                text: nv.inline_text(),
            },
        ))])
    }

    fn tab_count(state: &State, block: &Dot) -> usize {
        state
            .view()
            .node(*block)
            .unwrap()
            .children()
            .filter(|c| matches!(c, ChildView::Leaf(l) if l.as_atom() == Some(&AtomLeaf::Tab)))
            .count()
    }

    #[test]
    fn atom_tab_survives_build_and_move() {
        // build_state_from_plain must emit inline atoms (state! with bare `tab`).
        let (state, p1, _p2) = state! {
            doc {
                root {
                    p1: paragraph { text("a") tab text("b") }
                    p2: paragraph { text("c") }
                }
            }
            selection: (p1, 0)
        };
        assert_eq!(block_text(&state, &p1), "ab");
        assert_eq!(
            tab_count(&state, &p1),
            1,
            "build_state_from_plain emits the inline tab atom"
        );

        // Moving the paragraph routes through capture_subtree + emit_subtree; the
        // inline atom must survive onto the moved block's fresh dot.
        let root = root_id(&state);
        let moved = Step::MoveNode {
            block: p1,
            old_parent: root,
            old_index: 0,
            new_parent: root,
            new_index: 1,
        }
        .apply(&state)
        .unwrap()
        .state;

        let moved_p1 = moved
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .nth(1)
            .unwrap()
            .id();
        assert_eq!(
            block_text(&moved, &moved_p1),
            "ab",
            "text survives the move"
        );
        assert_eq!(
            tab_count(&moved, &moved_p1),
            1,
            "inline tab survives the move (capture_subtree + emit_subtree atom support)"
        );
    }

    #[test]
    fn atom_block_image_builds_at_root() {
        // Block-level atom (image) as a direct child of root → SeqItem::BlockAtom.
        let (state, _p1) = state! {
            doc { root { image p1: paragraph { text("x") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        let root = view.root().unwrap();
        let first = root.child_at(0).unwrap();
        match first {
            ChildView::Leaf(l) => {
                assert!(
                    matches!(l.as_atom(), Some(AtomLeaf::Image { .. })),
                    "root's first child is the block image atom"
                );
            }
            ChildView::Block(_) => panic!("expected the image atom as a leaf child of root"),
        }
    }
}
