use editor_model::Doc;

use crate::composition::Composition;
use crate::pending_modifier::PendingModifiers;
use crate::selection::Selection;

#[derive(Clone, Debug)]
pub struct State {
    pub doc: Doc,
    pub selection: Selection,
    pub pending_modifiers: PendingModifiers,
    pub composition: Option<Composition>,
}

impl State {
    pub fn new(doc: Doc, selection: Selection) -> Self {
        Self {
            doc,
            selection,
            pending_modifiers: PendingModifiers::new(),
            composition: None,
        }
    }
}
