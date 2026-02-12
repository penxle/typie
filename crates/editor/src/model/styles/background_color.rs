use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use crate::types::Theme;
use crate::utils::rgba_from_u32;
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct BackgroundColorStyle {
    pub color: String,
}

impl BackgroundColorStyle {
    pub const NONE: &'static str = "none";

    pub fn has_color(&self) -> bool {
        self.color != Self::NONE
    }
}

impl Default for BackgroundColorStyle {
    fn default() -> Self {
        Self {
            color: Self::NONE.to_string(),
        }
    }
}

impl StyleHtmlCodec for BackgroundColorStyle {
    fn to_dom(&self) -> DomSpec {
        let key = &self.color;
        let [r, g, b, _] = rgba_from_u32(Theme::bg_color_rgba(key).unwrap());
        DomSpec::el("span")
            .style(format!("background-color:#{:02x}{:02x}{:02x}", r, g, b))
            .data("bg-color", key)
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_data("data-bg-color", |elem| {
                elem.value()
                    .attr("data-bg-color")
                    .filter(|k| Theme::is_valid_bg_color_key(k))
                    .map(|k| Style::BackgroundColor(BackgroundColorStyle { color: k.into() }))
            }),
            StyleParseRule::from_style("background-color", |elem| {
                if elem.value().attr("data-bg-color").is_some() {
                    return None;
                }
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    m.get("background-color")
                        .and_then(|bg| {
                            if Theme::is_valid_bg_color_key(bg) {
                                Some(bg.clone())
                            } else {
                                Theme::nearest_bg_color(bg).map(String::from)
                            }
                        })
                        .map(|key| Style::BackgroundColor(BackgroundColorStyle { color: key }))
                })
            }),
        ]
    }
}
