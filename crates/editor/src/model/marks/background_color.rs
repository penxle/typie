use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use crate::types::Theme;
use crate::utils::rgba_from_u32;
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct BackgroundColorMark {
    pub key: String,
}

impl Default for BackgroundColorMark {
    fn default() -> Self {
        Self {
            key: "default".to_string(),
        }
    }
}

impl MarkHtmlCodec for BackgroundColorMark {
    fn to_dom(&self) -> DomSpec {
        let [r, g, b, _] = rgba_from_u32(Theme::bg_color_rgba(&self.key).unwrap());
        DomSpec::el("span")
            .style(format!("background-color:#{:02x}{:02x}{:02x}", r, g, b))
            .data("bg-color-key", &self.key)
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_data("data-bg-color-key", |elem| {
                elem.value()
                    .attr("data-bg-color-key")
                    .filter(|k| Theme::is_valid_bg_color_key(k))
                    .map(|k| Mark::BackgroundColor(BackgroundColorMark { key: k.into() }))
            }),
            MarkParseRule::from_style("background-color", |elem| {
                if elem.value().attr("data-bg-color-key").is_some() {
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
                        .map(|key| Mark::BackgroundColor(BackgroundColorMark { key }))
                })
            }),
        ]
    }
}
