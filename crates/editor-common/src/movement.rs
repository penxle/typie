use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::geometry::Axis;

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Backward,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Movement {
    Grapheme(Direction),
    Word(Direction),
    Sentence(Direction),
    Line(Direction, Axis),
    Block(Direction),
    Page(Direction),
    Document(Direction),
}
