use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableNode {
    #[serde(default)]
    pub border_style: TableBorderStyle,
    #[ffi(default = "1.0f")]
    #[serde(default = "default_proportion")]
    pub proportion: f32,
}

fn default_proportion() -> f32 {
    1.0
}

impl Default for TableNode {
    fn default() -> Self {
        Self {
            border_style: TableBorderStyle::default(),
            proportion: default_proportion(),
        }
    }
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableBorderStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    None,
}
