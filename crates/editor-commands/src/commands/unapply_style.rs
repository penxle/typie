use editor_model::NodeId;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn unapply_style(tr: &mut Transaction, node_id: NodeId, style_id: String) -> CommandResult {
    let entry = tr
        .state()
        .doc
        .get_entry(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;
    if entry.style.get().as_deref() != Some(style_id.as_str()) {
        return Ok(false);
    }
    tr.set_node_style(node_id, None)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::commands::apply_style;
    use crate::test_utils::*;

    #[test]
    fn removes_applied_style() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (after_apply, ..) =
            transact!(initial, |tr| apply_style(&mut tr, p1, "heading-1".into()));
        let (after_unapply, ..) = transact!(after_apply, |tr| unapply_style(
            &mut tr,
            p1,
            "heading-1".into()
        ));
        let entry = after_unapply.doc.get_entry(p1).unwrap();
        assert!(entry.style.get().is_none());
    }

    #[test]
    fn unapply_unapplied_style_is_noop() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) =
            transact_fail!(initial, |tr| unapply_style(&mut tr, p1, "heading-1".into()));
        let entry = actual.doc.get_entry(p1).unwrap();
        assert!(entry.style.get().is_none());
    }
}
