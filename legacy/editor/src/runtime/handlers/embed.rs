use crate::model::{EmbedNode, Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub fn handle_insert_embed(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.insert_node(Node::Embed(EmbedNode { id: None })))
    }

    pub fn handle_set_embed_id(&mut self, node_id: String, embed_id: String) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Some(Node::Embed(_))) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::Embed(embed) = node {
                    embed.id = Some(embed_id);
                }
            })?;
            tr.mark_attr_mutation(node_id);
            Ok(true)
        })
    }
}
