use editor_model::{BlockquoteVariant, Node, NodeType};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    lift_selected_block_run, promote_list_run, resolve_selected_block_run_for_wrapper,
};

pub fn lift_selected_blocks_from_blockquote_with_variant(
    tr: &mut Transaction,
    variant: BlockquoteVariant,
) -> CommandResult {
    let Some(run) = resolve_selected_block_run_for_wrapper(tr, NodeType::Blockquote)? else {
        return Ok(false);
    };
    let run = promote_list_run(tr, run)?;
    {
        let view = tr.view();
        let Some(wrapper) = view.node(run.parent_id) else {
            return Ok(false);
        };
        let Node::Blockquote(blockquote) = wrapper.node() else {
            return Ok(false);
        };
        if *blockquote.variant.get() != variant {
            return Ok(false);
        }
    }
    lift_selected_block_run(tr, run)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::BlockquoteVariant;

    use super::*;
    use crate::test_utils::*;

    fn lift(initial: editor_state::State) -> editor_state::State {
        transact!(initial, |tr| {
            lift_selected_blocks_from_blockquote_with_variant(
                &mut tr,
                BlockquoteVariant::MessageSent,
            )
        })
        .0
    }

    #[test]
    fn lifts_first_selected_block() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("A") }
                        paragraph { text("B") }
                        paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("B") }
                        paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_middle_selected_block_and_splits_wrapper() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                        paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("A") }
                    }
                    p2: paragraph { text("B") }
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_last_selected_block() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("A") }
                        paragraph { text("B") }
                        p3: paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p3, 0) -> (p3, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        paragraph { text("A") }
                        paragraph { text("B") }
                    }
                    p3: paragraph { text("C") }
                    paragraph {}
                }
            }
            selection: (p3, 0) -> (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_all_selected_blocks_and_removes_wrapper() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_only_the_selected_list_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { p1: paragraph { text("A") } }
                            list_item { paragraph { text("B") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { paragraph { text("B") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_middle_list_item_and_splits_wrapper() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { paragraph { text("A") } }
                            list_item { p2: paragraph { text("B") } }
                            list_item { paragraph { text("C") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { paragraph { text("A") } }
                        }
                    }
                    bullet_list {
                        list_item { p2: paragraph { text("B") } }
                    }
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { paragraph { text("C") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_partial_list_suffix_with_following_sibling() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { paragraph { text("A") } }
                            list_item { p2: paragraph { text("B") } }
                            list_item { paragraph { text("C") } }
                        }
                        after: paragraph { text("D") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (after, 1)
        };
        let actual = lift(initial);
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        bullet_list {
                            list_item { paragraph { text("A") } }
                        }
                    }
                    bullet_list {
                        list_item { p2: paragraph { text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    after: paragraph { text("D") }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (after, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn rejects_selection_crossing_wrapper_boundary() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote(variant: BlockquoteVariant::MessageSent) {
                        p1: paragraph { text("A") }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };

        transact_fail!(initial, |tr| {
            lift_selected_blocks_from_blockquote_with_variant(
                &mut tr,
                BlockquoteVariant::MessageSent,
            )
        });
    }
}
