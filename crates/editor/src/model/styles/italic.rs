use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ItalicStyle;

impl StyleHtmlCodec for ItalicStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("em").hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![
            StyleParseRule::from_tag("i", |_| Some(Style::Italic(ItalicStyle))),
            StyleParseRule::from_tag("em", |_| Some(Style::Italic(ItalicStyle))),
            StyleParseRule::from_style("font-style", |elem| {
                elem.value().attr("style").and_then(|s| {
                    let m = parse_styles(s);
                    if m.get("font-style") == Some(&"italic".into()) {
                        Some(Style::Italic(ItalicStyle))
                    } else {
                        None
                    }
                })
            }),
        ]
    }
}
