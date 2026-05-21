use std::fmt;

use crate::doc::Doc;
use crate::entry::NodeEntry;
use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::{Node, NodeType};

#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    doc: &'a Doc,
    id: NodeId,
}

impl<'a> NodeRef<'a> {
    pub fn new(doc: &'a Doc, id: NodeId) -> Self {
        Self { doc, id }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn entry(&self) -> &'a NodeEntry {
        self.doc
            .get_entry(self.id)
            .expect("NodeRef: node must exist")
    }

    pub fn node(&self) -> &'a Node {
        &self.entry().node
    }

    pub fn as_type(&self) -> NodeType {
        self.node().as_type()
    }

    pub fn modifiers(&self) -> impl Iterator<Item = &'a Modifier> + 'a {
        self.entry()
            .modifiers
            .iter()
            .map(|(_, v)| v)
            .chain(self.node().implicit_modifiers().iter())
    }

    // Persisted modifiers only, excluding the node type's implicit ones. Use
    // where virtual modifiers must not be treated as real: capturing a subtree
    // for the undo log, and deciding which modifiers to remove/replace.
    pub fn explicit_modifiers(&self) -> impl Iterator<Item = &'a Modifier> + 'a {
        self.entry().modifiers.iter().map(|(_, v)| v)
    }

    pub fn parent(&self) -> Option<NodeRef<'a>> {
        let parent_id = (*self.entry().parent.get())?;
        self.doc.node(parent_id)
    }

    pub fn children(&self) -> impl Iterator<Item = NodeRef<'a>> + 'a {
        let doc = self.doc;
        self.entry()
            .children
            .iter()
            .copied()
            .map(move |id| NodeRef::new(doc, id))
    }

    pub fn first_child(&self) -> Option<NodeRef<'a>> {
        let id = self.entry().children.iter().next().copied()?;
        Some(NodeRef::new(self.doc, id))
    }

    pub fn last_child(&self) -> Option<NodeRef<'a>> {
        let id = self.entry().children.iter().last().copied()?;
        Some(NodeRef::new(self.doc, id))
    }

    pub fn prev_sibling(&self) -> Option<NodeRef<'a>> {
        let parent = self.parent()?;
        let children: Vec<NodeId> = parent.entry().children.iter().copied().collect();
        let idx = children.iter().position(|&id| id == self.id)?;
        let prev_id = *children.get(idx.checked_sub(1)?)?;
        Some(NodeRef::new(self.doc, prev_id))
    }

    pub fn next_sibling(&self) -> Option<NodeRef<'a>> {
        let parent = self.parent()?;
        let children: Vec<NodeId> = parent.entry().children.iter().copied().collect();
        let idx = children.iter().position(|&id| id == self.id)?;
        let next_id = *children.get(idx + 1)?;
        Some(NodeRef::new(self.doc, next_id))
    }

    pub fn ancestors(&self) -> AncestorIter<'a> {
        AncestorIter {
            doc: self.doc,
            current: Some(self.id),
        }
    }

    pub fn descendants(&self) -> DescendantIter<'a> {
        let stack: Vec<NodeId> = self.entry().children.iter().copied().collect();
        let stack = stack.into_iter().rev().collect();
        DescendantIter {
            doc: self.doc,
            stack,
        }
    }

    pub fn index(&self) -> Option<usize> {
        let parent = self.parent()?;
        parent
            .entry()
            .children
            .iter()
            .copied()
            .position(|id| id == self.id)
    }

    pub fn path(&self) -> Vec<usize> {
        let mut path = Vec::new();
        let mut current = self.id;
        while let Some(node_ref) = self.doc.node(current) {
            if let Some(idx) = node_ref.index() {
                path.push(idx);
            }
            match *node_ref.entry().parent.get() {
                Some(parent_id) => current = parent_id,
                None => break,
            }
        }
        path.reverse();
        path
    }

    pub fn depth(&self) -> usize {
        let mut d = 0;
        let mut current = *self.entry().parent.get();
        while let Some(parent_id) = current {
            d += 1;
            current = self.doc.get_entry(parent_id).and_then(|e| *e.parent.get());
        }
        d
    }
}

impl fmt::Debug for NodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry = self.entry();
        let alternate = f.alternate();
        let mut s = f.debug_struct("NodeRef");
        s.field("id", &self.id);
        s.field("node", &entry.node);
        s.field(
            "modifiers",
            &entry.modifiers.iter().map(|(_, v)| v).collect::<Vec<_>>(),
        );
        if alternate {
            s.field("parent", &entry.parent.get());
            s.field(
                "children",
                &entry.children.iter().copied().collect::<Vec<_>>(),
            );
            s.field("depth", &self.depth());
            s.field("index", &self.index());
        }
        s.finish()
    }
}

pub struct AncestorIter<'a> {
    doc: &'a Doc,
    current: Option<NodeId>,
}

