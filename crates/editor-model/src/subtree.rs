use crate::doc::Doc;
use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::PlainNode;

#[derive(Clone, Debug, PartialEq)]
pub struct Subtree {
    pub id: NodeId,
    pub node: PlainNode,
    pub modifiers: Vec<Modifier>,
    pub style: Option<String>,
    pub children: Vec<Subtree>,
}

impl Subtree {
    pub fn leaf(id: NodeId, node: PlainNode) -> Self {
        Self {
            id,
            node,
            modifiers: vec![],
            style: None,
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

    pub fn capture(doc: &Doc, node_id: NodeId) -> Option<Self> {
        let entry = doc.get_entry(node_id)?;
        let children = entry
            .children
            .iter()
            .copied()
            .filter_map(|child_id| Self::capture(doc, child_id))
            .collect();
        let modifiers: Vec<Modifier> = entry.modifiers.iter().map(|(_, v)| v.clone()).collect();
        let style = entry.style.get().clone();
        Some(Self {
            id: node_id,
            node: entry.node.to_plain(),
            modifiers,
            style,
            children,
        })
    }

    pub fn contains_node(&self, id: NodeId) -> bool {
        if self.id == id {
            return true;
        }
        self.children.iter().any(|c| c.contains_node(id))
    }

    pub fn all_ids(&self) -> Vec<NodeId> {
        let mut ids = Vec::new();
        self.collect_ids(&mut ids);
        ids
    }

    fn collect_ids(&self, ids: &mut Vec<NodeId>) {
        ids.push(self.id);
        for child in &self.children {
            child.collect_ids(ids);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::*;

    #[test]
    fn leaf_creates_childless_subtree() {
        let id = NodeId::new();
        let tree = Subtree::leaf(id, PlainNode::Paragraph(PlainParagraphNode::default()));
        assert_eq!(tree.id, id);
        assert!(tree.children.is_empty());
        assert!(tree.modifiers.is_empty());
    }

    #[test]
    fn with_children_builds_nested_subtree() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();
        let tree = Subtree::leaf(parent_id, PlainNode::BulletList(PlainBulletListNode {}))
            .with_children(vec![Subtree::leaf(
                child_id,
                PlainNode::ListItem(PlainListItemNode {}),
            )]);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id, child_id);
    }

    #[test]
    fn capture_builds_subtree_from_doc() {
        use editor_macros::doc;

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
    fn contains_node_finds_self() {
        let id = NodeId::new();
        let tree = Subtree::leaf(id, PlainNode::Paragraph(PlainParagraphNode::default()));
        assert!(tree.contains_node(id));
    }

    #[test]
    fn contains_node_finds_descendant() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();
        let tree = Subtree::leaf(parent_id, PlainNode::BulletList(PlainBulletListNode {}))
            .with_children(vec![Subtree::leaf(
                child_id,
                PlainNode::ListItem(PlainListItemNode {}),
            )]);
        assert!(tree.contains_node(parent_id));
        assert!(tree.contains_node(child_id));
    }

    #[test]
    fn contains_node_misses_unrelated() {
        let id = NodeId::new();
        let other = NodeId::new();
        let tree = Subtree::leaf(id, PlainNode::Paragraph(PlainParagraphNode::default()));
        assert!(!tree.contains_node(other));
    }

    #[test]
    fn all_ids_collects_all_nodes() {
        let root_id = NodeId::new();
        let child_id = NodeId::new();
        let grandchild_id = NodeId::new();
        let tree = Subtree::leaf(root_id, PlainNode::BulletList(PlainBulletListNode {}))
            .with_children(vec![
                Subtree::leaf(child_id, PlainNode::ListItem(PlainListItemNode {})).with_children(
                    vec![Subtree::leaf(
                        grandchild_id,
                        PlainNode::Paragraph(PlainParagraphNode::default()),
                    )],
                ),
            ]);
        let ids = tree.all_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&root_id));
        assert!(ids.contains(&child_id));
        assert!(ids.contains(&grandchild_id));
    }
}
