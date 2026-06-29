use std::collections::{BTreeMap, BTreeSet};

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::{Marker, Modifier, ModifierType, PlainNode, PlainRootNode};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainDoc {
    pub root: PlainNodeEntry,
    #[serde(default)]
    pub styles: BTreeMap<String, PlainStyleEntry>,
}

impl Default for PlainDoc {
    fn default() -> Self {
        Self {
            root: PlainNodeEntry {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: BTreeMap::new(),
                style: None,
                marker: None,
                children: Vec::new(),
            },
            styles: BTreeMap::new(),
        }
    }
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlainStyleEntry {
    pub name: String,
    pub modifiers: BTreeSet<Modifier>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainNodeEntry {
    pub node: PlainNode,
    pub modifiers: BTreeMap<ModifierType, Modifier>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub marker: Option<Marker>,
    pub children: Vec<PlainNodeEntry>,
}
