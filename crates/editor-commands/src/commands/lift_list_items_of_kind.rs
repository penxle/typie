use editor_model::NodeType;
use editor_transaction::Transaction;

use crate::helpers::{lift_list_item_inner, lift_list_items_planned};
use crate::judgments::judge_lift_list_items_of_kind;
use crate::types::ListVerdict;
use crate::{CommandError, CommandResult};

pub fn lift_list_items_of_kind(tr: &mut Transaction, target_list_type: NodeType) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let plan = {
        let view = tr.view();
        if selection.anchor != selection.head && selection.resolve(&view).is_none() {
            return Err(CommandError::Corrupted(
                "cannot resolve list selection".into(),
            ));
        }
        match judge_lift_list_items_of_kind(&view, &selection, target_list_type) {
            ListVerdict::NotApplicable => return Ok(false),
            ListVerdict::AbsorbOnly => return Ok(true),
            ListVerdict::Change(plan) => plan,
        }
    };
    if selection.anchor == selection.head {
        return lift_list_item_inner(tr, plan.items[0]);
    }
    lift_list_items_planned(tr, plan.items)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeType;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_target_kind_lifts_current_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("A") } paragraph {} } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn middle_target_kind_item_lifts_and_splits_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p2: paragraph { text("B") }
                    bullet_list { list_item { paragraph { text("C") } } }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn different_kind_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
    }

    #[test]
    fn mixed_plain_paragraph_and_target_list_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { p2: paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
    }

    #[test]
    fn fully_selected_unsupported_wrapper_keeps_internal_target_list_unchanged() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote {
                        bullet_list {
                            list_item { p1: paragraph { text("A") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let expected = initial.clone();

        let (actual, steps, ..) = transact_fail!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));

        assert!(steps.is_empty());
        assert_state_eq!(&actual, &expected);
    }
}
