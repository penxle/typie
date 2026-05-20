use editor_model::{Modifier, NodeId};
use editor_transaction::Transaction;

use crate::helpers::{apply_modifier_to_node, is_unit_variant};
use crate::{CommandError, CommandResult};

pub fn set_node_modifier(tr: &mut Transaction, id: NodeId, modifier: Modifier) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }

    let doc = tr.doc();
    let node = doc.node(id).ok_or(CommandError::NodeNotFound(id))?;
    apply_modifier_to_node(tr, &node, &modifier)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sets_font_size_on_root_as_document_default() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_node_modifier(
            &mut tr,
            NodeId::ROOT,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(2400)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn missing_node_id_errors() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| set_node_modifier(
            &mut tr,
            NodeId::new(),
            Modifier::FontSize { value: 2400 },
        ));
        assert!(matches!(err, CommandError::NodeNotFound(_)));
    }

    #[test]
    fn rejects_unit_modifier() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| set_node_modifier(
            &mut tr,
            NodeId::ROOT,
            Modifier::Bold,
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }
}
