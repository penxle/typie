use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
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
        DomSpec::el("span")
            .style(format!("color:var(--color-{})", &self.key))
            .data("text-color-key", &self.key)
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_data("data-text-color-key", |elem| {
                elem.value()
                    .attr("data-text-color-key")
                    .map(|k| Mark::TextColor(TextColorMark { key: k.into() }))
            }),
            MarkParseRule::from_style("color", |elem| {
                if elem.value().attr("data-text-color-key").is_some() {
                    return None;
                }
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    m.get("color")
                        .map(|c| Mark::TextColor(TextColorMark { key: c.clone() }))
                })
            }),
        ]
    }
}
