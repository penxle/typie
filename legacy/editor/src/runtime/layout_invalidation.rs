use crate::model::NodeId;
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutInvalidationOp {
    Full,
    NodeAndAncestors { node_id: NodeId },
    SubtreeAndAncestors { node_id: NodeId },
}

#[derive(Debug, Clone, Default)]
pub struct LayoutInvalidationBatch {
    full: bool,
    node_and_ancestors: FxHashSet<NodeId>,
    subtree_and_ancestors: FxHashSet<NodeId>,
}

impl LayoutInvalidationBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, op: LayoutInvalidationOp) {
        if self.full {
            return;
        }

        match op {
            LayoutInvalidationOp::Full => {
                self.promote_to_full();
            }
            LayoutInvalidationOp::NodeAndAncestors { node_id } => {
                if !self.subtree_and_ancestors.contains(&node_id) {
                    self.node_and_ancestors.insert(node_id);
                }
            }
            LayoutInvalidationOp::SubtreeAndAncestors { node_id } => {
                if node_id == NodeId::ROOT {
                    self.promote_to_full();
                    return;
                }
                self.node_and_ancestors.remove(&node_id);
                self.subtree_and_ancestors.insert(node_id);
            }
        }
    }

    pub fn is_full(&self) -> bool {
        self.full
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.node_and_ancestors.is_empty() && self.subtree_and_ancestors.is_empty()
    }

    pub fn node_and_ancestors_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.node_and_ancestors.iter().copied()
    }

    pub fn subtree_and_ancestors_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.subtree_and_ancestors.iter().copied()
    }

    fn promote_to_full(&mut self) {
        self.full = true;
        self.node_and_ancestors.clear();
        self.subtree_and_ancestors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_overrides_partials() {
        let mut batch = LayoutInvalidationBatch::new();
        let node = NodeId::new();

        batch.push(LayoutInvalidationOp::NodeAndAncestors { node_id: node });
        batch.push(LayoutInvalidationOp::SubtreeAndAncestors { node_id: node });
        batch.push(LayoutInvalidationOp::Full);

        assert!(batch.is_full());
        assert_eq!(batch.node_and_ancestors_ids().count(), 0);
        assert_eq!(batch.subtree_and_ancestors_ids().count(), 0);
    }

    #[test]
    fn subtree_root_escalates_to_full() {
        let mut batch = LayoutInvalidationBatch::new();

        batch.push(LayoutInvalidationOp::SubtreeAndAncestors {
            node_id: NodeId::ROOT,
        });

        assert!(batch.is_full());
    }

    #[test]
    fn subtree_overrides_node_for_same_id() {
        let mut batch = LayoutInvalidationBatch::new();
        let node = NodeId::new();

        batch.push(LayoutInvalidationOp::NodeAndAncestors { node_id: node });
        batch.push(LayoutInvalidationOp::SubtreeAndAncestors { node_id: node });

        let nodes: Vec<_> = batch.node_and_ancestors_ids().collect();
        let subtrees: Vec<_> = batch.subtree_and_ancestors_ids().collect();
        assert!(nodes.is_empty());
        assert_eq!(subtrees, vec![node]);
    }
}
