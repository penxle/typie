use crate::layout::elements::{SplitEdges, Wrapper, WrapperPadding};
use crate::model::{BlockquoteVariant, NodeId};
use crate::types::{PaintOverflow, Size};

const MESSAGE_PADDING_X: f32 = 14.0;
const MESSAGE_PADDING_Y: f32 = 8.0;
pub const MESSAGE_TAIL_SIZE: f32 = 10.0;
const MESSAGE_TAIL_X_OVERFLOW_FACTOR: f32 = 0.4;
const MESSAGE_TAIL_BOTTOM_OVERFLOW_FACTOR: f32 = 0.15;
const MESSAGE_TAIL_ANTIALIAS_MARGIN: f32 = 1.0;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockquoteLineElement {
    pub size: Size,
    pub block_id: NodeId,
}

impl BlockquoteLineElement {
    pub fn new(size: Size, block_id: NodeId) -> Self {
        Self { size, block_id }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockquoteQuoteElement {
    pub size: Size,
    pub block_id: NodeId,
}

impl BlockquoteQuoteElement {
    pub fn new(size: Size, block_id: NodeId) -> Self {
        Self { size, block_id }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct BlockquoteMessageElement {
    pub size: Size,
    pub block_id: NodeId,
    pub variant: BlockquoteVariant,
    pub split_edges: SplitEdges,
}

impl BlockquoteMessageElement {
    pub fn new(
        size: Size,
        block_id: NodeId,
        variant: BlockquoteVariant,
        split_edges: SplitEdges,
    ) -> Self {
        Self {
            size,
            block_id,
            variant,
            split_edges,
        }
    }

    pub fn paint_overflow(&self) -> PaintOverflow {
        if self.split_edges.bottom {
            return PaintOverflow::default();
        }

        let horizontal_overflow =
            MESSAGE_TAIL_SIZE * MESSAGE_TAIL_X_OVERFLOW_FACTOR + MESSAGE_TAIL_ANTIALIAS_MARGIN;
        let bottom_overflow =
            MESSAGE_TAIL_SIZE * MESSAGE_TAIL_BOTTOM_OVERFLOW_FACTOR + MESSAGE_TAIL_ANTIALIAS_MARGIN;

        match self.variant {
            BlockquoteVariant::MessageSent => PaintOverflow {
                right: horizontal_overflow,
                bottom: bottom_overflow,
                ..PaintOverflow::default()
            },
            BlockquoteVariant::MessageReceived => PaintOverflow {
                left: horizontal_overflow,
                bottom: bottom_overflow,
                ..PaintOverflow::default()
            },
            _ => PaintOverflow::default(),
        }
    }
}

impl Wrapper for BlockquoteMessageElement {
    fn padding(&self) -> WrapperPadding {
        WrapperPadding::symmetric(MESSAGE_PADDING_Y, MESSAGE_PADDING_X)
    }

    fn prevent_empty_on_page_break(&self) -> bool {
        true
    }
}
