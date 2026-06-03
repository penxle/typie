use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    find_enclosing_list_item_id, is_at_list_item_content_start, sink_list_item_inner,
};

// Tab at a list item's content start indents it. Consumes the key (Ok(true))
// whenever the caret is at a list item's content start — even for the first
// item, where there is nothing to sink into — so Tab never falls through to
// literal tab insertion at a list item's start.
pub fn sink_list_item_at_start(tr: &mut Transaction) -> CommandResult {
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
    sink_list_item_inner(tr, item_id)?;
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
        transact_fail!(initial, |tr| sink_list_item_at_start(&mut tr));
    }

    #[test]
    fn mid_list_item_text_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("AB") } } }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| sink_list_item_at_start(&mut tr));
    }

    #[test]
    fn consumes_at_first_item_start_without_change() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        // First item cannot sink, but the key is consumed (transact! asserts Ok(true))
        // and the document is unchanged.
        let (actual, ..) = transact!(initial, |tr| sink_list_item_at_start(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sinks_at_second_item_start() {
        let (initial, ..) = state! {
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
        let (actual, ..) = transact!(initial, |tr| sink_list_item_at_start(&mut tr));
        let (expected, ..) = state! {
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
        assert_state_eq!(&actual, &expected);
    }
}
