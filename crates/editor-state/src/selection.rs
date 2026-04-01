use editor_macros::ffi;
use editor_model::Doc;
use serde::{Deserialize, Serialize};

use crate::position::Position;
use crate::resolved_selection::ResolvedSelection;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn collapsed(pos: Position) -> Self {
        Self {
            anchor: pos.clone(),
            head: pos,
        }
    }

    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn resolve<'a>(&self, doc: &'a Doc) -> Option<ResolvedSelection<'a>> {
        ResolvedSelection::resolve(doc, *self)
    }
}
