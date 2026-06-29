use editor_model::ChildView;
use editor_transaction::Transaction;

use crate::helpers::{find_enclosing_list_item_id, lift_list_item_inner};
use crate::{CommandError, CommandResult};

pub fn lift_first_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let list_item_id = {
        let view = tr.view();
        let Some(list_item_id) = find_enclosing_list_item_id(&view, pos.node) else {
            return Ok(false);
        };
        let list_item = view
            .node(list_item_id)
            .ok_or(CommandError::NodeNotFound(list_item_id))?;
        let paragraph_id = match list_item.first_child() {
            Some(ChildView::Block(p)) => p.id(),
            _ => {
                return Err(CommandError::Corrupted(
                    "list_item missing paragraph".into(),
                ));
            }
        };
        if pos.node != paragraph_id {
            return Ok(false);
        }
        if list_item.index() != Some(0) {
            return Ok(false);
        }
        list_item_id
    };

    lift_list_item_inner(tr, list_item_id)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn lift_first_item_top_level() {
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
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_first_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 1)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }

    #[test]
    fn prev_item_exists_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }

    #[test]
    fn not_at_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_first_list_item(&mut tr));
    }
}
