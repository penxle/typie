use editor_crdt::Dot;
use editor_model::Node;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult, unwrap_blockquote, unwrap_callout, unwrap_fold};

pub fn unwrap_node(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    let target = {
        let view = tr.view();
        let node = view
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        match node.node() {
            Node::Blockquote(_) => UnwrapTarget::Blockquote,
            Node::Callout(_) => UnwrapTarget::Callout,
            Node::Fold(_) => UnwrapTarget::Fold,
            _ => UnwrapTarget::Unsupported,
        }
    };

    match target {
        UnwrapTarget::Blockquote => unwrap_blockquote(tr, node_id),
        UnwrapTarget::Callout => unwrap_callout(tr, node_id),
        UnwrapTarget::Fold => unwrap_fold(tr, node_id),
        UnwrapTarget::Unsupported => Ok(false),
    }
}

enum UnwrapTarget {
    Blockquote,
    Callout,
    Fold,
    Unsupported,
}
