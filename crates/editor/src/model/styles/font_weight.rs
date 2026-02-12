use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontWeightStyle {
    pub weight: u16,
}

impl Default for FontWeightStyle {
    fn default() -> Self {
        Self { weight: 400 }
    }
}

impl StyleHtmlCodec for FontWeightStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-weight:{}", self.weight))
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_tag("b", |_| {
                Some(Style::FontWeight(FontWeightStyle { weight: 700 }))
            }),
            StyleParseRule::from_tag("strong", |_| {
                Some(Style::FontWeight(FontWeightStyle { weight: 700 }))
            }),
            StyleParseRule::from_style("font-weight", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    m.get("font-weight").and_then(|fw| {
                        if let Ok(w) = fw.parse::<u16>() {
                            Some(Style::FontWeight(FontWeightStyle { weight: w }))
                        } else if fw == "bold" {
                            Some(Style::FontWeight(FontWeightStyle { weight: 700 }))
                        } else {
                            None
                        }
                    })
                })
            }),
        ]
    }
}
