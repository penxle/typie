use editor_crdt::{Changeset, OpGraph};
use editor_model::{DocView, EditOp};

use crate::Selection;
use crate::composition::Composition;
use crate::error::StateError;
use crate::pending_modifier::PendingModifiers;
use crate::pending_style::PendingStyle;
use crate::projected_state::ProjectedState;

#[derive(Clone, Debug)]
pub struct State {
    pub projected: ProjectedState,
    pub selection: Option<Selection>,
    pub pending_modifiers: PendingModifiers,
    pub pending_style: Option<PendingStyle>,
    pub composition: Option<Composition>,
}

impl State {
    pub fn new(projected: ProjectedState, selection: Option<Selection>) -> Self {
        Self {
            projected,
            selection,
            pending_modifiers: PendingModifiers::new(),
            pending_style: None,
            composition: None,
        }
    }

    pub fn empty() -> Self {
        Self::new(ProjectedState::empty(), None)
    }

    pub fn from_changesets(
        css: Vec<Changeset<EditOp>>,
        selection: Option<Selection>,
    ) -> Result<Self, StateError> {
        let graph = OpGraph::from_changesets(css)?;
        let projected = ProjectedState::from_graph(graph)?;
        Ok(Self::new(projected, selection))
    }

    pub fn graph(&self) -> &OpGraph<EditOp> {
        self.projected.graph()
    }

    pub fn view(&self) -> DocView<'_> {
        self.projected.view()
    }

    pub fn from_plain(
        plain: &editor_model::PlainDoc,
    ) -> Result<Self, crate::load_builder::BuildError> {
        let (projected, selection) = crate::load_builder::load_from_plain(plain)?;
        Ok(Self::new(projected, selection))
    }

    pub fn to_plain(&self) -> editor_model::PlainDoc {
        crate::to_plain::to_plain(self.projected.projected())
    }
}

pub fn state_observably_changed(a: &State, b: &State) -> bool {
    a.projected.projected() != b.projected.projected()
        || a.selection != b.selection
        || a.pending_modifiers != b.pending_modifiers
        || a.pending_style != b.pending_style
        || a.composition != b.composition
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_has_no_selection() {
        let s = State::empty();
        assert!(s.selection.is_none());
        assert!(s.view().root().is_some());
    }

    #[test]
    fn clone_is_observably_unchanged() {
        let s = State::empty();
        let c = s.clone();
        assert!(!state_observably_changed(&s, &c));
    }
}
