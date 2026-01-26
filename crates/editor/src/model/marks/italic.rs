use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ItalicMark;

impl MarkHtmlCodec for ItalicMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("em").hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_tag("i", |_| Some(Mark::Italic(ItalicMark))),
            MarkParseRule::from_tag("em", |_| Some(Mark::Italic(ItalicMark))),
            MarkParseRule::from_style("font-style", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    if m.get("font-style") == Some(&"italic".into()) {
                        Some(Mark::Italic(ItalicMark))
                    } else {
                        None
                    }
                })
            }),
        ]
    }
}
