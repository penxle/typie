use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceLayer {
    Background,
    BelowMarks,
    Content,
    AboveMarks,
}

impl SurfaceLayer {
    pub const ALL: [SurfaceLayer; 4] = [
        SurfaceLayer::Background,
        SurfaceLayer::BelowMarks,
        SurfaceLayer::Content,
        SurfaceLayer::AboveMarks,
    ];

    pub const fn bit(self) -> u8 {
        1 << self as u8
    }
}

pub fn layers_to_mask(layers: &[SurfaceLayer]) -> u8 {
    layers.iter().fold(0u8, |m, l| m | l.bit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_layer_bits() {
        assert_eq!(SurfaceLayer::Background.bit(), 1);
        assert_eq!(SurfaceLayer::BelowMarks.bit(), 2);
        assert_eq!(SurfaceLayer::Content.bit(), 4);
        assert_eq!(SurfaceLayer::AboveMarks.bit(), 8);
        assert_eq!(layers_to_mask(&SurfaceLayer::ALL), 0b1111);
    }
}
