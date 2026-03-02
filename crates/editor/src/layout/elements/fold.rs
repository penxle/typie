use crate::layout::elements::{SplitEdges, Wrapper, WrapperPadding};
use crate::model::NodeId;
use crate::types::Size;

pub const FOLD_CONTENT_PADDING_X: f32 = 24.0;
pub const FOLD_CONTENT_PADDING_Y: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct FoldTitleElement {
    pub size: Size,
    pub block_id: NodeId,
    pub fold_id: NodeId,
    pub expanded: bool,
}

impl FoldTitleElement {
    pub fn new(size: Size, block_id: NodeId, fold_id: NodeId, expanded: bool) -> Self {
        Self {
            size,
            block_id,
            fold_id,
            expanded,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FoldTitleBackgroundElement {
    pub size: Size,
    pub expanded: bool,
    pub fold_id: NodeId,
}

impl FoldTitleBackgroundElement {
    pub fn new(size: Size, expanded: bool, fold_id: NodeId) -> Self {
        Self {
            size,
            expanded,
            fold_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FoldContentElement {
    pub size: Size,
    pub split_edges: SplitEdges,
    pub fold_id: NodeId,
}

impl FoldContentElement {
    pub fn new(size: Size, split_edges: SplitEdges, fold_id: NodeId) -> Self {
        Self {
            size,
            split_edges,
            fold_id,
        }
    }
}

impl Wrapper for FoldContentElement {
    fn padding(&self) -> WrapperPadding {
        WrapperPadding::symmetric(FOLD_CONTENT_PADDING_Y, FOLD_CONTENT_PADDING_X)
    }
}
