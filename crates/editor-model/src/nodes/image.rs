use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageNode {
    pub id: Option<String>,
    #[ffi(default = "1.0f")]
    #[serde(default = "default_proportion")]
    pub proportion: f32,
}

fn default_proportion() -> f32 {
    1.0
}

impl Default for ImageNode {
    fn default() -> Self {
        Self {
            id: None,
            proportion: default_proportion(),
        }
    }
}
