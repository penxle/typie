use editor_crdt::{CrdtError, Dot};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextNode;

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
pub enum TextNodeAttr {}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlainTextNode {
    pub text: String,
}

impl TextNode {
    pub fn to_plain(&self) -> PlainTextNode {
        PlainTextNode {
            text: String::new(),
        }
    }

    pub fn apply_attr(&mut self, _id: Dot, attr: &TextNodeAttr) -> Result<(), CrdtError> {
        match *attr {}
    }
}

impl PlainTextNode {
    pub fn to_attrs(&self) -> Vec<TextNodeAttr> {
        Vec::new()
    }
}
