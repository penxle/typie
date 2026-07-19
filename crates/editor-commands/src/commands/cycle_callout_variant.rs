use editor_crdt::Dot;
use editor_model::{Node, PlainCalloutNode, PlainNode};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn cycle_callout_variant(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    let next_variant = {
        let view = tr.view();
        let node = view
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        let Node::Callout(callout) = node.node() else {
            return Ok(false);
        };
        callout.variant.get().next()
    };

    // A synthetic callout scaffold has no op dot; materialize it before the SetNode
    // op so the type change is not silently dropped.
    let node_id = if node_id.is_synthetic() {
        editor_transaction::materialize_repair_target(tr, node_id)?
    } else {
        node_id
    };

    tr.set_node(
        node_id,
        PlainNode::Callout(PlainCalloutNode {
            variant: next_variant,
        }),
    )?;
    Ok(true)
}
