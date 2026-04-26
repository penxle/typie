use editor_model::{Doc, Node, NodeId, NodeRef, TextNode};
use editor_state::State;

pub trait DocTestExt {
    fn doc_ref(&self) -> &Doc;

    fn node(&self, id: NodeId) -> NodeRef<'_> {
        self.doc_ref()
            .node(id)
            .unwrap_or_else(|| panic!("node {id:?} not found"))
    }

    fn has_node(&self, id: NodeId) -> bool {
        self.doc_ref().node(id).is_some()
    }

    fn text(&self, id: NodeId) -> &TextNode {
        let entry = self
            .doc_ref()
            .get_entry(id)
            .unwrap_or_else(|| panic!("node {id:?} not found"));
        match &entry.node {
            Node::Text(t) => t,
            other => panic!("expected Text node at {id:?}, got {other:?}"),
        }
    }
}

impl DocTestExt for State {
    fn doc_ref(&self) -> &Doc {
        &self.doc
    }
}

impl DocTestExt for Doc {
    fn doc_ref(&self) -> &Doc {
        self
    }
}

#[cfg(test)]
pub(crate) mod proptest;