impl<'a> Iterator for AncestorIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current?;
        let entry = self.doc.get_entry(id)?;
        self.current = *entry.parent.get();
        Some(NodeRef::new(self.doc, id))
    }
}

pub struct DescendantIter<'a> {
    doc: &'a Doc,
    stack: Vec<NodeId>,
}

impl<'a> Iterator for DescendantIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        if let Some(entry) = self.doc.get_entry(id) {
            let children: Vec<NodeId> = entry.children.iter().copied().collect();
            for child_id in children.into_iter().rev() {
                self.stack.push(child_id);
            }
        }
        Some(NodeRef::new(self.doc, id))
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use crate::*;

    /// Build:
    /// Root
    ///   ├── P1 (Paragraph)
    ///   │   ├── T1 (Text "Hello")
    ///   │   └── T2 (Text "World")
    ///   └── P2 (Paragraph)
    ///       └── T3 (Text "!")
    fn make_doc() -> (Doc, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let (doc, p1, t1, t2, p2, t3, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("Hello")
                    t2: text("World")
                }
                p2: paragraph {
                    t3: text("!")
                }
            }
        };
        (doc, p1, p2, t1, t2, t3)
    }

    #[test]
    fn parent() {
        let (doc, p1, _, t1, _, _) = make_doc();
        let t1_ref = doc.node(t1).unwrap();
        let parent = t1_ref.parent().unwrap();
        assert_eq!(parent.id(), p1);
    }

    #[test]
    fn root_has_no_parent() {
        let (doc, _, _, _, _, _) = make_doc();
        let root = doc.root().unwrap();
        assert!(root.parent().is_none());
    }

    #[test]
    fn children_count() {
        let (doc, p1, _, _, _, _) = make_doc();
        let p1_ref = doc.node(p1).unwrap();
        assert_eq!(p1_ref.children().count(), 2);
    }

    #[test]
    fn first_last_child() {
        let (doc, p1, _, t1, t2, _) = make_doc();
        let p1_ref = doc.node(p1).unwrap();
        assert_eq!(p1_ref.first_child().unwrap().id(), t1);
        assert_eq!(p1_ref.last_child().unwrap().id(), t2);
    }

    #[test]
    fn siblings() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let t1_ref = doc.node(t1).unwrap();
        assert!(t1_ref.prev_sibling().is_none());
        assert_eq!(t1_ref.next_sibling().unwrap().id(), t2);

        let t2_ref = doc.node(t2).unwrap();
        assert_eq!(t2_ref.prev_sibling().unwrap().id(), t1);
        assert!(t2_ref.next_sibling().is_none());
    }

    #[test]
    fn ancestors() {
        let (doc, p1, _, t1, _, _) = make_doc();
        let t1_ref = doc.node(t1).unwrap();
        let ancestor_ids: Vec<NodeId> = t1_ref.ancestors().map(|n| n.id()).collect();
        assert_eq!(ancestor_ids, vec![t1, p1, NodeId::ROOT]);
    }

    #[test]
    fn descendants() {
        let (doc, p1, p2, t1, t2, t3) = make_doc();
        let root = doc.root().unwrap();
        let desc_ids: Vec<NodeId> = root.descendants().map(|n| n.id()).collect();
        assert_eq!(desc_ids, vec![p1, t1, t2, p2, t3]);
    }

    #[test]
    fn index() {
        let (doc, _, _, t1, t2, _) = make_doc();
        assert_eq!(doc.node(t1).unwrap().index(), Some(0));
        assert_eq!(doc.node(t2).unwrap().index(), Some(1));
    }

    #[test]
    fn depth() {
        let (doc, _, _, t1, _, _) = make_doc();
        assert_eq!(doc.root().unwrap().depth(), 0);
        assert_eq!(doc.node(t1).unwrap().depth(), 2);
    }

    #[test]
    fn path_of_root() {
        let (doc, _, _, _, _, _) = make_doc();
        assert_eq!(doc.root().unwrap().path(), Vec::<usize>::new());
    }

    #[test]
    fn path_of_first_paragraph() {
        let (doc, p1, _, _, _, _) = make_doc();
        assert_eq!(doc.node(p1).unwrap().path(), vec![0]);
    }

    #[test]
    fn path_of_second_paragraph() {
        let (doc, _, p2, _, _, _) = make_doc();
        assert_eq!(doc.node(p2).unwrap().path(), vec![1]);
    }

    #[test]
    fn path_of_nested_text() {
        let (doc, _, _, t1, t2, t3) = make_doc();
        assert_eq!(doc.node(t1).unwrap().path(), vec![0, 0]);
        assert_eq!(doc.node(t2).unwrap().path(), vec![0, 1]);
        assert_eq!(doc.node(t3).unwrap().path(), vec![1, 0]);
    }
}
