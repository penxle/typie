use editor_commands::{self as commands};
use editor_model::NodeType;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_block_op(editor: &mut Editor, op: BlockOp) -> Result<(), EditorError> {
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
            BlockOp::WrapFold => commands::chain!(
                tr,
                commands::optional!(commands::materialize_gap_paragraph()),
                commands::optional!(commands::materialize_synthetic_selection_blocks()),
                commands::wrap_selected_blocks_in_fold(),
            ),
        }?;
        Ok(())
    })
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
