use crate::layout::elements::SplitEdges;
use crate::model::BlockquoteVariant;
use crate::model::NodeId;
use crate::types::Size;

#[derive(Debug, Clone)]
pub struct BlockquoteLineElement {
    pub size: Size,
    pub block_id: NodeId,
}

impl BlockquoteLineElement {
    pub fn new(size: Size, block_id: NodeId) -> Self {
        Self { size, block_id }
    }
}

#[derive(Debug, Clone)]
pub struct BlockquoteQuoteElement {
    pub size: Size,
    pub block_id: NodeId,
}

impl BlockquoteQuoteElement {
    pub fn new(size: Size, block_id: NodeId) -> Self {
        Self { size, block_id }
    }
}

#[derive(Debug, Clone)]
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
