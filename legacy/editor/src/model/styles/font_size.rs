use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_font_size, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontSizeStyle {
    /// pt × 100 (e.g. 16pt → 1600)
    pub size: u32,
}

impl Hash for FontSizeStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.size.hash(state);
    }
}

impl StyleHtmlCodec for FontSizeStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-size:{}pt", self.size as f32 / 100.0))
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![StyleParseRule::from_style("font-size", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("font-size")
                    .and_then(|fs| parse_font_size(fs))
                    .map(|size| {
                        Style::FontSize(FontSizeStyle {
                            size: (size * 100.0).round() as u32,
                        })
                    })
            })
        })]
    }
}
