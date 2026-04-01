use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f64,
}

impl Viewport {
    pub fn new(width: f32, height: f32, scale_factor: f64) -> Self {
        Self {
            width,
            height,
            scale_factor,
        }
    }
}
