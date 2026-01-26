use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct StrikethroughMark;

impl MarkHtmlCodec for StrikethroughMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("s").hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_tag("s", |_| Some(Mark::Strikethrough(StrikethroughMark))),
            MarkParseRule::from_tag("strike", |_| Some(Mark::Strikethrough(StrikethroughMark))),
            MarkParseRule::from_tag("del", |_| Some(Mark::Strikethrough(StrikethroughMark))),
            MarkParseRule::from_style("text-decoration", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    if let Some(td) = m.get("text-decoration") {
                        if td.contains("line-through") {
                            return Some(Mark::Strikethrough(StrikethroughMark));
                        }
                    }
                    None
                })
            }),
        ]
    }
}
