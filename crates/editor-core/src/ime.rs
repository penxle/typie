use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImeRange {
    pub start: usize,
    pub end: usize,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ime {
    pub text: String,
    pub window_start: usize,
    pub selection: ImeRange,
    pub composing: Option<ImeRange>,
}
