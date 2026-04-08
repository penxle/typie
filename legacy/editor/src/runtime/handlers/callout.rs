use crate::model::{Node, NodeId};
use crate::runtime::{Effect, Runtime};
use crate::state::ancestor_helpers::lowest_common_ancestor_id;

impl Runtime {
    pub(crate) fn cycle_callout_variant(&mut self, node_id: NodeId) -> Vec<Effect> {
        let Some(node_ref) = self.state.doc.node(node_id) else {
            return vec![];
        };

        let Some(Node::Callout(callout)) = node_ref.node() else {
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

        vec![Effect::NodeMutated {
            node_id,
            kind: crate::runtime::MutationKind::Attr,
        }]
    }

    pub(crate) fn handle_cycle_callout_variant_at(&mut self, node_id: String) -> Vec<Effect> {
        let Ok(node_id) = node_id.parse::<NodeId>() else {
            return vec![];
        };
        self.cycle_callout_variant(node_id)
    }

    pub(crate) fn handle_cycle_callout_variant_in_selection(&mut self) -> Vec<Effect> {
        let selection = self.state.selection;
        let Some(lca_id) =
            lowest_common_ancestor_id(self.doc(), selection.anchor.node_id, selection.head.node_id)
        else {
            return vec![];
        };

        let mut current = Some(lca_id);
        while let Some(node_id) = current {
            let Some(node) = self.state.doc.node(node_id) else {
                break;
            };

            if matches!(node.node(), Some(Node::Callout(_))) {
                return self.cycle_callout_variant(node_id);
            }

            current = node.parent().map(|parent| parent.node_id());
        }

        vec![]
    }
}
