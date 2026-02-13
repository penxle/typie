use crate::font::get_available_fonts;
use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontFamilyStyle {
    pub family: String,
}

impl StyleHtmlCodec for FontFamilyStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-family:{}", self.family))
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![StyleParseRule::from_style("font-family", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("font-family").and_then(|ff| {
                    let family: String = ff.trim_matches(|c| c == '"' || c == '\'').into();
                    let available = get_available_fonts();
                    if available.contains_key(&family) {
                        Some(Style::FontFamily(FontFamilyStyle { family }))
                    } else {
                        None
                    }
                })
            })
        })]
    }
}
