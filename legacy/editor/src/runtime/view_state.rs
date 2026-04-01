use crate::model::NodeId;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Default)]
pub enum NodeViewState {
    #[default]
    None,
    Fold {
        expanded: bool,
    },
    ExternalHeight {
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

    pub fn external_height(&self) -> Option<f32> {
        match self {
            NodeViewState::ExternalHeight { height } => Some(*height),
            _ => None,
        }
    }
}

pub type ViewStates = FxHashMap<NodeId, NodeViewState>;
