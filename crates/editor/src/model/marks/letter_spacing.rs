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

const LETTER_SPACINGS: &[f32] = &[-0.1, -0.05, 0.0, 0.05, 0.1, 0.2, 0.4];

fn snap_letter_spacing(v: f32) -> f32 {
    let mut best = LETTER_SPACINGS[0];
    let mut best_dist = f32::MAX;
    for &ls in LETTER_SPACINGS {
        let d = (v - ls).abs();
        if d < best_dist {
            best_dist = d;
            best = ls;
        }
    }
    best
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
                    .map(|spacing| {
                        Mark::LetterSpacing(LetterSpacingMark {
                            spacing: snap_letter_spacing(spacing),
                        })
                    })
            })
        })]
    }
}
