use crate::model::style::Style;
use crate::model::styles::*;
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DefaultStyles {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub text_color: String,
    pub background_color: String,
    pub letter_spacing: f32,
    pub line_height: f32,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
}

impl Default for DefaultStyles {
    fn default() -> Self {
        Self {
            font_family: "Pretendard".to_string(),
            font_size: 12.0,
            font_weight: 400,
            text_color: "black".to_string(),
            background_color: BackgroundColorStyle::NONE.to_string(),
            letter_spacing: 0.0,
            line_height: 1.6,
            italic: false,
            strikethrough: false,
            underline: false,
        }
    }
}

impl DefaultStyles {
    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    pub fn font_weight(&self) -> u16 {
        self.font_weight
    }

    pub fn text_color(&self) -> &str {
        &self.text_color
    }

    pub fn to_styles(&self) -> Vec<Style> {
        let mut styles = vec![
            Style::FontFamily(FontFamilyStyle {
                family: self.font_family.clone(),
            }),
            Style::FontSize(FontSizeStyle {
                size: self.font_size,
            }),
            Style::FontWeight(FontWeightStyle {
                weight: self.font_weight,
            }),
            Style::TextColor(TextColorStyle {
                color: self.text_color.clone(),
            }),
            Style::BackgroundColor(BackgroundColorStyle {
                color: self.background_color.clone(),
            }),
            Style::LetterSpacing(LetterSpacingStyle {
                spacing: self.letter_spacing,
            }),
        ];
        if self.italic {
            styles.push(Style::Italic(ItalicStyle {}));
        }
        if self.strikethrough {
            styles.push(Style::Strikethrough(StrikethroughStyle {}));
        }
        if self.underline {
            styles.push(Style::Underline(UnderlineStyle {}));
        }
        styles
    }
}
