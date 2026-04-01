use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

pub const CONTINUOUS_PAGE_MARGIN: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LayoutMode {
    #[serde(rename_all = "camelCase")]
    Paginated {
        page_width: f32,
        page_height: f32,
        page_margin_top: f32,
        page_margin_bottom: f32,
        page_margin_left: f32,
        page_margin_right: f32,
    },
    #[serde(rename_all = "camelCase")]
    Continuous { max_width: f32 },
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Continuous { max_width: 600.0 }
    }
}

impl Hash for LayoutMode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => {
                page_width.to_bits().hash(state);
                page_height.to_bits().hash(state);
                page_margin_top.to_bits().hash(state);
                page_margin_bottom.to_bits().hash(state);
                page_margin_left.to_bits().hash(state);
                page_margin_right.to_bits().hash(state);
            }
            LayoutMode::Continuous { max_width } => {
                max_width.to_bits().hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Codec)]
pub struct DocumentSettings {
    /// × 100 (e.g. 100% → 100)
    pub block_gap: u32,
    /// × 100 (e.g. 100% → 100)
    pub paragraph_indent: u32,
    pub layout_mode: LayoutMode,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            block_gap: 100,
            paragraph_indent: 100,
            layout_mode: LayoutMode::default(),
        }
    }
}


impl Hash for DocumentSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.block_gap.hash(state);
        self.paragraph_indent.hash(state);
        self.layout_mode.hash(state);
    }
}
