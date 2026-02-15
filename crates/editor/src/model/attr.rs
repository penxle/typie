use crate::model::style::Style;
use loro::LoroValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphAttr {
    pub line_height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "attr", rename_all = "snake_case")]
pub enum Attr {
    Style(Style),
    Paragraph(ParagraphAttr),
}

impl Attr {
    /// Style → style.key() (e.g. "style:font_family"), Paragraph → "paragraph:line_height"
    pub fn key(&self) -> &'static str {
        match self {
            Attr::Style(s) => s.key(),
            Attr::Paragraph(_) => "paragraph:line_height",
        }
    }

    pub fn to_loro_value(&self) -> LoroValue {
        match self {
            Attr::Style(s) => s.to_loro_value(),
            Attr::Paragraph(p) => LoroValue::Double(p.line_height as f64),
        }
    }

    pub fn from_key_value(key: &str, value: LoroValue) -> Option<Self> {
        if key == "paragraph:line_height" {
            match value {
                LoroValue::Double(v) => Some(Attr::Paragraph(ParagraphAttr {
                    line_height: v as f32,
                })),
                LoroValue::I64(v) => Some(Attr::Paragraph(ParagraphAttr {
                    line_height: v as f32,
                })),
                _ => None,
            }
        } else {
            Style::from_key_value(key, value).map(Attr::Style)
        }
    }

    pub fn from_styles(styles: &[Style]) -> Vec<Attr> {
        styles.iter().map(|s| Attr::Style(s.clone())).collect()
    }

    pub fn extract_styles(attrs: &[Attr]) -> Vec<Style> {
        attrs
            .iter()
            .filter_map(|a| match a {
                Attr::Style(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn extract_paragraph_attr(attrs: &[Attr]) -> Option<ParagraphAttr> {
        attrs.iter().find_map(|a| match a {
            Attr::Paragraph(p) => Some(p.clone()),
            _ => None,
        })
    }
}
