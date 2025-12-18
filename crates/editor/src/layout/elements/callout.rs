use crate::layout::elements::{SplitEdges, WrapperPadding};
use crate::layout::interactive::{InteractionKind, Interactive};
use crate::model::CalloutType;
use crate::model::NodeId;
use crate::types::Size;

pub const CALLOUT_PADDING_X: f32 = 12.0;
pub const CALLOUT_PADDING_Y: f32 = 16.0;

#[derive(Debug, Clone)]
pub struct CalloutBackgroundElement {
    pub size: Size,
    pub callout_type: CalloutType,
    pub node_id: NodeId,
    pub split_edges: SplitEdges,
}

impl CalloutBackgroundElement {
    pub fn new(size: Size, callout_type: CalloutType, node_id: NodeId, split_edges: SplitEdges) -> Self {
        Self {
            size,
            callout_type,
            node_id,
            split_edges,
        }
    }
}

impl crate::layout::elements::Wrapper for CalloutBackgroundElement {
    fn padding(&self) -> WrapperPadding {
        WrapperPadding::symmetric(CALLOUT_PADDING_Y, CALLOUT_PADDING_X)
    }

    fn prevent_empty_on_page_break(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct CalloutIconElement {
    pub size: Size,
    pub callout_type: CalloutType,
    pub node_id: NodeId,
}

impl CalloutIconElement {
    pub fn new(size: Size, callout_type: CalloutType, node_id: NodeId) -> Self {
        Self {
            size,
            callout_type,
            node_id,
        }
    }
}

impl Interactive for CalloutIconElement {
    fn interaction_kind(&self) -> InteractionKind {
        InteractionKind::CycleCalloutType {
            node_id: self.node_id,
        }
    }
}
