use crate::model::Style;
use crate::model::html::{
    DomSpec, LengthUnit, StyleHtmlCodec, StyleParseRule, parse_as, parse_styles,
};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LetterSpacingStyle {
    /// em × 100 (e.g. 0.05em → 5)
    pub spacing: i32,
}

impl Hash for LetterSpacingStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.spacing.hash(state);
    }
}

const LETTER_SPACINGS: &[i32] = &[-10, -5, 0, 5, 10, 20, 40];

fn snap_letter_spacing(v: i32) -> i32 {
    let mut best = LETTER_SPACINGS[0];
    let mut best_dist = i32::MAX;
    for &ls in LETTER_SPACINGS {
        let d = (v - ls).abs();
        if d < best_dist {
            best_dist = d;
            best = ls;
        }
    }
    best
}

impl StyleHtmlCodec for LetterSpacingStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("letter-spacing:{}em", self.spacing as f32 / 100.0))
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![StyleParseRule::from_style("letter-spacing", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("letter-spacing")
                    .and_then(|ls| parse_as(ls, LengthUnit::Em))
                    .map(|spacing| {
                        Style::LetterSpacing(LetterSpacingStyle {
                            spacing: snap_letter_spacing((spacing * 100.0).round() as i32),
                        })
                    })
            })
        })]
    }
}
