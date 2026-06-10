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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Movement {
    Grapheme { direction: Direction },
    Word { direction: Direction },
    Sentence { direction: Direction },
    Line { direction: Direction, axis: Axis },
    Page { direction: Direction },
    Document { direction: Direction },
}
