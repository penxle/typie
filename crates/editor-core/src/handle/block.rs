use editor_commands::{self as commands};
use editor_crdt::Dot;
use editor_model::{Node, NodeType};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_block_op(editor: &mut Editor, op: BlockOp) -> Result<(), EditorError> {
    let mut created_fold = None;
    editor.transact(|tr| {
        match op {
            BlockOp::ToggleBlockquote { variant } => commands::chain!(
                tr,
                commands::optional!(commands::materialize_gap_paragraph()),
                commands::optional!(commands::materialize_synthetic_selection_blocks()),
                commands::optional!(commands::materialize_synthetic_selected_block_children(
                    NodeType::Blockquote
                )),
                |tr| commands::first!(
                    tr,
                    commands::lift_selected_blocks_from_blockquote_with_variant(variant),
                    commands::set_enclosing_blockquote_variant(variant),
                    commands::normalize_selected_blocks_in_blockquote(variant),
                ),
            ),
            BlockOp::ToggleCallout => commands::chain!(
                tr,
                commands::optional!(commands::materialize_gap_paragraph()),
                commands::optional!(commands::materialize_synthetic_selection_blocks()),
                commands::optional!(commands::materialize_synthetic_selected_block_children(
                    NodeType::Callout
                )),
                |tr| commands::first!(
                    tr,
                    commands::lift_selected_blocks_from_callout(),
                    commands::normalize_selected_blocks_in_callout(),
                ),
            ),
            BlockOp::WrapFold => {
                let enclosing_before = selection_enclosing_fold(tr);
                let applied = commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::materialize_synthetic_selection_blocks()),
                    commands::wrap_selected_blocks_in_fold(),
                )?;
                if applied {
                    created_fold =
                        selection_enclosing_fold(tr).filter(|id| Some(*id) != enclosing_before);
                }
                Ok(applied)
            }
        }?;
        Ok(())
    })?;

    // Legacy parity: folds default collapsed, so a freshly created one is expanded explicitly.
    if let Some(fold_id) = created_fold {
        editor.set_fold_expanded(fold_id, true);
    }
    Ok(())
}

