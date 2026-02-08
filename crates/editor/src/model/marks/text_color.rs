use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use crate::types::Theme;
use crate::utils::rgba_from_u32;
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TextColorMark {
    pub key: String,
}

impl Default for TextColorMark {
    fn default() -> Self {
        Self {
            key: "black".to_string(),
        }
    }
}

impl MarkHtmlCodec for TextColorMark {
    fn to_dom(&self) -> DomSpec {
        let [r, g, b, _] = rgba_from_u32(Theme::text_color_rgba(&self.key).unwrap());
        DomSpec::el("span")
            .style(format!("color:#{:02x}{:02x}{:02x}", r, g, b))
            .data("text-color-key", &self.key)
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_data("data-text-color-key", |elem| {
                elem.value()
                    .attr("data-text-color-key")
                    .filter(|k| Theme::is_valid_text_color_key(k))
                    .map(|k| Mark::TextColor(TextColorMark { key: k.into() }))
            }),
            MarkParseRule::from_style("color", |elem| {
                if elem.value().attr("data-text-color-key").is_some() {
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
                        .map(|key| Mark::TextColor(TextColorMark { key }))
                })
            }),
        ]
    }
}
