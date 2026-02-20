use crate::model::attr::{Attr, ParagraphAttr};
use crate::model::style::Style;
use crate::model::styles::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi))]
#[serde(transparent)]
pub struct DefaultAttrs(Vec<Attr>);

const DEFAULT_FONT_FAMILY: &str = "Pretendard";
const DEFAULT_FONT_SIZE: u32 = 1200;
const DEFAULT_FONT_WEIGHT: u16 = 400;
const DEFAULT_TEXT_COLOR: &str = "black";
const DEFAULT_LETTER_SPACING: i32 = 0;
const DEFAULT_LINE_HEIGHT: u32 = 160;

impl Default for DefaultAttrs {
    fn default() -> Self {
        Self(vec![
            Attr::Style(Style::FontFamily(FontFamilyStyle {
                family: DEFAULT_FONT_FAMILY.to_string(),
            })),
            Attr::Style(Style::FontSize(FontSizeStyle {
                size: DEFAULT_FONT_SIZE,
            })),
            Attr::Style(Style::FontWeight(FontWeightStyle {
                weight: DEFAULT_FONT_WEIGHT,
            })),
            Attr::Style(Style::TextColor(TextColorStyle {
                color: DEFAULT_TEXT_COLOR.to_string(),
            })),
            Attr::Style(Style::BackgroundColor(BackgroundColorStyle {
                color: BackgroundColorStyle::NONE.to_string(),
            })),
            Attr::Style(Style::LetterSpacing(LetterSpacingStyle {
                spacing: DEFAULT_LETTER_SPACING,
            })),
            Attr::Paragraph(ParagraphAttr {
                line_height: DEFAULT_LINE_HEIGHT,
            }),
        ])
    }
}

impl DefaultAttrs {
    pub fn font_family(&self) -> &str {
        for attr in &self.0 {
            if let Attr::Style(Style::FontFamily(s)) = attr {
                return &s.family;
            }
        }
        DEFAULT_FONT_FAMILY
    }

    pub fn font_weight(&self) -> u16 {
        for attr in &self.0 {
            if let Attr::Style(Style::FontWeight(s)) = attr {
                return s.weight;
            }
        }
        DEFAULT_FONT_WEIGHT
    }

    pub fn text_color(&self) -> &str {
        for attr in &self.0 {
            if let Attr::Style(Style::TextColor(s)) = attr {
                return &s.color;
            }
        }
        DEFAULT_TEXT_COLOR
    }

    pub fn attrs(&self) -> &[Attr] {
        &self.0
    }

    pub fn to_styles(&self) -> Vec<Style> {
        Attr::extract_styles(&self.0)
    }

    pub fn to_attrs(&self) -> Vec<Attr> {
        self.0.clone()
    }

    pub fn from_attrs(attrs: &[Attr]) -> Self {
        Self(attrs.to_vec())
    }
}
