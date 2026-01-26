use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule, parse_font_size, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontSizeMark {
    pub size: f32,
}

impl Hash for FontSizeMark {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.size.to_bits().hash(state);
    }
}

impl Default for FontSizeMark {
    fn default() -> Self {
        Self { size: 12.0 }
    }
}

impl MarkHtmlCodec for FontSizeMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-size:{}pt", self.size))
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![MarkParseRule::from_style("font-size", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("font-size")
                    .and_then(|fs| parse_font_size(fs))
                    .map(|size| Mark::FontSize(FontSizeMark { size }))
            })
        })]
    }
}
