use crate::model::{ImageNode, Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_image(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            tr.insert_node(Node::Image(ImageNode {
                src: None,
                width: None,
                height: None,
                proportion: 1.0,
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

            if !matches!(node_ref.node(), Node::Image(_)) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::Image(image) = node {
                    image.proportion = proportion;
                }
            })?;

            tr.push_effect(Effect::NodeChanged { node_id });
            Ok(true)
        })
    }

    pub(crate) fn handle_set_image_dimensions(
        &mut self,
        node_id: String,
        width: f32,
        height: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Node::Image(_)) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::Image(image) = node {
                    image.width = Some(width);
                    image.height = Some(height);
                    image.proportion = 1.0;
                }
            })?;
            tr.push_effect(Effect::NodeChanged { node_id });
            Ok(true)
        })
    }

    pub(crate) fn handle_set_image_src(
        &mut self,
        node_id: String,
        src: String,
        width: f32,
        height: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Node::Image(_)) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::Image(image) = node {
                    image.src = Some(src);
                    image.width = Some(width);
                    image.height = Some(height);
                    image.proportion = 1.0;
                }
            })?;
            tr.push_effect(Effect::NodeChanged { node_id });
            Ok(true)
        })
    }
}
