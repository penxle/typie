use crate::layout::elements::{SplitEdges, Wrapper, WrapperPadding};
use crate::model::{BlockquoteVariant, NodeId};
use crate::types::Size;

const MESSAGE_PADDING_X: f32 = 14.0;
const MESSAGE_PADDING_Y: f32 = 8.0;

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
}

impl Wrapper for BlockquoteMessageElement {
    fn padding(&self) -> WrapperPadding {
        WrapperPadding::symmetric(MESSAGE_PADDING_Y, MESSAGE_PADDING_X)
    }

    fn prevent_empty_on_page_break(&self) -> bool {
        true
    }
}
