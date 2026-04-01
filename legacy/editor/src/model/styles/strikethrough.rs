use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct StrikethroughStyle {}

impl StyleHtmlCodec for StrikethroughStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("s").hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_tag("s", |_| Some(Style::Strikethrough(StrikethroughStyle {}))),
            StyleParseRule::from_tag("strike", |_| {
                Some(Style::Strikethrough(StrikethroughStyle {}))
            }),
            StyleParseRule::from_tag("del", |_| Some(Style::Strikethrough(StrikethroughStyle {}))),
            StyleParseRule::from_style("text-decoration", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    if let Some(td) = m.get("text-decoration") {
                        if td.contains("line-through") {
                            return Some(Style::Strikethrough(StrikethroughStyle {}));
                        }
                    }
                    None
                })
            }),
        ]
    }
}
