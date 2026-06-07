use editor_crdt::{LwwReg, OrMap, Rga};

use crate::id::NodeId;
use crate::marker::Marker;
use crate::modifier::{Modifier, ModifierType};
use crate::nodes::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeEntry {
    pub parent: LwwReg<Option<NodeId>>,
    pub children: Rga<NodeId>,
    pub modifiers: OrMap<ModifierType, Modifier>,
    pub style: LwwReg<Option<String>>,
    pub marker: LwwReg<Option<Marker>>,
    pub node: Node,
}

impl NodeEntry {
    pub fn new(node: Node) -> Self {
        Self {
            parent: LwwReg::with_value(None),
            children: Rga::new(),
            modifiers: OrMap::new(),
            style: LwwReg::with_value(None),
            marker: LwwReg::with_value(None),
            node,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::ParagraphNode;

    #[test]
    fn new_yields_default_wrappers() {
        let entry = NodeEntry::new(Node::Paragraph(ParagraphNode::default()));
        assert!(entry.parent.get().is_none());
        assert!(entry.children.is_empty());
        assert!(entry.modifiers.is_empty());
        assert!(entry.style.get().is_none());
    }

    #[test]
    fn new_node_marker_is_none() {
        let entry = NodeEntry::new(Node::Paragraph(ParagraphNode::default()));
        assert!(entry.marker.get().is_none());
    }
}
