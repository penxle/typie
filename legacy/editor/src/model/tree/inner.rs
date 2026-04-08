use crate::model::NodeId;
use crate::schema::Schema;
use loro::{LoroDoc, LoroList, LoroMap};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct DocInner {
    pub loro: LoroDoc,
    pub schema: Rc<Schema>,
    children_cache: RefCell<FxHashMap<NodeId, Rc<Vec<NodeId>>>>,
    reachable: RefCell<Option<FxHashSet<NodeId>>>,
}

impl DocInner {
    pub fn new(loro: LoroDoc, schema: Rc<Schema>) -> Self {
        Self {
            loro,
            schema,
            children_cache: RefCell::new(FxHashMap::default()),
            reachable: RefCell::new(None),
        }
    }

    pub fn is_reachable(&self, node_id: NodeId) -> bool {
        let needs_build = self.reachable.borrow().is_none();
        if needs_build {
            self.build_reachable_set();
        }
        self.reachable.borrow().as_ref().unwrap().contains(&node_id)
    }

    fn build_reachable_set(&self) {
        let mut set = FxHashSet::default();
        let mut stack = vec![NodeId::ROOT];
        while let Some(id) = stack.pop() {
            if set.insert(id) {
                let children = self.get_children_ids_cached(id);
                stack.extend(children.iter().copied());
            }
        }
        *self.reachable.borrow_mut() = Some(set);
    }

    pub fn mark_reachable(&self, node_id: NodeId) {
        if let Some(set) = self.reachable.borrow_mut().as_mut() {
            set.insert(node_id);
        }
    }

    pub fn mark_unreachable_subtree(&self, node_id: NodeId) {
        if let Some(set) = self.reachable.borrow_mut().as_mut() {
            let mut stack = vec![node_id];
            while let Some(id) = stack.pop() {
                if set.remove(&id) {
                    let children = self.get_children_ids_cached(id);
                    stack.extend(children.iter().copied());
                }
            }
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

    pub fn create_node_map(&self, node_id: NodeId) -> Option<LoroMap> {
        let nodes = self.loro.get_map("nodes");
        let key = node_id.to_string();
        if nodes.get(&key).is_some() {
            let _ = nodes.delete(&key);
        }
        nodes.get_or_create_container(&key, LoroMap::new()).ok()
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

    pub fn is_ancestor_of(&self, ancestor_id: NodeId, node_id: NodeId) -> bool {
        let mut visited = FxHashSet::default();
        let mut current = node_id;
        while visited.insert(current) {
            let Some(map) = self.get_node_map(current) else {
                return false;
            };
            let parent_id = map
                .get("parent")
                .and_then(|v| v.into_value().ok())
                .and_then(|v| v.into_string().ok())
                .and_then(|s| NodeId::from_string(&s));
            match parent_id {
                Some(pid) if pid == ancestor_id => return true,
                Some(pid) => current = pid,
                None => return false,
            }
        }
        false
    }

    pub fn invalidate_children_cache_for(&self, node_id: NodeId) {
        self.children_cache.borrow_mut().remove(&node_id);
    }

    pub fn clear_children_cache(&self) {
        self.children_cache.borrow_mut().clear();
        *self.reachable.borrow_mut() = None;
    }
}
