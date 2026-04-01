use crate::model::Style;
use crate::model::html::{DomSpec, StyleHtmlCodec, StyleParseRule, parse_styles};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FontWeightStyle {
    pub weight: u16,
}

/// CSS Fonts Level 4 §5.2 font-weight matching algorithm.
/// 방향성 우선 탐색: target 기준으로 선호 방향의 weight를 거리와 무관하게 우선 선택한다.
///
/// 동일 알고리즘이 아래 위치에도 구현되어 있으므로 함께 수정해야 한다:
/// - apps/api/src/export/core/fonts.ts (nearestWeight)
/// - apps/website/src/lib/editor/editor.svelte.ts (#handleFontRequired)
/// - apps/website/src/lib/editor/fonts.ts (ensureRequiredFallbackFont)
/// - apps/mobile/lib/screens/native_editor/state/fonts.dart (findFont, fallback)
pub fn nearest_weight(weights: &[u16], target: u16) -> u16 {
    if weights.is_empty() {
        return target;
    }

    let mut sorted = weights.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    if target >= 400 && target <= 500 {
        // Case 1: ascending from target to 500, then descending below target, then ascending above 500
        if let Some(&w) = sorted.iter().find(|&&w| w >= target && w <= 500) {
            return w;
        }
        if let Some(&w) = sorted.iter().rev().find(|&&w| w < target) {
            return w;
        }
        if let Some(&w) = sorted.iter().find(|&&w| w > 500) {
            return w;
        }
    } else if target < 400 {
        // Case 2: descending from target, then ascending above target
        if let Some(&w) = sorted.iter().rev().find(|&&w| w <= target) {
            return w;
        }
        if let Some(&w) = sorted.iter().find(|&&w| w > target) {
            return w;
        }
    } else {
        // Case 3 (target > 500): ascending from target, then descending below target
        if let Some(&w) = sorted.iter().find(|&&w| w >= target) {
            return w;
        }
        if let Some(&w) = sorted.iter().rev().find(|&&w| w < target) {
            return w;
        }
    }

    target
}

#[cfg(test)]
mod tests {
    use super::nearest_weight;

    // === Case 2: target < 400 (lighter-first) ===

    #[test]
    fn below_400_prefers_lighter() {
        assert_eq!(nearest_weight(&[100, 500], 300), 100);
    }

    #[test]
    fn below_400_exact_match() {
        assert_eq!(nearest_weight(&[100, 300, 500], 300), 300);
    }

    #[test]
    fn below_400_falls_back_to_heavier() {
        assert_eq!(nearest_weight(&[300, 500], 200), 300);
    }

    #[test]
    fn below_400_picks_closest_lighter() {
        assert_eq!(nearest_weight(&[200, 400], 350), 200);
    }

    // === Case 3: target > 500 (heavier-first) ===

    #[test]
    fn above_500_prefers_heavier() {
        assert_eq!(nearest_weight(&[400, 900], 600), 900);
    }

    #[test]
    fn above_500_exact_match() {
        assert_eq!(nearest_weight(&[400, 700, 900], 700), 700);
    }

    #[test]
    fn above_500_falls_back_to_lighter() {
        assert_eq!(nearest_weight(&[400, 600], 800), 600);
    }

    #[test]
    fn above_500_picks_closest_heavier() {
        assert_eq!(nearest_weight(&[400, 800], 700), 800);
    }

    // === Case 1: target ∈ [400, 500] ===

    #[test]
    fn mid_range_ascending_to_500() {
        assert_eq!(nearest_weight(&[400, 500], 450), 500);
    }

    #[test]
    fn mid_range_exact_400() {
        assert_eq!(nearest_weight(&[400, 700], 400), 400);
    }

    #[test]
    fn mid_range_exact_500() {
        assert_eq!(nearest_weight(&[300, 500], 500), 500);
    }

    #[test]
    fn mid_range_prefers_ascending_to_500_over_lighter() {
        assert_eq!(nearest_weight(&[300, 500], 400), 500);
    }

    #[test]
    fn mid_range_then_lighter_then_heavier() {
        assert_eq!(nearest_weight(&[200, 700], 450), 200);
    }

    #[test]
    fn mid_range_falls_back_to_above_500() {
        assert_eq!(nearest_weight(&[600, 800], 450), 600);
    }

    // === Boundary values ===

    #[test]
    fn boundary_400_no_match_in_range_prefers_lighter() {
        assert_eq!(nearest_weight(&[200, 600], 400), 200);
    }

    #[test]
    fn boundary_500_no_exact_prefers_lighter() {
        assert_eq!(nearest_weight(&[300, 700], 500), 300);
    }

    // === Edge cases ===

    #[test]
    fn single_candidate() {
        assert_eq!(nearest_weight(&[700], 400), 700);
    }

    #[test]
    fn empty_returns_target() {
        assert_eq!(nearest_weight(&[], 400), 400);
    }

    #[test]
    fn all_standard_weights_target_400() {
        let weights = [100, 200, 300, 400, 500, 600, 700, 800, 900];
        assert_eq!(nearest_weight(&weights, 400), 400);
    }

    #[test]
    fn all_standard_weights_target_350() {
        let weights = [100, 200, 300, 400, 500, 600, 700, 800, 900];
        assert_eq!(nearest_weight(&weights, 350), 300);
    }

    #[test]
    fn all_standard_weights_target_750() {
        let weights = [100, 200, 300, 400, 500, 600, 700, 800, 900];
        assert_eq!(nearest_weight(&weights, 750), 800);
    }
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
