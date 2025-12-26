use crate::model::NodeId;
use crate::schema::Schema;
use loro::{LoroDoc, LoroList, LoroMap};
use std::rc::Rc;

#[derive(Debug)]
pub struct DocInner {
    pub loro: LoroDoc,
    pub schema: Rc<Schema>,
}

impl DocInner {
    pub fn new(loro: LoroDoc, schema: Rc<Schema>) -> Self {
        Self { loro, schema }
    }

    pub fn fork(&self) -> Self {
        Self {
            loro: self.loro.fork(),
            schema: self.schema.clone(),
        }
    }

    pub fn get_node_map(&self, node_id: NodeId) -> Option<LoroMap> {
        let nodes = self.loro.get_map("nodes");
        nodes
            .get(&node_id.to_string())
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
    }

    pub fn get_or_create_node_map(&self, node_id: NodeId) -> Option<LoroMap> {
        let nodes = self.loro.get_map("nodes");
        nodes
            .get_or_create_container(&node_id.to_string(), LoroMap::new())
            .ok()
    }

    pub fn get_children_list(&self, node_id: NodeId) -> Option<LoroList> {
        self.get_node_map(node_id)?
            .get("children")
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_list().ok())
    }

    pub fn get_or_create_children_list(&self, node_id: NodeId) -> Option<LoroList> {
        self.get_or_create_node_map(node_id)?
            .get_or_create_container("children", LoroList::new())
            .ok()
    }
}
