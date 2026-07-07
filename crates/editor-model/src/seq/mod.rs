use editor_crdt::Dot;

use crate::nodes::NodeType;

mod normalize;
mod project;
pub use normalize::*;
pub use project::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeqItem {
    Char(char),
    Atom(AtomLeaf),
    Block {
        node_type: NodeType,
        parents: Vec<Dot>,
        attrs: Vec<crate::NodeAttr>,
    },
    BlockAtom {
        leaf: AtomLeaf,
        parents: Vec<Dot>,
    },
    Unknown {
        tag: u64,
        bytes: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomLeaf {
    HardBreak,
    Tab,
    PageBreak,
    HorizontalRule {
        variant: crate::nodes::HorizontalRuleVariant,
    },
    Image {
        node: crate::nodes::ImageNode,
    },
    File {
        node: crate::nodes::FileNode,
    },
    Embed {
        node: crate::nodes::EmbedNode,
    },
    Archived {
        node: crate::nodes::ArchivedNode,
    },
    Unknown(crate::nodes::UnknownNode),
}

impl SeqItem {
    pub fn as_child_type(&self) -> Option<NodeType> {
        match self {
            SeqItem::Char(_) => Some(NodeType::Text),
            SeqItem::Atom(l) => Some(l.node_type()),
            SeqItem::Block { node_type, .. } => Some(*node_type),
            SeqItem::BlockAtom { leaf, .. } => Some(leaf.node_type()),
            SeqItem::Unknown { .. } => None,
        }
    }

    /// Whether this item is one of the three placeholder shapes for lossy
    /// unknown content: a classless inline `Unknown`, or an atom/block-atom
    /// carrying `AtomLeaf::Unknown`. `SeqItem::Block { node_type: NodeType::Unknown, .. }`
    /// is a fourth shape but is addressed by `Child`/block-tree walking, not
    /// per-item inspection, since it is a container rather than a leaf value.
    pub fn is_unknown_bearing(&self) -> bool {
        matches!(
            self,
            SeqItem::Unknown { .. }
                | SeqItem::Atom(AtomLeaf::Unknown(_))
                | SeqItem::BlockAtom {
                    leaf: AtomLeaf::Unknown(_),
                    ..
                }
        )
    }
}

impl AtomLeaf {
    pub fn node_type(&self) -> NodeType {
        match self {
            AtomLeaf::HardBreak => NodeType::HardBreak,
            AtomLeaf::Tab => NodeType::Tab,
            AtomLeaf::PageBreak => NodeType::PageBreak,
            AtomLeaf::HorizontalRule { .. } => NodeType::HorizontalRule,
            AtomLeaf::Image { .. } => NodeType::Image,
            AtomLeaf::File { .. } => NodeType::File,
            AtomLeaf::Embed { .. } => NodeType::Embed,
            AtomLeaf::Archived { .. } => NodeType::Archived,
            AtomLeaf::Unknown(_) => NodeType::Unknown,
        }
    }

    pub fn is_block_level(&self) -> bool {
        !self.node_type().spec().inline
    }

    pub fn node_type_set() -> [NodeType; 8] {
        [
            NodeType::HardBreak,
            NodeType::Tab,
            NodeType::PageBreak,
            NodeType::HorizontalRule,
            NodeType::Image,
            NodeType::File,
            NodeType::Embed,
            NodeType::Archived,
        ]
    }

    pub fn into_node(self) -> crate::Node {
        use crate::Node;
        use crate::nodes::{HardBreakNode, HorizontalRuleNode, PageBreakNode, TabNode};
        match self {
            AtomLeaf::HardBreak => Node::HardBreak(HardBreakNode {}),
            AtomLeaf::Tab => Node::Tab(TabNode {}),
            AtomLeaf::PageBreak => Node::PageBreak(PageBreakNode {}),
            AtomLeaf::HorizontalRule { variant } => Node::HorizontalRule(HorizontalRuleNode {
                variant: editor_crdt::LwwReg::with_value(variant),
            }),
            AtomLeaf::Image { node } => Node::Image(node),
            AtomLeaf::File { node } => Node::File(node),
            AtomLeaf::Embed { node } => Node::Embed(node),
            AtomLeaf::Archived { node } => Node::Archived(node),
            AtomLeaf::Unknown(node) => Node::Unknown(node),
        }
    }

    pub fn from_node(node: crate::Node) -> Option<Self> {
        use crate::Node;
        Some(match node {
            Node::HardBreak(_) => AtomLeaf::HardBreak,
            Node::Tab(_) => AtomLeaf::Tab,
            Node::PageBreak(_) => AtomLeaf::PageBreak,
            Node::HorizontalRule(n) => AtomLeaf::HorizontalRule {
                variant: *n.variant.get(),
            },
            Node::Image(n) => AtomLeaf::Image { node: n },
            Node::File(n) => AtomLeaf::File { node: n },
            Node::Embed(n) => AtomLeaf::Embed { node: n },
            Node::Archived(n) => AtomLeaf::Archived { node: n },
            Node::Unknown(n) => AtomLeaf::Unknown(n),
            _ => return None,
        })
    }

    /// Reconstructs an atom leaf from a plain node. Attr values are seeded onto a
    /// default live node with the global-minimum dot so any later edit supersedes
    /// them; the value lives inside the (synced) seq op payload, so every replica
    /// agrees on the baseline.
    pub fn from_plain_node(plain: &crate::PlainNode) -> Option<Self> {
        let node_type = plain.as_type();
        if classify(node_type) != SeqClass::Atom {
            return None;
        }
        let mut node = node_type.into_node();
        for attr in plain.to_attrs() {
            node.apply_attr(Dot::new(0, 0), &attr).ok()?;
        }
        Self::from_node(node)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqClass {
    Text,
    Atom,
    Block,
}

pub fn classify(t: NodeType) -> SeqClass {
    if !t.spec().is_leaf() {
        SeqClass::Block
    } else if t == NodeType::Text {
        SeqClass::Text
    } else {
        SeqClass::Atom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atom_leaf_from_plain_node_preserves_image_attrs() {
        use crate::nodes::{PlainImageNode, PlainNode};
        let plain = PlainNode::Image(PlainImageNode {
            id: Some("img-001".to_string()),
            proportion: 50,
        });
        let leaf = AtomLeaf::from_plain_node(&plain).expect("image converts");
        match &leaf {
            AtomLeaf::Image { node } => {
                assert_eq!(node.id.get(), &Some("img-001".to_string()));
                assert_eq!(*node.proportion.get(), 50);
            }
            other => panic!("expected Image, got {other:?}"),
        }
        assert_eq!(leaf.into_node().to_plain(), plain, "plain round-trip");
    }

    #[test]
    fn atom_leaf_unit_round_trips() {
        use crate::nodes::{PlainNode, PlainTabNode};
        let plain = PlainNode::Tab(PlainTabNode {});
        let leaf = AtomLeaf::from_plain_node(&plain).expect("tab converts");
        assert_eq!(leaf, AtomLeaf::Tab);
        assert_eq!(leaf.into_node().to_plain(), plain);
    }

    #[test]
    fn classify_matches_spec_and_inline_enum() {
        use strum::IntoEnumIterator;
        for t in NodeType::iter() {
            match classify(t) {
                SeqClass::Block => assert!(!t.spec().is_leaf(), "{t:?} block but leaf"),
                SeqClass::Text => assert_eq!(t, NodeType::Text),
                SeqClass::Atom => assert!(
                    AtomLeaf::node_type_set().contains(&t),
                    "{t:?} missing in AtomLeaf"
                ),
            }
        }
        for t in AtomLeaf::node_type_set() {
            assert!(matches!(classify(t), SeqClass::Atom), "{t:?}");
        }
    }

    #[test]
    fn is_block_level_matches_inline_flag() {
        use crate::nodes::{HorizontalRuleVariant, Node};
        assert!(!AtomLeaf::HardBreak.is_block_level());
        assert!(!AtomLeaf::Tab.is_block_level());
        assert!(!AtomLeaf::PageBreak.is_block_level());
        assert!(
            AtomLeaf::HorizontalRule {
                variant: HorizontalRuleVariant::default()
            }
            .is_block_level()
        );
        for ty in AtomLeaf::node_type_set() {
            let inline = ty.spec().inline;
            let a = match ty {
                NodeType::HardBreak => AtomLeaf::HardBreak,
                NodeType::Tab => AtomLeaf::Tab,
                NodeType::PageBreak => AtomLeaf::PageBreak,
                NodeType::HorizontalRule => AtomLeaf::HorizontalRule {
                    variant: HorizontalRuleVariant::default(),
                },
                NodeType::Image => {
                    let node = ty.into_node();
                    match node {
                        Node::Image(n) => AtomLeaf::Image { node: n },
                        _ => unreachable!(),
                    }
                }
                NodeType::File => {
                    let node = ty.into_node();
                    match node {
                        Node::File(n) => AtomLeaf::File { node: n },
                        _ => unreachable!(),
                    }
                }
                NodeType::Embed => {
                    let node = ty.into_node();
                    match node {
                        Node::Embed(n) => AtomLeaf::Embed { node: n },
                        _ => unreachable!(),
                    }
                }
                NodeType::Archived => {
                    let node = ty.into_node();
                    match node {
                        Node::Archived(n) => AtomLeaf::Archived { node: n },
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            };
            assert_eq!(a.is_block_level(), !inline, "{:?}", a.node_type());
        }
    }
}
