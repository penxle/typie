use editor_model::{Node, NodeType};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    lift_selected_block_run, promote_list_run, resolve_selected_block_run_for_wrapper,
};

pub fn lift_selected_blocks_from_callout(tr: &mut Transaction) -> CommandResult {
    let Some(run) = resolve_selected_block_run_for_wrapper(tr, NodeType::Callout)? else {
        return Ok(false);
    };
    let run = promote_list_run(tr, run)?;
    {
        let view = tr.view();
        let Some(wrapper) = view.node(run.parent_id) else {
            return Ok(false);
        };
        if !matches!(wrapper.node(), Node::Callout(_)) {
            return Ok(false);
        }
    }
    lift_selected_block_run(tr, run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use editor_macros::state;

    #[test]
    fn lifts_middle_block_and_preserves_variant_on_remnants() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Warning) {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                        paragraph { text("C") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_selected_blocks_from_callout(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Warning) {
                        paragraph { text("A") }
                    }
                    p2: paragraph { text("B") }
                    callout(variant: CalloutVariant::Warning) {
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
    fn lifts_only_the_selected_list_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout(variant: CalloutVariant::Danger) {
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
        let (actual, ..) = transact!(initial, |tr| lift_selected_blocks_from_callout(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    callout(variant: CalloutVariant::Danger) {
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
    fn rejects_selection_crossing_callout_boundary() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph { text("A") } }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| lift_selected_blocks_from_callout(&mut tr));
    }
}
