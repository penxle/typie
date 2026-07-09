use editor_commands as commands;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_node_op(editor: &mut Editor, op: NodeOp) -> Result<(), EditorError> {
    editor.transact(|tr| match op {
        NodeOp::SetAttrs { id, attrs } => {
            tr.set_node(id, attrs)?;
            Ok(())
        }
        NodeOp::Delete { id } => {
            commands::delete_node(tr, id)?;
            Ok(())
        }
        NodeOp::CycleCalloutVariant { id } => {
            commands::cycle_callout_variant(tr, id)?;
            Ok(())
        }
        NodeOp::Unwrap { id } => {
            commands::unwrap_node(tr, id)?;
            Ok(())
        }
        NodeOp::Table { id, op } => match op {
            TableOp::InsertAxis {
                axis,
                index,
                before,
            } => {
                commands::insert_table_axis(tr, id, axis, index, before)?;
                Ok(())
            }
            TableOp::DeleteAxis { axis, index } => {
                commands::delete_table_axis(tr, id, axis, index)?;
                Ok(())
            }
            TableOp::MoveAxis { axis, from, to } => {
                commands::move_table_axis(tr, id, axis, from, to)?;
                Ok(())
            }
            TableOp::SelectAxis { axis, index } => {
                commands::select_table_axis(tr, id, axis, index)?;
                Ok(())
            }
            TableOp::SetColumnWidths { widths } => {
                commands::set_table_column_widths(tr, id, widths)?;
                Ok(())
            }
            TableOp::SetBorderStyle { border_style } => {
                commands::set_table_border_style(tr, id, border_style)?;
                Ok(())
            }
            TableOp::SetProportion { proportion } => {
                commands::set_table_proportion(tr, id, proportion)?;
                Ok(())
            }
            TableOp::SetAxisBackgroundColor { axis, index, color } => {
                commands::set_table_axis_background_color(tr, id, axis, index, color)?;
                Ok(())
            }
            TableOp::SetCellBackgroundColor { color } => {
                commands::set_table_cell_background_color(tr, id, color)?;
                Ok(())
            }
        },
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use editor_macros::state;
    use editor_model::{
        CalloutVariant, ChildView, HorizontalRuleVariant, Node, PlainDoc, PlainHorizontalRuleNode,
        PlainNode, PlainNodeEntry,
    };
    use editor_state::{Affinity, Position, Selection, State, assert_state_eq};

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_delete_node() {
        let (state, _r, img) = state! {
            doc { r: root {
                paragraph { text("a") }
                img: image
            } }
            selection: (r, 1, >) -> (r, 1, <)
        };
        assert_probe_predicts_apply(
            state,
            Message::Node {
                op: NodeOp::Delete { id: img },
            },
        );
    }

    #[test]
    fn delete_node_removes_selected_external_block_and_records_history() {
        let (initial, _root, _p1, img, ..) = state! {
            doc { r: root {
                p1: paragraph { text("Before") }
                img: image
                p2: paragraph { text("After") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Node {
            op: NodeOp::Delete { id: img },
        });

        let (deleted, ..) = state! {
            doc { root {
                p1: paragraph { text("Before") }
                p2: paragraph { text("After") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &deleted);
        assert!(editor.undo_history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_state_eq!(editor.state(), &deleted);
    }

    #[test]
    fn set_attrs_updates_block_atom_leaf_and_records_history() {
        fn entry(node: PlainNode, children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
            PlainNodeEntry {
                node,
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children,
            }
        }

        let doc = PlainDoc {
            root: entry(
                PlainNode::Root(Default::default()),
                vec![
                    entry(
                        PlainNode::HorizontalRule(PlainHorizontalRuleNode {
                            variant: HorizontalRuleVariant::Diamond,
                        }),
                        vec![],
                    ),
                    entry(PlainNode::Paragraph(Default::default()), vec![]),
                ],
            ),
        };
        let mut initial = State::from_plain(&doc).unwrap();
        let root = initial.view().root().unwrap().id();
        let hr = match initial.view().root().unwrap().child_at(0).unwrap() {
            ChildView::Leaf(leaf) => leaf.dot(),
            ChildView::Block(_) => panic!("expected horizontal rule leaf"),
        };
        initial.selection = Some(Selection::new(
            Position {
                node: root,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: root,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        ));
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Node {
            op: NodeOp::SetAttrs {
                id: hr,
                attrs: PlainNode::HorizontalRule(PlainHorizontalRuleNode {
                    variant: HorizontalRuleVariant::Zigzag,
                }),
            },
        });

        let variant = match editor.state().view().leaf(hr).unwrap().node().unwrap() {
            Node::HorizontalRule(horizontal_rule) => *horizontal_rule.variant.get(),
            other => panic!("expected horizontal rule, got {other:?}"),
        };
        assert_eq!(variant, HorizontalRuleVariant::Zigzag);
        let block_state = crate::block_state::resolve_block_state(editor.state()).unwrap();
        let block_state_variant = block_state
            .nodes
            .iter()
            .find_map(|block| match &block.node {
                PlainNode::HorizontalRule(horizontal_rule) if block.id == hr => {
                    Some(horizontal_rule.variant)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(block_state_variant, HorizontalRuleVariant::Zigzag);
        assert!(editor.undo_history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        let variant = match editor.state().view().leaf(hr).unwrap().node().unwrap() {
            Node::HorizontalRule(horizontal_rule) => *horizontal_rule.variant.get(),
            other => panic!("expected horizontal rule, got {other:?}"),
        };
        assert_eq!(variant, HorizontalRuleVariant::Zigzag);
    }
    #[test]
    fn cycle_callout_variant_updates_target_and_records_history() {
        let (initial, co, ..) = state! {
            doc { root {
                co: callout {
                    p1: paragraph { text("body") }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Node {
            op: NodeOp::CycleCalloutVariant { id: co },
        });

        let variant = match editor.state().view().node(co).unwrap().node() {
            Node::Callout(callout) => *callout.variant.get(),
            other => panic!("expected callout, got {other:?}"),
        };
        assert_eq!(variant, CalloutVariant::Success);
        assert!(editor.undo_history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        let variant = match editor.state().view().node(co).unwrap().node() {
            Node::Callout(callout) => *callout.variant.get(),
            other => panic!("expected callout, got {other:?}"),
        };
        assert_eq!(variant, CalloutVariant::Success);
    }

    #[test]
    fn unwrap_node_lifts_blockquote_contents_and_records_history() {
        let (initial, bq, ..) = state! {
            doc { root {
                bq: blockquote {
                    p1: paragraph { text("quote") }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Node {
            op: NodeOp::Unwrap { id: bq },
        });

        let (unwrapped, ..) = state! {
            doc { root {
                p1: paragraph { text("quote") }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &unwrapped);
        assert!(editor.undo_history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_state_eq!(editor.state(), &unwrapped);
    }
}
