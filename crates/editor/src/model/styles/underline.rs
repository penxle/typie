use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct UnderlineStyle;

impl StyleHtmlCodec for UnderlineStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("u").hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_tag("u", |_| Some(Style::Underline(UnderlineStyle))),
            StyleParseRule::from_style("text-decoration", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    if let Some(td) = m.get("text-decoration") {
                        if td.contains("underline") {
                            return Some(Style::Underline(UnderlineStyle));
                        }
                    }
                    None
                })
            }),
        ]
    }
}
