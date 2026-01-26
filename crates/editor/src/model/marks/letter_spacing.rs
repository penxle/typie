use crate::model::Mark;
use crate::model::html::{
    DomSpec, LengthUnit, MarkHtmlCodec, MarkParseRule, parse_as, parse_styles,
};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LetterSpacingMark {
    pub spacing: f32,
}

impl Hash for LetterSpacingMark {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.spacing.to_bits().hash(state);
    }
}

impl Default for LetterSpacingMark {
    fn default() -> Self {
        Self { spacing: 0.0 }
    }
}

impl MarkHtmlCodec for LetterSpacingMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("letter-spacing:{}em", self.spacing))
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![MarkParseRule::from_style("letter-spacing", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("letter-spacing")
                    .and_then(|ls| parse_as(ls, LengthUnit::Em))
                    .map(|spacing| Mark::LetterSpacing(LetterSpacingMark { spacing }))
            })
        })]
    }
}