fn selection_enclosing_fold(tr: &Transaction) -> Option<Dot> {
    let selection = tr.selection()?;
    let view = tr.view();
    let node = view.node(selection.head.node).or_else(|| {
        view.leaf(selection.head.node)
            .and_then(|leaf| leaf.parent())
    })?;
    if matches!(node.node(), Node::Fold(_)) {
        return Some(node.id());
    }
    node.ancestors()
        .find(|ancestor| matches!(ancestor.node(), Node::Fold(_)))
        .map(|ancestor| ancestor.id())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::BlockquoteVariant;
    use editor_state::assert_state_eq;

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    fn block_message(op: BlockOp) -> Message {
        Message::Block { op }
    }

    fn root_block_types(state: &editor_state::State) -> Vec<NodeType> {
        state
            .view()
            .root()
            .expect("test document has root")
            .child_blocks()
            .map(|block| block.node_type())
            .collect()
    }

    #[test]
    fn matching_blockquote_variant_lifts_selected_block() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("A") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleBlockquote {
            variant: BlockquoteVariant::MessageSent,
        }));

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("A") } paragraph {} } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn matching_blockquote_variant_lifts_explicitly_selected_wrapper() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleBlockquote {
            variant: BlockquoteVariant::MessageSent,
        }));

        assert_eq!(
            root_block_types(editor.state()),
            vec![
                NodeType::Paragraph,
                NodeType::Paragraph,
                NodeType::Paragraph
            ],
        );
    }

    #[test]
    fn matching_blockquote_variant_lifts_explicitly_selected_empty_wrapper() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {}
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleBlockquote {
            variant: BlockquoteVariant::MessageSent,
        }));

        assert_eq!(
            root_block_types(editor.state()),
            vec![NodeType::Paragraph, NodeType::Paragraph],
        );
    }

    #[test]
    fn different_blockquote_variant_updates_explicitly_selected_wrapper() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("A") }
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleBlockquote {
            variant: BlockquoteVariant::LeftQuote,
        }));

        let (expected, _root, ..) = state! {
            doc {
                root: root {
                    blockquote(variant: BlockquoteVariant::LeftQuote) {
                        p1: paragraph { text("A") }
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn different_blockquote_variant_updates_explicitly_selected_empty_wrapper() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {}
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleBlockquote {
            variant: BlockquoteVariant::LeftQuote,
        }));

        let (expected, _root, ..) = state! {
            doc {
                root: root {
                    blockquote(variant: BlockquoteVariant::LeftQuote) {
                        paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn callout_toggle_lifts_explicitly_selected_wrapper() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    callout {
                        p1: paragraph { text("A") }
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleCallout));

        assert_eq!(
            root_block_types(editor.state()),
            vec![NodeType::Paragraph, NodeType::Paragraph],
        );
    }

    #[test]
    fn callout_toggle_lifts_explicitly_selected_empty_wrapper() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    callout {}
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::ToggleCallout));

        assert_eq!(
            root_block_types(editor.state()),
            vec![NodeType::Paragraph, NodeType::Paragraph],
        );
    }

    #[test]
    fn fold_message_materializes_synthetic_empty_paragraph_and_selects_title() {
        let (initial, ..) = state! {
            doc { root { p1: synthetic paragraph {} } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::WrapFold));

        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        title: fold_title {}
                        fold_content { paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (title, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn fold_message_preserves_range_from_synthetic_blockquote_content() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { empty: synthetic paragraph {} }
                    p2: paragraph { text("B") }
                }
            }
            selection: (empty, 0) -> (p2, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(block_message(BlockOp::WrapFold));

        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            blockquote { empty: paragraph {} }
                            p2: paragraph { text("B") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (empty, 0) -> (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn wrap_fold_expands_created_fold() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } paragraph {} } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        editor.apply(block_message(BlockOp::WrapFold));

        let fold = editor
            .state()
            .view()
            .root()
            .expect("test document has root")
            .child_blocks()
            .find(|block| block.node_type() == NodeType::Fold)
            .expect("fold created")
            .id();
        assert!(
            editor.fold_expanded(fold),
            "freshly created fold must be expanded"
        );
    }

    #[test]
    fn wrap_fold_expands_only_the_inner_fold() {
        let (initial, f1, fc, _p1) = state! {
            doc {
                root {
                    f1: fold {
                        fold_title { text("Outer") }
                        fc: fold_content { p1: paragraph { text("Body") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        editor.apply(block_message(BlockOp::WrapFold));

        let inner = editor
            .state()
            .view()
            .node(fc)
            .expect("fold content exists")
            .child_blocks()
            .find(|block| block.node_type() == NodeType::Fold)
            .expect("inner fold created")
            .id();
        assert!(editor.fold_expanded(inner), "inner fold must be expanded");
        assert!(
            !editor.fold_expanded(f1),
            "pre-existing outer fold keeps its collapsed default"
        );
    }

    #[test]
    fn probe_predicts_valid_and_invalid_fold_messages() {
        let (valid, ..) = state! {
            doc { root { p1: paragraph { text("A") } paragraph {} } }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(valid, block_message(BlockOp::WrapFold));

        let (invalid, ..) = state! {
            doc {
                root {
                    fold {
                        title: fold_title { text("Title") }
                        fold_content { paragraph { text("Body") } }
                    }
                    paragraph {}
                }
            }
            selection: (title, 0)
        };
        let mut editor = Editor::new_test(invalid.clone());
        assert!(!editor.can(block_message(BlockOp::WrapFold)).unwrap());
        assert_probe_predicts_apply(invalid, block_message(BlockOp::WrapFold));
    }
}
