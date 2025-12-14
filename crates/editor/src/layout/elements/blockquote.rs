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
