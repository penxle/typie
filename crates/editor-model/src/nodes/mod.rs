mod archived;
mod blockquote;
mod bullet_list;
mod callout;
mod embed;
mod file;
mod fold;
mod fold_content;
mod fold_title;
mod hard_break;
mod horizontal_rule;
mod image;
mod list_item;
mod ordered_list;
mod page_break;
mod paragraph;
mod root;
mod table;
mod table_cell;
mod table_row;
mod text;

pub use archived::*;
pub use blockquote::*;
pub use bullet_list::*;
pub use callout::*;
pub use embed::*;
pub use file::*;
pub use fold::*;
pub use fold_content::*;
pub use fold_title::*;
pub use hard_break::*;
pub use horizontal_rule::*;
pub use image::*;
pub use list_item::*;
pub use ordered_list::*;
pub use page_break::*;
pub use paragraph::*;
pub use root::*;
pub use table::*;
pub use table_cell::*;
pub use table_row::*;
pub use text::*;

use crate::ModelError;
use editor_macros::{FromDiscriminant, NodeCompanion, ffi};
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoStaticStr};

#[derive(Debug, Clone, PartialEq, EnumDiscriminants, FromDiscriminant, NodeCompanion)]
#[strum_discriminants(name(NodeType))]
#[strum_discriminants(ffi)]
#[strum_discriminants(derive(
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
    Enum,
    IntoStaticStr,
    minicbor::Encode,
    minicbor::Decode,
))]
#[strum_discriminants(cbor(index_only))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[from_discriminant(NodeType)]
pub enum Node {
    #[strum_discriminants(n(0))]
    Root(RootNode),
    #[strum_discriminants(n(1))]
    Paragraph(ParagraphNode),
    #[strum_discriminants(n(2))]
    Blockquote(BlockquoteNode),
    #[strum_discriminants(n(3))]
    Callout(CalloutNode),
    #[strum_discriminants(n(4))]
    Text(TextNode),
    #[strum_discriminants(n(5))]
    BulletList(BulletListNode),
    #[strum_discriminants(n(6))]
    OrderedList(OrderedListNode),
    #[strum_discriminants(n(7))]
    ListItem(ListItemNode),
    #[strum_discriminants(n(8))]
    Fold(FoldNode),
    #[strum_discriminants(n(9))]
    FoldTitle(FoldTitleNode),
    #[strum_discriminants(n(10))]
    FoldContent(FoldContentNode),
    #[strum_discriminants(n(11))]
    Table(TableNode),
    #[strum_discriminants(n(12))]
    TableRow(TableRowNode),
    #[strum_discriminants(n(13))]
    TableCell(TableCellNode),
    #[strum_discriminants(n(14))]
    Image(ImageNode),
    #[strum_discriminants(n(15))]
    File(FileNode),
    #[strum_discriminants(n(16))]
    Embed(EmbedNode),
    #[strum_discriminants(n(17))]
    Archived(ArchivedNode),
    #[strum_discriminants(n(18))]
    HardBreak(HardBreakNode),
    #[strum_discriminants(n(19))]
    HorizontalRule(HorizontalRuleNode),
    #[strum_discriminants(n(20))]
    PageBreak(PageBreakNode),
}

impl Node {
    pub fn as_type(&self) -> NodeType {
        NodeType::from(self)
    }
}

impl NodeType {
    pub fn into_node(self) -> Node {
        Node::from_discriminant(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::Dot;

    #[test]
    fn node_type_roundtrip() {
        let node = Node::Paragraph(ParagraphNode::default());
        assert_eq!(node.as_type(), NodeType::Paragraph);
    }

    #[test]
    fn from_discriminant_creates_default_for_each_type() {
        use strum::IntoEnumIterator;
        for node_type in NodeType::iter() {
            let node = Node::from_discriminant(node_type);
            assert_eq!(node.as_type(), node_type);
        }
    }

    #[test]
    fn into_node_convenience_matches_from_discriminant() {
        let node = NodeType::Paragraph.into_node();
        assert!(matches!(node, Node::Paragraph(_)));
    }

    #[test]
    fn apply_attr_dispatches_by_kind() {
        let mut node = Node::Callout(CalloutNode::default());
        node.apply_attr(
            Dot::new(1, 0),
            &NodeAttr::Callout {
                attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
            },
        )
        .unwrap();
        if let Node::Callout(n) = &node {
            assert_eq!(*n.variant.get(), CalloutVariant::Warning);
        } else {
            panic!("expected Callout");
        }
    }

    #[test]
    fn apply_attr_kind_mismatch_returns_error() {
        let mut node = Node::Root(RootNode::default());
        let result = node.apply_attr(
            Dot::new(1, 0),
            &NodeAttr::Callout {
                attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
            },
        );
        assert_eq!(result, Err(ModelError::AttrNodeKindMismatch));
    }
}
