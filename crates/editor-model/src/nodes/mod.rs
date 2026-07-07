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
mod tab;
mod table;
mod table_cell;
mod table_row;
mod text;
mod unknown;

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
pub use tab::*;
pub use table::*;
pub use table_cell::*;
pub use table_row::*;
pub use text::*;
pub use unknown::*;

use std::sync::LazyLock;

use crate::ModelError;
use crate::Modifier;
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
))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
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
    Tab(TabNode),
    Unknown(UnknownNode),
}

static FOLD_TITLE_IMPLICIT: LazyLock<Vec<Modifier>> = LazyLock::new(|| {
    vec![
        Modifier::FontWeight { value: 500 },
        Modifier::FontSize { value: 1050 },
        Modifier::LineHeight { value: 160 },
        Modifier::LetterSpacing { value: 0 },
        Modifier::TextColor {
            value: "gray".to_string(),
        },
    ]
});

static MESSAGE_SENT_IMPLICIT: LazyLock<Vec<Modifier>> = LazyLock::new(|| {
    vec![Modifier::TextColor {
        value: "bright".to_string(),
    }]
});

impl Node {
    pub fn as_type(&self) -> NodeType {
        NodeType::from(self)
    }

    // Modifiers a node type imposes implicitly, surfaced through
    // `NodeRef::modifiers()` as if they sat on the node. They are never
    // written to the document — `NodeRef::explicit_modifiers()` is the
    // persisted-only view. Resolvers therefore resolve identical (family,
    // weight) for render and font collection without per-call-site wiring.
    pub fn implicit_modifiers(&self) -> &'static [Modifier] {
        match self {
            Node::FoldTitle(_) => FOLD_TITLE_IMPLICIT.as_slice(),
            Node::Blockquote(bq) if *bq.variant.get() == BlockquoteVariant::MessageSent => {
                MESSAGE_SENT_IMPLICIT.as_slice()
            }
            _ => &[],
        }
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
