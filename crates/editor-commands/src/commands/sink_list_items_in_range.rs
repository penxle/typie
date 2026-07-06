use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::sink_selected_list_items;

pub fn sink_list_items_in_range(tr: &mut Transaction) -> CommandResult {
    if tr
        .selection()
        .is_none_or(|selection| selection.is_collapsed())
    {
        return Ok(false);
    }
    sink_selected_list_items(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_returns_false() {
        let (initial, _) = state! {
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
        let (actual, ..) = transact_fail!(initial, |tr| sink_list_items_in_range(&mut tr));
        let (expected, _) = state! {
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
}
