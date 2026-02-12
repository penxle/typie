use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use crate::types::Theme;
use crate::utils::rgba_from_u32;
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TextColorStyle {
    pub color: String,
}

impl StyleHtmlCodec for TextColorStyle {
    fn to_dom(&self) -> DomSpec {
        let [r, g, b, _] = rgba_from_u32(Theme::text_color_rgba(&self.color).unwrap());
        DomSpec::el("span")
            .style(format!("color:#{:02x}{:02x}{:02x}", r, g, b))
            .data("text-color", &self.color)
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_data("data-text-color", |elem| {
                elem.value()
                    .attr("data-text-color")
                    .filter(|k| Theme::is_valid_text_color_key(k))
                    .map(|k| Style::TextColor(TextColorStyle { color: k.into() }))
            }),
            StyleParseRule::from_style("color", |elem| {
                if elem.value().attr("data-text-color").is_some() {
                    return None;
                }
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    m.get("color")
                        .and_then(|c| {
                            if Theme::is_valid_text_color_key(c) {
                                Some(c.clone())
                            } else {
                                Theme::nearest_text_color(c).map(String::from)
                            }
                        })
                        .map(|color| Style::TextColor(TextColorStyle { color }))
                })
            }),
        ]
    }
}
