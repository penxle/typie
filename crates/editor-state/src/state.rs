use std::sync::Arc;

use editor_crdt::{Changeset, OpGraph};
use editor_model::{DocView, EditOp};

use crate::Selection;
use crate::composition::Composition;
use crate::error::StateError;
use crate::pending_modifier::PendingModifiers;
use crate::projected_state::ProjectedState;

#[derive(Clone, Debug)]
pub struct State {
    // `Arc`-wrapped so `State::clone` (every `Transaction::new`) shares the
    // projected document by pointer instead of deep-copying it. Transactions
    // that never touch the document (selection / caret moves) keep the shared
    // copy; document edits go through `projected_mut` (copy-on-write).
    pub projected: Arc<ProjectedState>,
    pub selection: Option<Selection>,
    pub pending_modifiers: PendingModifiers,
    pub composition: Option<Composition>,
}

impl State {
    pub fn new(projected: ProjectedState, selection: Option<Selection>) -> Self {
        Self {
            projected: Arc::new(projected),
            selection,
            pending_modifiers: PendingModifiers::new(),
            composition: None,
        }
    }

    /// Copy-on-write access to the projected state. Clones the shared
    /// `ProjectedState` only if another `State` still references it.
    pub fn projected_mut(&mut self) -> &mut ProjectedState {
        Arc::make_mut(&mut self.projected)
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

    fn zombie_css() -> Vec<Changeset<EditOp>> {
        use editor_crdt::{Dot, ListOp, Op};
        use editor_model::{NodeType, SeqItem};
        let d = |c| Dot::new(1, c);
        vec![Changeset {
            ops: vec![
                Op {
                    id: d(0),
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: d(1),
                    parents: vec![d(0)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
                Op {
                    id: d(2),
                    parents: vec![d(1)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 2,
                        item: SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![Dot::ROOT, Dot::new(9, 999)],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: d(3),
                    parents: vec![d(2)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 3,
                        item: SeqItem::Char('z'),
                    }),
                },
            ],
        }]
    }

    fn clean_css() -> Vec<Changeset<EditOp>> {
        let mut css = zombie_css();
        css[0].ops.truncate(2);
        css
    }

    #[test]
    fn projection_counts_dropped_subtree_real_dots() {
        use editor_crdt::Dot;
        let state = State::from_changesets(zombie_css(), None).unwrap();
        assert!(
            state.view().node(Dot::new(1, 2)).is_none(),
            "전제: 현행 워크는 이 마커를 드롭"
        );
        assert_eq!(state.projected.projected().drops, 2, "마커+char");
    }

    #[test]
    fn projection_of_clean_document_counts_no_drops() {
        let state = State::from_changesets(clean_css(), None).unwrap();
        assert_eq!(state.projected.projected().drops, 0);
    }

    #[test]
    fn persistent_zombie_recounts_on_each_full_reprojection() {
        let mut state = State::from_changesets(zombie_css(), None).unwrap();
        assert_eq!(
            state.projected_mut().take_drop_stats(),
            2,
            "initial projection counts the zombie marker + char, then drains to 0"
        );
        // The zombie still lives in the sequence, so a fresh whole-document
        // reprojection drops it again — the tripwire is per-pass, not deduplicated.
        state.projected_mut().reproject_all().unwrap();
        assert_eq!(
            state.projected_mut().take_drop_stats(),
            2,
            "a persistent zombie re-counts on every full reprojection"
        );
    }
}
