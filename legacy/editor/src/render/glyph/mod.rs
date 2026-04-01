mod internal;
pub mod outline;
pub mod rasterize;
pub mod scale;

use std::hash::{Hash, Hasher};

pub const EMBOLDEN_RATIO: f32 = 1.0 / 64.0;

#[derive(Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

pub fn calculate_font_hash(font_data: &[u8]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    font_data.as_ptr().hash(&mut hasher);
    font_data.len().hash(&mut hasher);
    crate::global::font_version(font_data.as_ptr()).hash(&mut hasher);
    hasher.finish()
}
