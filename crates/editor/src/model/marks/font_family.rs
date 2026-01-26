use crate::global::get_available_fonts;
use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontFamilyMark {
    pub family: String,
}

impl Default for FontFamilyMark {
    fn default() -> Self {
        Self {
            family: "Pretendard".to_string(),
        }
    }
}

impl MarkHtmlCodec for FontFamilyMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-family:{}", self.family))
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![MarkParseRule::from_style("font-family", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("font-family").and_then(|ff| {
                    let family: String = ff.trim_matches(|c| c == '"' || c == '\'').into();
                    let available = get_available_fonts();
                    if available.contains_key(&family) {
                        Some(Mark::FontFamily(FontFamilyMark { family }))
                    } else {
                        None
                    }
                })
            })
        })]
    }
}
