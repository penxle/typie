use editor_codec_macros::Durable;
use editor_crdt::Dot;

use crate::framing::{UnknownPayload, UnknownTail};
use crate::types::attr::DurableAttr;

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableNodeType {
    #[durable(n(0))]
    Root,
    #[durable(n(1))]
    Paragraph,
    #[durable(n(2))]
    Blockquote,
    #[durable(n(3))]
    Callout,
    #[durable(n(4))]
    Text,
    #[durable(n(5))]
    BulletList,
    #[durable(n(6))]
    OrderedList,
    #[durable(n(7))]
    ListItem,
    #[durable(n(8))]
    Fold,
    #[durable(n(9))]
    FoldTitle,
    #[durable(n(10))]
    FoldContent,
    #[durable(n(11))]
    Table,
    #[durable(n(12))]
    TableRow,
    #[durable(n(13))]
    TableCell,
    #[durable(n(14))]
    Image,
    #[durable(n(15))]
    File,
    #[durable(n(16))]
    Embed,
    #[durable(n(17))]
    Archived,
    #[durable(n(18))]
    HardBreak,
    #[durable(n(19))]
    HorizontalRule,
    #[durable(n(20))]
    PageBreak,
    #[durable(n(21))]
    Tab,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableNodeType {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableNodeType::Root
            | DurableNodeType::Paragraph
            | DurableNodeType::Blockquote
            | DurableNodeType::Callout
            | DurableNodeType::Text
            | DurableNodeType::BulletList
            | DurableNodeType::OrderedList
            | DurableNodeType::ListItem
            | DurableNodeType::Fold
            | DurableNodeType::FoldTitle
            | DurableNodeType::FoldContent
            | DurableNodeType::Table
            | DurableNodeType::TableRow
            | DurableNodeType::TableCell
            | DurableNodeType::Image
            | DurableNodeType::File
            | DurableNodeType::Embed
            | DurableNodeType::Archived
            | DurableNodeType::HardBreak
            | DurableNodeType::HorizontalRule
            | DurableNodeType::PageBreak
            | DurableNodeType::Tab => false,
            DurableNodeType::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableItem {
    #[durable(n(0))]
    #[durable(frozen)]
    Char(char),
    #[durable(n(1))]
    Atom {
        node_type: DurableNodeType,
        init: Vec<DurableAttr>,
        tail: UnknownTail,
    },
    #[durable(n(2))]
    Block {
        node_type: DurableNodeType,
        parents: Vec<Dot>,
        init: Vec<DurableAttr>,
        tail: UnknownTail,
    },
    #[durable(n(3))]
    BlockAtom {
        node_type: DurableNodeType,
        parents: Vec<Dot>,
        init: Vec<DurableAttr>,
        tail: UnknownTail,
    },
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableItem {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableItem::Char(_) => false,
            DurableItem::Atom {
                node_type,
                init,
                tail,
            } => {
                node_type.contains_ctx_unknown()
                    || !tail.0.is_empty()
                    || init.iter().any(DurableAttr::is_unknown_bearing)
            }
            DurableItem::Block {
                node_type,
                parents: _,
                init: _,
                tail,
            } => node_type.contains_ctx_unknown() || !tail.0.is_empty(),
            DurableItem::BlockAtom {
                node_type,
                parents: _,
                init,
                tail,
            } => {
                node_type.contains_ctx_unknown()
                    || !tail.0.is_empty()
                    || init.iter().any(DurableAttr::is_unknown_bearing)
            }
            DurableItem::Unknown(_) => true,
        }
    }
}
