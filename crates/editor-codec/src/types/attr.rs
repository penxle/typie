use editor_codec_macros::Durable;

use crate::framing::UnknownPayload;
use crate::types::values::{
    DurableBlockquoteVariant, DurableCalloutVariant, DurableHorizontalRuleVariant,
    DurableLayoutMode, DurableTableBorderStyle,
};

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableAttr {
    #[durable(n(0))]
    #[durable(frozen)]
    RootLayoutMode(DurableLayoutMode),
    #[durable(n(1))]
    #[durable(frozen)]
    BlockquoteVariant(DurableBlockquoteVariant),
    #[durable(n(2))]
    #[durable(frozen)]
    CalloutVariant(DurableCalloutVariant),
    #[durable(n(3))]
    #[durable(frozen)]
    TableBorderStyle(DurableTableBorderStyle),
    #[durable(n(4))]
    #[durable(frozen)]
    TableProportion(u32),
    #[durable(n(5))]
    #[durable(frozen)]
    TableCellColWidth(Option<u32>),
    #[durable(n(6))]
    #[durable(frozen)]
    TableCellBackgroundColor(Option<String>),
    #[durable(n(7))]
    #[durable(frozen)]
    ImageId(Option<String>),
    #[durable(n(8))]
    #[durable(frozen)]
    ImageProportion(u32),
    #[durable(n(9))]
    #[durable(frozen)]
    FileId(Option<String>),
    #[durable(n(10))]
    #[durable(frozen)]
    EmbedId(Option<String>),
    #[durable(n(11))]
    #[durable(frozen)]
    ArchivedId(Option<String>),
    #[durable(n(12))]
    #[durable(frozen)]
    HorizontalRuleVariant(DurableHorizontalRuleVariant),
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableAttr {
    pub fn is_unknown_bearing(&self) -> bool {
        match self {
            DurableAttr::RootLayoutMode(m) => m.contains_ctx_unknown(),
            DurableAttr::BlockquoteVariant(v) => v.contains_ctx_unknown(),
            DurableAttr::CalloutVariant(v) => v.contains_ctx_unknown(),
            DurableAttr::TableBorderStyle(v) => v.contains_ctx_unknown(),
            DurableAttr::TableProportion(_) => false,
            DurableAttr::TableCellColWidth(_) => false,
            DurableAttr::TableCellBackgroundColor(_) => false,
            DurableAttr::ImageId(_) => false,
            DurableAttr::ImageProportion(_) => false,
            DurableAttr::FileId(_) => false,
            DurableAttr::EmbedId(_) => false,
            DurableAttr::ArchivedId(_) => false,
            DurableAttr::HorizontalRuleVariant(v) => v.contains_ctx_unknown(),
            DurableAttr::Unknown(_) => true,
        }
    }
}
