use std::collections::BTreeMap;

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::{Marker, Modifier, ModifierType, PlainNode, PlainRootNode};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainDoc {
    pub root: PlainNodeEntry,
}

impl Default for PlainDoc {
    fn default() -> Self {
        Self {
            root: PlainNodeEntry {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: BTreeMap::new(),
                marker: None,
                children: Vec::new(),
            },
        }
    }
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainNodeEntry {
    pub node: PlainNode,
    pub modifiers: BTreeMap<ModifierType, Modifier>,
    #[serde(default)]
    pub marker: Option<Marker>,
    pub children: Vec<PlainNodeEntry>,
}
