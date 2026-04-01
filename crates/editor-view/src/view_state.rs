use editor_model::NodeId;
use hashbrown::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ViewState {
    pub fold_states: HashMap<NodeId, bool>,
    pub external_heights: HashMap<NodeId, f32>,
}

impl ViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fold_expanded(&self, node_id: NodeId) -> bool {
        self.fold_states.get(&node_id).copied().unwrap_or(true)
    }

    pub fn external_height(&self, node_id: NodeId) -> Option<f32> {
        self.external_heights.get(&node_id).copied()
    }
}
