use editor_codec_macros::Durable;

use crate::framing::UnknownPayload;

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableLayoutMode {
    #[durable(n(0))]
    Paginated {
        page_width: u32,
        page_height: u32,
        page_margin_top: u32,
        page_margin_bottom: u32,
        page_margin_left: u32,
        page_margin_right: u32,
        tail: crate::framing::UnknownTail,
    },
    #[durable(n(1))]
    Continuous {
        max_width: u32,
        tail: crate::framing::UnknownTail,
    },
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableLayoutMode {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableLayoutMode::Paginated { tail, .. } => !tail.0.is_empty(),
            DurableLayoutMode::Continuous { tail, .. } => !tail.0.is_empty(),
            DurableLayoutMode::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableBlockquoteVariant {
    #[durable(n(0))]
    LeftLine,
    #[durable(n(1))]
    LeftQuote,
    #[durable(n(2))]
    MessageSent,
    #[durable(n(3))]
    MessageReceived,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableBlockquoteVariant {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableBlockquoteVariant::LeftLine
            | DurableBlockquoteVariant::LeftQuote
            | DurableBlockquoteVariant::MessageSent
            | DurableBlockquoteVariant::MessageReceived => false,
            DurableBlockquoteVariant::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableCalloutVariant {
    #[durable(n(0))]
    Info,
    #[durable(n(1))]
    Success,
    #[durable(n(2))]
    Warning,
    #[durable(n(3))]
    Danger,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableCalloutVariant {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableCalloutVariant::Info
            | DurableCalloutVariant::Success
            | DurableCalloutVariant::Warning
            | DurableCalloutVariant::Danger => false,
            DurableCalloutVariant::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableHorizontalRuleVariant {
    #[durable(n(0))]
    Line,
    #[durable(n(1))]
    DashedLine,
    #[durable(n(2))]
    CircleLine,
    #[durable(n(3))]
    DiamondLine,
    #[durable(n(4))]
    Circle,
    #[durable(n(5))]
    Diamond,
    #[durable(n(6))]
    ThreeCircles,
    #[durable(n(7))]
    ThreeDiamonds,
    #[durable(n(8))]
    Zigzag,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableHorizontalRuleVariant {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableHorizontalRuleVariant::Line
            | DurableHorizontalRuleVariant::DashedLine
            | DurableHorizontalRuleVariant::CircleLine
            | DurableHorizontalRuleVariant::DiamondLine
            | DurableHorizontalRuleVariant::Circle
            | DurableHorizontalRuleVariant::Diamond
            | DurableHorizontalRuleVariant::ThreeCircles
            | DurableHorizontalRuleVariant::ThreeDiamonds
            | DurableHorizontalRuleVariant::Zigzag => false,
            DurableHorizontalRuleVariant::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableTableBorderStyle {
    #[durable(n(0))]
    Solid,
    #[durable(n(1))]
    Dashed,
    #[durable(n(2))]
    Dotted,
    #[durable(n(3))]
    None,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableTableBorderStyle {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableTableBorderStyle::Solid
            | DurableTableBorderStyle::Dashed
            | DurableTableBorderStyle::Dotted
            | DurableTableBorderStyle::None => false,
            DurableTableBorderStyle::Unknown(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableAlignment {
    #[durable(n(0))]
    Left,
    #[durable(n(1))]
    Center,
    #[durable(n(2))]
    Right,
    #[durable(n(3))]
    Justify,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableAlignment {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableAlignment::Left
            | DurableAlignment::Center
            | DurableAlignment::Right
            | DurableAlignment::Justify => false,
            DurableAlignment::Unknown(_) => true,
        }
    }
}
