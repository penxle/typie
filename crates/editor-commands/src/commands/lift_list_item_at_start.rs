use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    find_enclosing_list_item_id, is_at_list_item_content_start, lift_list_item_inner,
};

// Shift+Tab at a list item's content start outdents it. Consumes the key
// (Ok(true)) whenever the caret is at a list item's content start, so Shift+Tab
// never falls through to "delete preceding tab" at a list item's start.
pub fn lift_list_item_at_start(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let doc = tr.doc();
    if !is_at_list_item_content_start(&doc, &selection) {
        return Ok(false);
    }
    let Some(item_id) = find_enclosing_list_item_id(&doc, selection.head.node_id) else {
        return Ok(false);
    };
    lift_list_item_inner(tr, item_id)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn not_at_list_item_start_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| lift_list_item_at_start(&mut tr));
    }

    #[test]
    fn lifts_at_nested_item_start() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { paragraph { t1: text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item_at_start(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
