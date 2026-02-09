use crate::model::NodeId;
use crate::types::Affinity;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub node_id: NodeId,
    pub offset: usize,
    pub affinity: Affinity,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
            && self.offset == other.offset
            && self.affinity == other.affinity
    }
}

impl Position {
    pub fn new(node_id: NodeId, offset: usize, affinity: Affinity) -> Self {
        Self {
            node_id,
            offset,
            affinity,
        }
    }

    pub fn with_affinity(&self, affinity: Affinity) -> Self {
        Self {
            node_id: self.node_id,
            offset: self.offset,
            affinity,
        }
    }
}
