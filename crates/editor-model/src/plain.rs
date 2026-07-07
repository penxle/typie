use std::collections::BTreeMap;

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::{Modifier, ModifierType, PlainNode, PlainRootNode};

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
                carry: Vec::new(),
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
    pub carry: Vec<Modifier>,
    pub children: Vec<PlainNodeEntry>,
}
