use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontWeightMark {
    pub weight: u16,
}

impl Default for FontWeightMark {
    fn default() -> Self {
        Self { weight: 400 }
    }
}

impl MarkHtmlCodec for FontWeightMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-weight:{}", self.weight))
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![
            MarkParseRule::from_tag("b", |_| {
                Some(Mark::FontWeight(FontWeightMark { weight: 700 }))
            }),
            MarkParseRule::from_tag("strong", |_| {
                Some(Mark::FontWeight(FontWeightMark { weight: 700 }))
            }),
            MarkParseRule::from_style("font-weight", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    m.get("font-weight").and_then(|fw| {
                        if let Ok(w) = fw.parse::<u16>() {
                            Some(Mark::FontWeight(FontWeightMark { weight: w }))
                        } else if fw == "bold" {
                            Some(Mark::FontWeight(FontWeightMark { weight: 700 }))
                        } else {
                            None
                        }
                    })
                })
            }),
        ]
    }
}
