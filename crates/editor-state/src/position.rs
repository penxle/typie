use editor_macros::ffi;
use editor_model::{Doc, NodeId};
use serde::{Deserialize, Serialize};

use crate::affinity::Affinity;
use crate::resolved_position::ResolvedPosition;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Position {
    pub node_id: NodeId,
    pub offset: usize,
    pub affinity: Affinity,
}

impl Position {
    pub fn new(node_id: NodeId, offset: usize) -> Self {
        Self {
            node_id,
            offset,
            affinity: Affinity::default(),
        }
    }

    pub fn resolve<'a>(&self, doc: &'a Doc) -> Option<ResolvedPosition<'a>> {
        ResolvedPosition::resolve(doc, *self)
    }
}
