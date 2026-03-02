use crate::model::{ImageNode, Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_image(&mut self, upload_id: Option<String>) -> Vec<Effect> {
        self.transact(|tr| {
            tr.insert_node(Node::Image(ImageNode {
                id: None,
                proportion: 1.0,
                upload_id,
            }))
        })
    }

    pub(crate) fn handle_set_image_proportion(
        &mut self,
        node_id: String,
        proportion: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        let proportion = proportion.clamp(0.1, 1.0);

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Some(Node::Image(_))) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::Image(image) = node {
                    image.proportion = proportion;
                }
            })?;

            tr.mark_attr_mutation(node_id);
            Ok(true)
        })
    }

    pub(crate) fn handle_set_image_id(&mut self, node_id: String, image_id: String) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Some(Node::Image(_))) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::Image(image) = node {
                    image.id = Some(image_id);
                }
            })?;
            tr.mark_attr_mutation(node_id);
            Ok(true)
        })
    }

    pub(crate) fn handle_set_external_element_height(
        &mut self,
        node_id: String,
        height: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        let current_height = self.layout_engine.external_height(node_id);

        if current_height == Some(height) {
            return vec![];
        }

        self.layout_engine.set_external_height(node_id, height);

        vec![Effect::NodeMutated {
            node_id,
            kind: crate::runtime::MutationKind::ViewState,
        }]
    }
}
