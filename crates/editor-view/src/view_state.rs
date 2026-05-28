use editor_common::DecorationStyle;
use editor_model::NodeId;
use editor_state::PendingModifiers;
use hashbrown::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct PendingStyle {
    pub node_id: NodeId,
    pub modifiers: PendingModifiers,
}

/// A gap cursor's phantom-paragraph descriptor. View-only — the document
/// is not mutated. `parent` is the gap's container (the document root for
/// a leading-unit gap, or the between-monolithic parent container);
/// `index` is the child slot the phantom occupies. Mirrors `PendingStyle`
/// as a ViewState-driven layout input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapPhantom {
    pub parent: NodeId,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupDecoration {
    pub style: DecorationStyle,
    pub enabled: bool,
    pub z_index: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ViewState {
    pub fold_states: HashMap<NodeId, bool>,
    pub external_heights: HashMap<NodeId, f32>,
    pub preferred_x: Option<f32>,
    pub pending_style: Option<PendingStyle>,
    pub gap_phantom: Option<GapPhantom>,
    pub tracked_decoration_groups: HashMap<String, GroupDecoration>,
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

    pub fn group_decoration(&self, group: &str) -> Option<&GroupDecoration> {
        self.tracked_decoration_groups.get(group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::Modifier;
    use editor_state::PendingModifier;

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
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::Bold,
            }],
        };
        let b = PendingStyle {
            node_id: n,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::Bold,
            }],
        };
        assert_eq!(a, b);
    }

    #[test]
    fn gap_phantom_default_is_none() {
        let vs = ViewState::new();
        assert!(vs.gap_phantom.is_none());
    }

    #[test]
    fn gap_phantom_equality() {
        let parent = NodeId::new();
        let a = GapPhantom { parent, index: 1 };
        let b = GapPhantom { parent, index: 1 };
        let c = GapPhantom { parent, index: 2 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
