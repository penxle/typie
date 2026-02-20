use crate::model::Codec;
use crate::model::html::{DomSpec, StyleHtmlCodec};
use crate::model::styles::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum StyleType {
    BackgroundColor,
    Bold,
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
            StyleType::Bold,
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
            StyleType::Bold => "style:bold",
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
    Bold(BoldStyle),
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
            Style::Bold(_) => StyleType::Bold,
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
        match self {
            Style::BackgroundColor(inner) => inner.to_value().expect("style must serialize"),
            Style::Bold(inner) => inner.to_value().expect("style must serialize"),
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
        let style_key = key.strip_prefix("style:")?;
        match style_key {
            "background_color" => BackgroundColorStyle::from_value(value)
                .ok()
                .map(Style::BackgroundColor),
            "bold" => BoldStyle::from_value(value).ok().map(Style::Bold),
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

impl StyleHtmlCodec for Style {
    fn to_dom(&self) -> DomSpec {
        match self {
            Style::BackgroundColor(s) => StyleHtmlCodec::to_dom(s),
            Style::Bold(s) => StyleHtmlCodec::to_dom(s),
            Style::TextColor(s) => StyleHtmlCodec::to_dom(s),
            Style::FontSize(s) => StyleHtmlCodec::to_dom(s),
            Style::FontFamily(s) => StyleHtmlCodec::to_dom(s),
            Style::FontWeight(s) => StyleHtmlCodec::to_dom(s),
            Style::Italic(s) => StyleHtmlCodec::to_dom(s),
            Style::LetterSpacing(s) => StyleHtmlCodec::to_dom(s),
            Style::Strikethrough(s) => StyleHtmlCodec::to_dom(s),
            Style::Underline(s) => StyleHtmlCodec::to_dom(s),
        }
    }
}

impl Eq for Style {}

impl Hash for Style {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Style::BackgroundColor(s) => s.hash(state),
            Style::Bold(s) => s.hash(state),
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
