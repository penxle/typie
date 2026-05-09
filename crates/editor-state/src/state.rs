use editor_crdt::{Changeset, OpGraph};
use editor_model::{Doc, DocOp};

use crate::composition::Composition;
use crate::error::StateError;
use crate::pending_modifier::PendingModifiers;
use crate::selection::Selection;

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub graph: OpGraph<DocOp>,
    pub doc: Doc,
    pub selection: Selection,
    pub pending_modifiers: PendingModifiers,
    pub composition: Option<Composition>,
}

impl State {
    pub fn new(doc: Doc, graph: OpGraph<DocOp>, selection: Selection) -> Self {
        Self {
            doc,
            graph,
            selection,
            pending_modifiers: PendingModifiers::new(),
            composition: None,
        }
    }

    pub fn from_changesets(
        css: Vec<Changeset<DocOp>>,
        selection: Selection,
    ) -> Result<Self, StateError> {
        let graph = OpGraph::from_changesets(css)?;
        let doc = Doc::from_op_graph(&graph)?;
        Ok(Self::new(doc, graph, selection))
    }
}
