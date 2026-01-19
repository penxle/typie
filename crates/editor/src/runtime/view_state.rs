use crate::model::NodeId;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Default)]
pub enum NodeViewState {
    #[default]
    None,
    Fold {
        expanded: bool,
    },
    Image {
        width: f32,
        height: f32,
    },
}

impl NodeViewState {
    pub fn fold_expanded(&self) -> bool {
        match self {
            NodeViewState::Fold { expanded } => *expanded,
            _ => false,
        }
    }

    pub fn image_dimensions(&self) -> Option<(f32, f32)> {
        match self {
            NodeViewState::Image { width, height } => Some((*width, *height)),
            _ => None,
        }
    }
}

pub type ViewStates = FxHashMap<NodeId, NodeViewState>;
