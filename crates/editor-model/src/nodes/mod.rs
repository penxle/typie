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

use editor_macros::{FromDiscriminant, ffi};
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumDiscriminants, EnumIter};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumDiscriminants, FromDiscriminant)]
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
))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[serde(tag = "type", rename_all = "snake_case")]
#[from_discriminant(NodeType)]
pub enum Node {
    Root(RootNode),
    Paragraph(ParagraphNode),
    Blockquote(BlockquoteNode),
    Callout(CalloutNode),
    Text(TextNode),
    BulletList(BulletListNode),
    OrderedList(OrderedListNode),
    ListItem(ListItemNode),
    Fold(FoldNode),
    FoldTitle(FoldTitleNode),
    FoldContent(FoldContentNode),
    Table(TableNode),
    TableRow(TableRowNode),
    TableCell(TableCellNode),
    Image(ImageNode),
    File(FileNode),
    Embed(EmbedNode),
    Archived(ArchivedNode),
    HardBreak(HardBreakNode),
    HorizontalRule(HorizontalRuleNode),
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
    fn serde_roundtrip() {
        let node = Node::Callout(CalloutNode {
            variant: CalloutVariant::Warning,
        });
        let json = serde_json::to_string(&node).unwrap();
        let parsed: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(node, parsed);
    }
}
