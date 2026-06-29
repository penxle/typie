use crate::view_state::GapPhantom;
use editor_crdt::Dot;
use hashbrown::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct MeasureContext {
    pub fold_states: HashMap<Dot, bool>,
    pub external_heights: HashMap<Dot, f32>,
    pub gap_phantom: Option<GapPhantom>,
    pub pending_style: Option<(Dot, editor_state::PendingModifiers)>,
}

impl MeasureContext {
    pub fn fold_expanded(&self, node: &Dot) -> bool {
        self.fold_states.get(node).copied().unwrap_or(true)
    }

    pub fn external_height(&self, node: &Dot) -> Option<f32> {
        self.external_heights.get(node).copied()
    }

    pub fn gap_phantom_index(&self, node: &Dot) -> Option<usize> {
        self.gap_phantom
            .as_ref()
            .filter(|gp| &gp.parent == node)
            .map(|gp| gp.index)
    }

    pub fn pending_for(&self, node: &Dot) -> Option<&editor_state::PendingModifiers> {
        self.pending_style
            .as_ref()
            .filter(|(id, _)| id == node)
            .map(|(_, m)| m)
    }
}

pub(crate) fn measure_context(vs: &crate::view_state::ViewState) -> MeasureContext {
    MeasureContext {
        fold_states: vs.fold_states.clone(),
        external_heights: vs.external_heights.clone(),
        gap_phantom: vs.gap_phantom.as_ref().map(|gp| GapPhantom {
            parent: gp.parent,
            index: gp.index,
        }),
        pending_style: vs
            .pending_style
            .as_ref()
            .map(|ps| (ps.node_id, ps.modifiers.clone())),
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_model::Modifier;
    use editor_state::PendingModifier;
    use hashbrown::HashMap;

    use crate::view_state::{GapPhantom, PendingStyle, ViewState};

    use super::{MeasureContext, measure_context};

    #[test]
    fn fold_expanded_default_and_set() {
        let id = Dot::new(1, 1);
        let ctx = MeasureContext::default();
        assert!(ctx.fold_expanded(&id));

        let ctx_false = MeasureContext {
            fold_states: HashMap::from([(id, false)]),
            ..Default::default()
        };
        assert!(!ctx_false.fold_expanded(&id));
    }

    #[test]
    fn external_height_some_none() {
        let id = Dot::new(1, 1);
        let ctx = MeasureContext {
            external_heights: HashMap::from([(id, 150.0)]),
            ..Default::default()
        };
        assert_eq!(ctx.external_height(&id), Some(150.0));

        let other = Dot::new(1, 2);
        assert_eq!(ctx.external_height(&other), None);
    }

    #[test]
    fn gap_phantom_index_matches_only_parent() {
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let ctx = MeasureContext {
            gap_phantom: Some(GapPhantom {
                parent: a,
                index: 2,
            }),
            ..Default::default()
        };
        assert_eq!(ctx.gap_phantom_index(&a), Some(2));
        assert_eq!(ctx.gap_phantom_index(&b), None);
    }

    #[test]
    fn pending_for_matches_only_own_elem() {
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let ctx = MeasureContext {
            pending_style: Some((a, modifiers.clone())),
            ..Default::default()
        };
        assert_eq!(ctx.pending_for(&a), Some(&modifiers));
        assert_eq!(ctx.pending_for(&b), None);
    }

    #[test]
    fn measure_context_copies_state_fields() {
        let p1 = Dot::new(1, 1);
        let i1 = Dot::new(1, 2);
        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let mut vs = ViewState::new();
        vs.fold_states.insert(p1, false);
        vs.external_heights.insert(i1, 200.0);
        vs.gap_phantom = Some(GapPhantom {
            parent: p1,
            index: 1,
        });
        vs.pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: modifiers.clone(),
        });

        let ctx = measure_context(&vs);

        assert_eq!(ctx.fold_states.get(&p1), Some(&false));
        assert_eq!(ctx.external_heights.get(&i1), Some(&200.0));
        assert_eq!(
            ctx.gap_phantom,
            Some(GapPhantom {
                parent: p1,
                index: 1
            })
        );
        assert_eq!(ctx.pending_style, Some((p1, modifiers)));
    }
}
