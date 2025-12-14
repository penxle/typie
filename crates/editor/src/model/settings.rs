use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tsify::Tsify;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Codec, Tsify)]
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
    Continuous { max_width: f32, page_margin: f32 },
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Paginated {
            page_width: 794.0,
            page_height: 1123.0,
            page_margin_top: 96.0,
            page_margin_bottom: 96.0,
            page_margin_left: 96.0,
            page_margin_right: 96.0,
        }
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
            LayoutMode::Continuous {
                max_width,
                page_margin,
            } => {
                max_width.to_bits().hash(state);
                page_margin.to_bits().hash(state);
            }
        }
    }
}

#[derive(Debug, Clone, Codec)]
pub struct DocumentSettings {
    pub block_gap: f32,
    pub paragraph_indent: f32,
    pub layout_mode: LayoutMode,
}

impl DocumentSettings {
    pub fn new() -> Self {
        Self {
            block_gap: 1.0,
            paragraph_indent: 1.0,
            layout_mode: LayoutMode::default(),
        }
    }
}

impl Hash for DocumentSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.block_gap.to_bits().hash(state);
        self.paragraph_indent.to_bits().hash(state);
        self.layout_mode.hash(state);
    }
}
