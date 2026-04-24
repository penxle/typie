use editor_model::NodeId;
use editor_state::PendingModifiers;
use hashbrown::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct PendingStyle {
    pub node_id: NodeId,
    pub modifiers: PendingModifiers,
}

#[derive(Debug, Clone, Default)]
pub struct ViewState {
    pub fold_states: HashMap<NodeId, bool>,
    pub external_heights: HashMap<NodeId, f32>,
    pub preferred_x: Option<f32>,
    pub pending_style: Option<PendingStyle>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::Modifier;
    use editor_state::PendingModifier;
    use smallvec::smallvec;

    #[test]
    fn pending_style_default_is_none() {
        let vs = ViewState::new();
        assert!(vs.pending_style.is_none());
    }

    #[test]
    fn pending_style_equality() {
        let n = NodeId::new();
        let a = PendingStyle {
            node_id: n,
            modifiers: smallvec![PendingModifier::Set(Modifier::Bold)],
        };
        let b = PendingStyle {
            node_id: n,
            modifiers: smallvec![PendingModifier::Set(Modifier::Bold)],
        };
        assert_eq!(a, b);
    }
}
