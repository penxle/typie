use editor_crdt::{Changeset, OpGraph};
use editor_model::{Doc, DocOp};

use crate::composition::Composition;
use crate::error::StateError;
use crate::pending_modifier::PendingModifiers;
use crate::pending_style::PendingStyle;
use crate::selection::Selection;

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub graph: OpGraph<DocOp>,
    pub doc: Doc,
    pub selection: Option<Selection>,
    pub pending_modifiers: PendingModifiers,
    pub pending_style: Option<PendingStyle>,
    pub composition: Option<Composition>,
}

impl State {
    pub fn new(doc: Doc, graph: OpGraph<DocOp>, selection: Option<Selection>) -> Self {
        Self {
            doc,
            graph,
            selection,
            pending_modifiers: PendingModifiers::new(),
            pending_style: None,
            composition: None,
        }
    }

    pub fn from_changesets(
        css: Vec<Changeset<DocOp>>,
        selection: Option<Selection>,
    ) -> Result<Self, StateError> {
        let graph = OpGraph::from_changesets(css)?;
        let doc = Doc::from_op_graph(&graph)?;
        Ok(Self::new(doc, graph, selection))
    }
}

pub fn state_observably_changed(a: &State, b: &State) -> bool {
    a.doc != b.doc
        || a.selection != b.selection
        || a.pending_modifiers != b.pending_modifiers
        || a.pending_style != b.pending_style
        || a.composition != b.composition
}

#[cfg(test)]
mod tests_observable {
    use super::*;
    use crate::pending_modifier::PendingModifier;
    use crate::position::Position;
    use editor_macros::state;
    use editor_model::Modifier;

    #[test]
    fn same_state_is_unchanged() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        assert!(!state_observably_changed(&s, &s));
    }

    #[test]
    fn different_selection_is_changed() {
        let (a, t1) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut b = a.clone();
        b.selection = Some(Selection::collapsed(Position::new(t1, 1)));
        assert!(state_observably_changed(&a, &b));
    }

    #[test]
    fn different_pending_modifiers_is_changed() {
        let (a, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut b = a.clone();
        b.pending_modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        assert!(state_observably_changed(&a, &b));
    }

    #[test]
    fn different_pending_style_is_changed() {
        let (a, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut b = a.clone();
        b.pending_style = Some(crate::PendingStyle::Set {
            style_id: "s1".into(),
        });
        assert!(state_observably_changed(&a, &b));
    }
}
