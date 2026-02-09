use crate::model::NodeId;
use crate::schema::Schema;
use loro::{LoroDoc, LoroList, LoroMap};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct DocInner {
    pub loro: LoroDoc,
    pub schema: Rc<Schema>,
    children_cache: RefCell<FxHashMap<NodeId, Rc<Vec<NodeId>>>>,
}

impl DocInner {
    pub fn new(loro: LoroDoc, schema: Rc<Schema>) -> Self {
        Self {
            loro,
            schema,
            children_cache: RefCell::new(FxHashMap::default()),
        }
    }

    pub fn fork(&self) -> Self {
        Self {
            loro: self.loro.fork(),
            schema: self.schema.clone(),
            children_cache: RefCell::new(FxHashMap::default()),
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

    pub fn get_children_ids_cached(&self, node_id: NodeId) -> Rc<Vec<NodeId>> {
        if let Some(cached) = self.children_cache.borrow().get(&node_id) {
            return Rc::clone(cached);
        }
        let ids = Rc::new(self.build_children_ids(node_id));
        self.children_cache
            .borrow_mut()
            .insert(node_id, Rc::clone(&ids));
        ids
    }

    fn build_children_ids(&self, node_id: NodeId) -> Vec<NodeId> {
        let Some(children) = self.get_children_list(node_id) else {
            return Vec::new();
        };
        if let loro::LoroValue::List(values) = children.get_value() {
            let mut ids = Vec::with_capacity(values.len());
            for value in values.iter() {
                if let loro::LoroValue::String(s) = value {
                    if let Some(id) = NodeId::from_string(s) {
                        ids.push(id);
                    }
                }
            }
            ids
        } else {
            Vec::new()
        }
    }

    pub fn invalidate_children_cache_for(&self, node_id: NodeId) {
        self.children_cache.borrow_mut().remove(&node_id);
    }

    pub fn clear_children_cache(&self) {
        self.children_cache.borrow_mut().clear();
    }
}
