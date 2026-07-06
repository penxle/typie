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

    tr.set_node(
        node_id,
        PlainNode::Callout(PlainCalloutNode {
            variant: next_variant,
        }),
    )?;
    Ok(true)
}
