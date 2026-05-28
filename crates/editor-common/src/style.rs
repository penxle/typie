use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnderlineStyle {
    Solid,
    Dashed,
    Wavy,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Underline {
    pub color: String,
    pub style: UnderlineStyle,
    pub thickness: f32,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DecorationStyle {
    pub background: Option<String>,
    #[serde(default)]
    pub background_radius: Option<f32>,
    #[serde(default)]
    pub background_inset: Option<f32>,
    pub underline: Option<Underline>,
}
