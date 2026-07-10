use editor_model::{BlockquoteVariant, NodeType, PlainBlockquoteNode, PlainNode};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    normalize_selected_block_run, promote_list_run, resolve_selected_block_run_for_wrapper,
};

pub fn normalize_selected_blocks_in_blockquote(
    tr: &mut Transaction,
    variant: BlockquoteVariant,
) -> CommandResult {
    let Some(run) = resolve_selected_block_run_for_wrapper(tr, NodeType::Blockquote)? else {
        return Ok(false);
    };
    let run = promote_list_run(tr, run)?;
    normalize_selected_block_run(
        tr,
        run,
        PlainNode::Blockquote(PlainBlockquoteNode { variant }),
    )
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::BlockquoteVariant;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn wraps_collapsed_paragraph_and_preserves_offset() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::LeftQuote,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::LeftQuote) {
                        p1: paragraph { text("Hello") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_selected_sibling_paragraphs() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::LeftLine,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merges_wrapper_islands_and_outside_paragraph_preserving_selection() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::LeftQuote) {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    middle: paragraph { text("C") }
                    blockquote(variant: BlockquoteVariant::MessageReceived) {
                        p4: paragraph { text("D") }
                        paragraph { text("E") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p4, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::MessageSent,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                        middle: paragraph { text("C") }
                        p4: paragraph { text("D") }
                        paragraph { text("E") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p4, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn absorbs_paragraphs_before_and_after_existing_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    blockquote { p2: paragraph { text("B") } }
                    p3: paragraph { text("C") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p3, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::LeftLine,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        p2: paragraph { text("B") }
                        p3: paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn preserves_empty_existing_blockquote_while_absorbing_neighbors() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    blockquote {}
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::LeftLine,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        paragraph {}
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_only_the_selected_list_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::LeftLine,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list {
                            list_item { p1: paragraph { text("A") } }
                        }
                    }
                    bullet_list {
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn rejects_selection_with_unsupported_block() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    paragraph { text("A") }
                    image
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 2, <)
        };

        transact_fail!(initial, |tr| normalize_selected_blocks_in_blockquote(
            &mut tr,
            BlockquoteVariant::LeftLine,
        ));
    }

    #[test]
    fn rejects_incompatible_target_before_materializing_list_segment() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                    }
                    image
                    p3: paragraph { text("C") }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p3, 1)
        };
        let expected = initial.clone();

        let (actual, steps, ..) = transact_fail!(initial, |tr| {
            normalize_selected_blocks_in_blockquote(&mut tr, BlockquoteVariant::LeftLine)
        });

        assert!(steps.is_empty());
        assert_state_eq!(&actual, &expected);
    }
}
