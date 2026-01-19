use crate::model::{ImageNode, Node, NodeId};
use crate::runtime::{Effect, NodeViewState, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_image(&mut self, upload_id: Option<String>) -> Vec<Effect> {
        self.transact(|tr| {
            tr.insert_node(Node::Image(ImageNode {
                src: None,
                width: None,
                height: None,
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

    pub(crate) fn handle_set_image_dimensions_ephemeral(
        &mut self,
        node_id: String,
        width: f32,
        height: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.view_states
            .insert(node_id, NodeViewState::Image { width, height });

        if let Some(node) = self.doc().node(node_id) {
            let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
            self.layout_cache
                .borrow_mut()
                .invalidate_with_ancestors(node_id, ancestors.into_iter());
        } else {
            self.layout_cache.borrow_mut().invalidate(node_id);
        }

        vec![Effect::LayoutChanged]
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

        self.view_states.remove(&node_id);

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
