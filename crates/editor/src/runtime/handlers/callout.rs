use crate::model::{Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn cycle_callout_variant(&mut self, node_id: NodeId) -> Vec<Effect> {
        let Some(node_ref) = self.state.doc.node(node_id) else {
            return vec![];
        };

        let Node::Callout(callout) = node_ref.node() else {
            return vec![];
        };

        let next_type = callout.variant.next();

        if let Err(e) = node_ref.as_mut().update(|node| {
            if let Node::Callout(c) = node {
                c.variant = next_type;
            }
        }) {
            eprintln!("Failed to cycle callout variant: {:?}", e);
            return vec![];
        }

        self.layout_cache.borrow_mut().invalidate_all();

        vec![Effect::LayoutChanged]
    }
}
