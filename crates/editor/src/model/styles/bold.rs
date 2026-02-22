use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct BoldStyle {}

impl StyleHtmlCodec for BoldStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style("font-weight:bold".to_string())
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_tag("b", |_| Some(Style::Bold(BoldStyle {}))),
            StyleParseRule::from_tag("strong", |_| Some(Style::Bold(BoldStyle {}))),
            StyleParseRule::from_style("font-weight", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    m.get("font-weight").and_then(|fw| {
                        if fw.eq_ignore_ascii_case("bold") {
                            Some(Style::Bold(BoldStyle {}))
                        } else {
                            None
                        }
                    })
                })
            }),
        ]
    }
}
