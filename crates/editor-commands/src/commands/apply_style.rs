use editor_model::NodeId;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn apply_style(tr: &mut Transaction, node_id: NodeId, style_id: String) -> CommandResult {
    let node = tr
        .state()
        .doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;
    if !node.spec().is_textblock() {
        return Err(CommandError::InvalidArgument(
            "style can only be applied to textblock nodes".into(),
        ));
    }
    if node.entry().style.get().as_deref() == Some(style_id.as_str()) {
        return Ok(false);
    }
    tr.set_node_style(node_id, Some(style_id))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn applies_style_to_node() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| apply_style(&mut tr, p1, "heading-1".into()));
        let entry = actual.doc.get_entry(p1).unwrap();
        assert_eq!(entry.style.get().as_deref(), Some("heading-1"));
    }

    #[test]
    fn missing_node_errors() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| apply_style(
            &mut tr,
            NodeId::new(),
            "heading-1".into()
        ));
        assert!(matches!(err, CommandError::NodeNotFound(_)));
    }

    #[test]
    fn non_textblock_node_errors() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| apply_style(
            &mut tr,
            NodeId::ROOT,
            "heading-1".into()
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }
}
