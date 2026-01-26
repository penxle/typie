use crate::model::html::{DomSpec, MarkHtmlCodec};
use crate::model::marks::*;
use macros::{Codec, LoroMark};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum MarkType {
    BackgroundColor,
    TextColor,
    FontSize,
    FontFamily,
    FontWeight,
    Italic,
    LetterSpacing,
    Link,
    Ruby,
    Strikethrough,
    Underline,
}

impl MarkType {
    pub const fn all() -> [MarkType; 11] {
        [
            MarkType::BackgroundColor,
            MarkType::TextColor,
            MarkType::FontSize,
            MarkType::FontFamily,
            MarkType::FontWeight,
            MarkType::Italic,
            MarkType::LetterSpacing,
            MarkType::Link,
            MarkType::Ruby,
            MarkType::Strikethrough,
            MarkType::Underline,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec, LoroMark)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mark {
    BackgroundColor(BackgroundColorMark),
    TextColor(TextColorMark),
    FontSize(FontSizeMark),
    FontFamily(FontFamilyMark),
    FontWeight(FontWeightMark),
    Italic(ItalicMark),
    LetterSpacing(LetterSpacingMark),
    Link(LinkMark),
    Ruby(RubyMark),
    Strikethrough(StrikethroughMark),
    Underline(UnderlineMark),
}

impl Mark {
    pub fn as_type(&self) -> MarkType {
        match self {
            Mark::BackgroundColor(_) => MarkType::BackgroundColor,
            Mark::TextColor(_) => MarkType::TextColor,
            Mark::FontSize(_) => MarkType::FontSize,
            Mark::FontFamily(_) => MarkType::FontFamily,
            Mark::FontWeight(_) => MarkType::FontWeight,
            Mark::Italic(_) => MarkType::Italic,
            Mark::LetterSpacing(_) => MarkType::LetterSpacing,
            Mark::Link(_) => MarkType::Link,
            Mark::Ruby(_) => MarkType::Ruby,
            Mark::Strikethrough(_) => MarkType::Strikethrough,
            Mark::Underline(_) => MarkType::Underline,
        }
    }

    pub fn is_default(&self) -> bool {
        match self {
            Mark::BackgroundColor(m) => m == &BackgroundColorMark::default(),
            Mark::TextColor(m) => m == &TextColorMark::default(),
            Mark::FontSize(m) => m == &FontSizeMark::default(),
            Mark::FontFamily(m) => m == &FontFamilyMark::default(),
            Mark::FontWeight(m) => m == &FontWeightMark::default(),
            Mark::Italic(_) => false,
            Mark::LetterSpacing(m) => m == &LetterSpacingMark::default(),
            Mark::Link(m) => m == &LinkMark::default(),
            Mark::Ruby(m) => m == &RubyMark::default(),
            Mark::Strikethrough(_) => false,
            Mark::Underline(_) => false,
        }
    }
}

impl Hash for Mark {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Mark::BackgroundColor(m) => m.hash(state),
            Mark::TextColor(m) => m.hash(state),
            Mark::FontSize(m) => m.hash(state),
            Mark::FontFamily(m) => m.hash(state),
            Mark::FontWeight(m) => m.hash(state),
            Mark::Italic(m) => m.hash(state),
            Mark::LetterSpacing(m) => m.hash(state),
            Mark::Link(m) => m.hash(state),
            Mark::Ruby(m) => m.hash(state),
            Mark::Strikethrough(m) => m.hash(state),
            Mark::Underline(m) => m.hash(state),
        }
    }
}

impl MarkHtmlCodec for Mark {
    fn to_dom(&self) -> DomSpec {
        match self {
            Mark::BackgroundColor(m) => m.to_dom(),
            Mark::TextColor(m) => m.to_dom(),
            Mark::FontSize(m) => m.to_dom(),
            Mark::FontFamily(m) => m.to_dom(),
            Mark::FontWeight(m) => m.to_dom(),
            Mark::Italic(m) => m.to_dom(),
            Mark::LetterSpacing(m) => m.to_dom(),
            Mark::Link(m) => m.to_dom(),
            Mark::Ruby(m) => m.to_dom(),
            Mark::Strikethrough(m) => m.to_dom(),
            Mark::Underline(m) => m.to_dom(),
        }
    }
}
