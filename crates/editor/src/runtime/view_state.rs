use crate::model::NodeId;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Default)]
pub enum NodeViewState {
    #[default]
    None,
    Fold {
        expanded: bool,
    },
}

impl NodeViewState {
    pub fn fold_expanded(&self) -> bool {
        match self {
            NodeViewState::Fold { expanded } => *expanded,
            _ => false,
        }
    }
}

pub type ViewStates = FxHashMap<NodeId, NodeViewState>;
