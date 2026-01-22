use crate::model::{FileNode, Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_file(&mut self, upload_id: Option<String>) -> Vec<Effect> {
        self.transact(|tr| {
            tr.insert_node(Node::File(FileNode {
                name: None,
                size: None,
                src: None,
                upload_id,
            }))
        })
    }

    pub(crate) fn handle_set_file_src(
        &mut self,
        node_id: String,
        src: String,
        name: String,
        size: u64,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Node::File(_)) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::File(file) = node {
                    file.src = Some(src);
                    file.name = Some(name);
                    file.size = Some(size);
                }
            })?;
            tr.push_effect(Effect::NodeChanged { node_id });
            Ok(true)
        })
    }
}
