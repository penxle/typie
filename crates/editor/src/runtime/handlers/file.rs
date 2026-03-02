use crate::model::{FileNode, Node, NodeId};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_file(&mut self, upload_id: Option<String>) -> Vec<Effect> {
        self.transact(|tr| {
            tr.insert_node(Node::File(FileNode {
                id: None,
                upload_id,
            }))
        })
    }

    pub(crate) fn handle_set_file_id(&mut self, node_id: String, file_id: String) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        self.transact(move |tr| {
            let Some(node_ref) = tr.node_mut(node_id) else {
                return Ok(false);
            };

            if !matches!(node_ref.node(), Some(Node::File(_))) {
                return Ok(false);
            }

            node_ref.as_mut().update(|node| {
                if let Node::File(file) = node {
                    file.id = Some(file_id);
                }
            })?;
            tr.mark_attr_mutation(node_id);
            Ok(true)
        })
    }
}
