use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FontFamilySource {
    Default,
    User,
    Fallback,
}

#[ffi]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontFamily {
    pub name: String,
    pub source: FontFamilySource,
    pub weights: Vec<FontWeight>,
}

/// chunk별 flat 정수 배열 `[start0, end0, start1, end1, ...]` (inclusive).
#[ffi]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontWeight {
    pub value: u16,
    pub hash: String,
    pub chunks: Vec<Vec<u32>>,
}
