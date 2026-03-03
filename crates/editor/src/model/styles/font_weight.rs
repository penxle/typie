use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontWeightStyle {
    pub weight: u16,
}

/// 주어진 후보 중 target에 가장 가까운 weight를 반환한다.
/// 거리가 동일하면 더 높은 weight를 선택한다.
/// NOTE: 웹 로더 쪽도 동일 규칙(가까운 값 + 동점 시 더 무거운 weight)이어야 한다.
/// - apps/website/src/lib/editor/editor.svelte.ts (#handleFontRequired)
/// - apps/website/src/lib/editor/fonts.ts (ensureRequiredFallbackFont)
pub fn nearest_weight(weights: &[u16], target: u16) -> u16 {
    weights
        .iter()
        .copied()
        .min_by(|&a, &b| {
            let da = (a as i32 - target as i32).abs();
            let db = (b as i32 - target as i32).abs();
            da.cmp(&db).then(b.cmp(&a))
        })
        .unwrap_or(target)
}

impl StyleHtmlCodec for FontWeightStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style(format!("font-weight:{}", self.weight))
            .hole()
    }

    fn parse_rules() -> Vec<StyleParseRule> {
        vec![StyleParseRule::from_style("font-weight", |elem| {
            elem.value().attr("style").and_then(|s| {
                let m = parse_styles(s);
                m.get("font-weight").and_then(|fw| {
                    fw.parse::<u16>()
                        .ok()
                        .map(|weight| Style::FontWeight(FontWeightStyle { weight }))
                })
            })
        })]
    }
}
