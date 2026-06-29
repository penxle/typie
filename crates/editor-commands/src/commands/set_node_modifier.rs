use editor_crdt::Dot;
use editor_model::Modifier;
use editor_transaction::Transaction;

use crate::helpers::{apply_modifier_to_node, is_unit_variant};
use crate::{CommandError, CommandResult};

pub fn set_node_modifier(tr: &mut Transaction, id: Dot, modifier: Modifier) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }

    apply_modifier_to_node(tr, id, &modifier)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sets_font_size_on_root_as_document_default() {
        let (initial, r, ..) = state! {
            doc {
                r: root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(2400)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn missing_node_id_errors() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| set_node_modifier(
            &mut tr,
            Dot::new(u64::MAX, 1),
            Modifier::FontSize { value: 2400 },
        ));
        assert!(matches!(err, CommandError::NodeNotFound(_)));
    }

    #[test]
    fn rejects_unit_modifier() {
        let (initial, r, ..) = state! {
            doc { r: root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| set_node_modifier(&mut tr, r, Modifier::Bold,));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }
}
