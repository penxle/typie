use editor_codec_macros::Durable;

use crate::framing::UnknownPayload;
use crate::types::values::DurableAlignment;

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableModifier {
    #[durable(n(0))]
    Bold,
    #[durable(n(1))]
    Italic,
    #[durable(n(2))]
    Underline,
    #[durable(n(3))]
    Strikethrough,
    #[durable(n(4))]
    #[durable(frozen)]
    FontSize(u32),
    #[durable(n(5))]
    #[durable(frozen)]
    FontFamily(String),
    #[durable(n(6))]
    #[durable(frozen)]
    FontWeight(u16),
    #[durable(n(7))]
    #[durable(frozen)]
    TextColor(String),
    #[durable(n(8))]
    #[durable(frozen)]
    BackgroundColor(String),
    #[durable(n(9))]
    #[durable(frozen)]
    LetterSpacing(i32),
    #[durable(n(10))]
    #[durable(frozen)]
    Link(String),
    #[durable(n(11))]
    #[durable(frozen)]
    Ruby(String),
    #[durable(n(12))]
    #[durable(frozen)]
    LineHeight(u32),
    #[durable(n(13))]
    #[durable(frozen)]
    BlockGap(u32),
    #[durable(n(14))]
    #[durable(frozen)]
    ParagraphIndent(u32),
    #[durable(n(15))]
    #[durable(frozen)]
    Alignment(DurableAlignment),
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableModifier {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableModifier::Bold
            | DurableModifier::Italic
            | DurableModifier::Underline
            | DurableModifier::Strikethrough
            | DurableModifier::FontSize(_)
            | DurableModifier::FontFamily(_)
            | DurableModifier::FontWeight(_)
            | DurableModifier::TextColor(_)
            | DurableModifier::BackgroundColor(_)
            | DurableModifier::LetterSpacing(_)
            | DurableModifier::Link(_)
            | DurableModifier::Ruby(_)
            | DurableModifier::LineHeight(_)
            | DurableModifier::BlockGap(_)
            | DurableModifier::ParagraphIndent(_) => false,
            DurableModifier::Alignment(a) => a.contains_ctx_unknown(),
            DurableModifier::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableModifierKind {
    #[durable(n(0))]
    Bold,
    #[durable(n(1))]
    Italic,
    #[durable(n(2))]
    Underline,
    #[durable(n(3))]
    Strikethrough,
    #[durable(n(4))]
    FontSize,
    #[durable(n(5))]
    FontFamily,
    #[durable(n(6))]
    FontWeight,
    #[durable(n(7))]
    TextColor,
    #[durable(n(8))]
    BackgroundColor,
    #[durable(n(9))]
    LetterSpacing,
    #[durable(n(10))]
    Link,
    #[durable(n(11))]
    Ruby,
    #[durable(n(12))]
    LineHeight,
    #[durable(n(13))]
    BlockGap,
    #[durable(n(14))]
    ParagraphIndent,
    #[durable(n(15))]
    Alignment,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableModifierKind {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableModifierKind::Bold
            | DurableModifierKind::Italic
            | DurableModifierKind::Underline
            | DurableModifierKind::Strikethrough
            | DurableModifierKind::FontSize
            | DurableModifierKind::FontFamily
            | DurableModifierKind::FontWeight
            | DurableModifierKind::TextColor
            | DurableModifierKind::BackgroundColor
            | DurableModifierKind::LetterSpacing
            | DurableModifierKind::Link
            | DurableModifierKind::Ruby
            | DurableModifierKind::LineHeight
            | DurableModifierKind::BlockGap
            | DurableModifierKind::ParagraphIndent
            | DurableModifierKind::Alignment => false,
            DurableModifierKind::Unknown(_) => true,
        }
    }
}
