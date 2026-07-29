use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::is_in_direct_list_item_paragraph;
use crate::judgments::lift_selected_list_items;

// Shift+Tab anywhere in a direct list-item paragraph outdents the whole item.
pub fn lift_list_item_at_caret(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    {
        let view = tr.view();
        if !is_in_direct_list_item_paragraph(&view, &selection) {
            return Ok(false);
        }
    }
    lift_selected_list_items(tr)?;
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
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| lift_list_item_at_caret(&mut tr));
    }

    #[test]
    fn lifts_at_nested_item_start() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { p1: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item_at_caret(&mut tr));
        let (expected, ..) = state! {
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
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lifts_from_later_direct_paragraph_and_preserves_position() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item {
                                    paragraph { text("B") }
                                    p2: paragraph { text("CD") }
                                }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item_at_caret(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item {
                            paragraph { text("B") }
                            p2: paragraph { text("CD") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
