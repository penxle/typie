use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

use super::styles::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum StyleType {
    BackgroundColor,
    TextColor,
    FontSize,
    FontFamily,
    FontWeight,
    Italic,
    LetterSpacing,
    Strikethrough,
    Underline,
}

impl StyleType {
    pub fn all() -> &'static [StyleType] {
        &[
            StyleType::BackgroundColor,
            StyleType::TextColor,
            StyleType::FontSize,
            StyleType::FontFamily,
            StyleType::FontWeight,
            StyleType::Italic,
            StyleType::LetterSpacing,
            StyleType::Strikethrough,
            StyleType::Underline,
        ]
    }

    pub fn key(&self) -> &'static str {
        match self {
            StyleType::BackgroundColor => "style:background_color",
            StyleType::TextColor => "style:text_color",
            StyleType::FontSize => "style:font_size",
            StyleType::FontFamily => "style:font_family",
            StyleType::FontWeight => "style:font_weight",
            StyleType::Italic => "style:italic",
            StyleType::LetterSpacing => "style:letter_spacing",
            StyleType::Strikethrough => "style:strikethrough",
            StyleType::Underline => "style:underline",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Style {
    BackgroundColor(BackgroundColorStyle),
    TextColor(TextColorStyle),
    FontSize(FontSizeStyle),
    FontFamily(FontFamilyStyle),
    FontWeight(FontWeightStyle),
    Italic(ItalicStyle),
    LetterSpacing(LetterSpacingStyle),
    Strikethrough(StrikethroughStyle),
    Underline(UnderlineStyle),
}

impl Style {
    pub fn as_type(&self) -> StyleType {
        match self {
            Style::BackgroundColor(_) => StyleType::BackgroundColor,
            Style::TextColor(_) => StyleType::TextColor,
            Style::FontSize(_) => StyleType::FontSize,
            Style::FontFamily(_) => StyleType::FontFamily,
            Style::FontWeight(_) => StyleType::FontWeight,
            Style::Italic(_) => StyleType::Italic,
            Style::LetterSpacing(_) => StyleType::LetterSpacing,
            Style::Strikethrough(_) => StyleType::Strikethrough,
            Style::Underline(_) => StyleType::Underline,
        }
    }

    pub fn key(&self) -> &'static str {
        self.as_type().key()
    }

    pub fn to_loro_value(&self) -> loro::LoroValue {
        use crate::model::Codec;
        match self {
            Style::BackgroundColor(inner) => inner.to_value().expect("style must serialize"),
            Style::TextColor(inner) => inner.to_value().expect("style must serialize"),
            Style::FontSize(inner) => inner.to_value().expect("style must serialize"),
            Style::FontFamily(inner) => inner.to_value().expect("style must serialize"),
            Style::FontWeight(inner) => inner.to_value().expect("style must serialize"),
            Style::Italic(inner) => inner.to_value().expect("style must serialize"),
            Style::LetterSpacing(inner) => inner.to_value().expect("style must serialize"),
            Style::Strikethrough(inner) => inner.to_value().expect("style must serialize"),
            Style::Underline(inner) => inner.to_value().expect("style must serialize"),
        }
    }

    pub fn from_key_value(key: &str, value: loro::LoroValue) -> Option<Self> {
        use crate::model::Codec;
        let style_key = key.strip_prefix("style:")?;
        match style_key {
            "background_color" => BackgroundColorStyle::from_value(value)
                .ok()
                .map(Style::BackgroundColor),
            "text_color" => TextColorStyle::from_value(value).ok().map(Style::TextColor),
            "font_size" => FontSizeStyle::from_value(value).ok().map(Style::FontSize),
            "font_family" => FontFamilyStyle::from_value(value)
                .ok()
                .map(Style::FontFamily),
            "font_weight" => FontWeightStyle::from_value(value)
                .ok()
                .map(Style::FontWeight),
            "italic" => ItalicStyle::from_value(value).ok().map(Style::Italic),
            "letter_spacing" => LetterSpacingStyle::from_value(value)
                .ok()
                .map(Style::LetterSpacing),
            "strikethrough" => StrikethroughStyle::from_value(value)
                .ok()
                .map(Style::Strikethrough),
            "underline" => UnderlineStyle::from_value(value).ok().map(Style::Underline),
            _ => None,
        }
    }
}

impl crate::model::html::StyleHtmlCodec for Style {
    fn to_dom(&self) -> crate::model::html::DomSpec {
        match self {
            Style::BackgroundColor(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::TextColor(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::FontSize(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::FontFamily(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::FontWeight(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::Italic(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::LetterSpacing(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::Strikethrough(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
            Style::Underline(s) => crate::model::html::StyleHtmlCodec::to_dom(s),
        }
    }
}

impl Eq for Style {}

impl Hash for Style {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Style::BackgroundColor(s) => s.hash(state),
            Style::TextColor(s) => s.hash(state),
            Style::FontSize(s) => s.hash(state),
            Style::FontFamily(s) => s.hash(state),
            Style::FontWeight(s) => s.hash(state),
            Style::Italic(s) => s.hash(state),
            Style::LetterSpacing(s) => s.hash(state),
            Style::Strikethrough(s) => s.hash(state),
            Style::Underline(s) => s.hash(state),
        }
    }
}
