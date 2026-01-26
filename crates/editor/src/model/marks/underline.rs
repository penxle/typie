use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct UnderlineMark;

impl MarkHtmlCodec for UnderlineMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("u").hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_tag("u", |_| Some(Mark::Underline(UnderlineMark))),
            MarkParseRule::from_style("text-decoration", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    if let Some(td) = m.get("text-decoration") {
                        if td.contains("underline") {
                            return Some(Mark::Underline(UnderlineMark));
                        }
                    }
                    None
                })
            }),
        ]
    }
}
