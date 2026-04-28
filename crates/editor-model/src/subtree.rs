use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::doc::Doc;
use crate::entry::NodeEntry;
use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::Node;

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subtree {
    pub id: NodeId,
    pub node: Node,
    pub modifiers: Vec<Modifier>,
    pub children: Vec<Subtree>,
}

impl Subtree {
    pub fn leaf(id: NodeId, node: Node) -> Self {
        Self {
            id,
            node,
            modifiers: vec![],
            children: vec![],
        }
    }

    pub fn with_children(mut self, children: Vec<Subtree>) -> Self {
        self.children = children;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Vec<Modifier>) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn into_entries(self, parent_id: NodeId) -> Vec<(NodeId, NodeEntry)> {
        let mut entries = Vec::new();
        self.collect_entries(parent_id, &mut entries);
        entries
    }

    fn collect_entries(self, parent_id: NodeId, entries: &mut Vec<(NodeId, NodeEntry)>) {
        let child_ids: imbl::Vector<NodeId> = self.children.iter().map(|c| c.id).collect();
        let entry = NodeEntry {
            node: self.node,
            parent: Some(parent_id),
            children: child_ids,
            modifiers: self.modifiers,
        };
        let self_id = self.id;
        entries.push((self_id, entry));
        for child in self.children {
            child.collect_entries(self_id, entries);
        }
    }

    pub fn capture(doc: &Doc, node_id: NodeId) -> Option<Self> {
        let entry = doc.get_entry(node_id)?;
        let children = entry
            .children
            .iter()
            .filter_map(|&child_id| Self::capture(doc, child_id))
            .collect();
        Some(Self {
            id: node_id,
            node: entry.node.clone(),
            modifiers: entry.modifiers.clone(),
            children,
        })
    }

    pub fn contains_node(&self, id: NodeId) -> bool {
        if self.id == id {
            return true;
        }
        self.children.iter().any(|c| c.contains_node(id))
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::nodes::*;

    #[test]
    fn leaf_creates_childless_subtree() {
        let id = NodeId::new();
        let tree = Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()));
        assert_eq!(tree.id, id);
        assert!(tree.children.is_empty());
        assert!(tree.modifiers.is_empty());
    }

    #[test]
    fn with_children_builds_nested_subtree() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();
        let tree =
            Subtree::leaf(parent_id, Node::BulletList(BulletListNode {})).with_children(vec![
                Subtree::leaf(child_id, Node::ListItem(ListItemNode {})),
            ]);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id, child_id);
    }

    #[test]
    fn into_entries_produces_parent_first_order() {
        let root_id = NodeId::new();
        let child_id = NodeId::new();
        let grandchild_id = NodeId::new();
        let tree = Subtree::leaf(root_id, Node::BulletList(BulletListNode {})).with_children(vec![
            Subtree::leaf(child_id, Node::ListItem(ListItemNode {})).with_children(vec![
                Subtree::leaf(grandchild_id, Node::Paragraph(ParagraphNode::default())),
            ]),
        ]);

        let insertion_parent = NodeId::new();
        let entries = tree.into_entries(insertion_parent);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, root_id);
        assert_eq!(entries[0].1.parent, Some(insertion_parent));
        assert_eq!(entries[0].1.children.len(), 1);
        assert_eq!(entries[1].0, child_id);
        assert_eq!(entries[1].1.parent, Some(root_id));
        assert_eq!(entries[2].0, grandchild_id);
        assert_eq!(entries[2].1.parent, Some(child_id));
    }

    #[test]
    fn capture_builds_subtree_from_doc() {
        let (doc, p1, t1, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("Hi")
                }
            }
        };

        let tree = Subtree::capture(&doc, p1).unwrap();
        assert_eq!(tree.id, p1);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id, t1);
    }

    #[test]
    fn subtree_serde_roundtrip() {
        let id = NodeId::new();
        let tree = Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()));
        let json = serde_json::to_string(&tree).unwrap();
        let back: Subtree = serde_json::from_str(&json).unwrap();
        assert_eq!(tree, back);
    }

    #[test]
    fn contains_node_finds_self() {
        let id = NodeId::new();
        let tree = Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()));
        assert!(tree.contains_node(id));
    }

    #[test]
    fn contains_node_finds_descendant() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();
        let tree =
            Subtree::leaf(parent_id, Node::BulletList(BulletListNode {})).with_children(vec![
                Subtree::leaf(child_id, Node::ListItem(ListItemNode {})),
            ]);
        assert!(tree.contains_node(parent_id));
        assert!(tree.contains_node(child_id));
    }

    #[test]
    fn contains_node_misses_unrelated() {
        let id = NodeId::new();
        let other = NodeId::new();
        let tree = Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()));
        assert!(!tree.contains_node(other));
    }
}
