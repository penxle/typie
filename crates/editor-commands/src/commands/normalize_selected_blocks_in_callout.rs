use editor_model::{NodeType, PlainCalloutNode, PlainNode};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    normalize_selected_block_run, promote_list_run, resolve_selected_block_run_for_wrapper,
};

pub fn normalize_selected_blocks_in_callout(tr: &mut Transaction) -> CommandResult {
    let Some(run) = resolve_selected_block_run_for_wrapper(tr, NodeType::Callout)? else {
        return Ok(false);
    };
    let run = promote_list_run(tr, run)?;
    let wrapper = {
        let view = tr.view();
        run.blocks
            .iter()
            .find(|block| block.node_type == NodeType::Callout)
            .and_then(|block| view.node(block.id))
            .map(|callout| callout.node().to_plain())
            .unwrap_or_else(|| PlainNode::Callout(PlainCalloutNode::default()))
    };
    normalize_selected_block_run(tr, run, wrapper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use editor_macros::state;

    #[test]
    fn wraps_collapsed_paragraph_and_preserves_selection() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    paragraph {}
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_callout(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph { text("Hello") } }
                    paragraph {}
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merges_callout_islands_and_keeps_first_variant() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Warning) {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    middle: paragraph { text("C") }
                    callout(variant: CalloutVariant::Danger) {
                        p4: paragraph { text("D") }
                        paragraph { text("E") }
                    }
                    paragraph {}
                }
            }
            selection: (p4, 1) -> (p2, 0)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_callout(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Warning) {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                        middle: paragraph { text("C") }
                        p4: paragraph { text("D") }
                        paragraph { text("E") }
                    }
                    paragraph {}
                }
            }
            selection: (p4, 1) -> (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn absorbs_outside_paragraph_into_existing_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Info) {
                        p1: paragraph { text("A") }
                    }
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_callout(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Info) {
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
    fn preserves_empty_existing_callout_while_absorbing_neighbors() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    callout(variant: CalloutVariant::Info) {}
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };

        let (actual, ..) = transact!(initial, |tr| normalize_selected_blocks_in_callout(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Info) {
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
}
