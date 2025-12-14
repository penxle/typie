use crate::model::{Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn cycle_callout_type(&mut self, node_id: NodeId) -> Vec<Effect> {
        let Some(node_ref) = self.state.doc.node(node_id) else {
            return vec![];
        };

        let Node::Callout(callout) = node_ref.node() else {
            return vec![];
        };

        let next_type = callout.callout_type.next();

        if let Err(e) = node_ref.as_mut().update(|node| {
            if let Node::Callout(c) = node {
                c.callout_type = next_type;
            }
        }) {
            eprintln!("Failed to cycle callout type: {:?}", e);
            return vec![];
        }

        self.layout_cache.borrow_mut().invalidate_all();

        vec![Effect::LayoutChanged]
    }
}
