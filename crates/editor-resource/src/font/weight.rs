/// CSS Fonts Level 4 §5.2 font-weight matching.
/// `weights` must be sorted and deduplicated.
pub fn match_weight(weights: &[u16], target: u16) -> Option<u16> {
    if weights.is_empty() {
        return None;
    }

    if (400..=500).contains(&target) {
        // [target, 500] ascending -> < target descending -> > 500 ascending
        if let Some(&w) = weights.iter().find(|&&w| (target..=500).contains(&w)) {
            return Some(w);
        }
        if let Some(&w) = weights.iter().rev().find(|&&w| w < target) {
            return Some(w);
        }
        if let Some(&w) = weights.iter().find(|&&w| w > 500) {
            return Some(w);
        }
    } else if target < 400 {
        // <= target descending -> > target ascending
        if let Some(&w) = weights.iter().rev().find(|&&w| w <= target) {
            return Some(w);
        }
        if let Some(&w) = weights.iter().find(|&&w| w > target) {
            return Some(w);
        }
    } else {
        // >= target ascending -> < target descending
        if let Some(&w) = weights.iter().find(|&&w| w >= target) {
            return Some(w);
        }
        if let Some(&w) = weights.iter().rev().find(|&&w| w < target) {
            return Some(w);
        }
    }

    None
}

pub fn find_bold_target(current_weight: u16, available_weights: &[u16]) -> Option<u16> {
    let candidates: Vec<u16> = available_weights
        .iter()
        .copied()
        .filter(|&w| w > current_weight)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let bold_candidates: Vec<u16> = candidates.iter().copied().filter(|&w| w >= 700).collect();

    let pool = if bold_candidates.is_empty() {
        &candidates
    } else {
        &bold_candidates
    };

    nearest_in(pool, 700)
}

pub fn find_unbold_target(current_weight: u16, available_weights: &[u16]) -> u16 {
    let candidates: Vec<u16> = available_weights
        .iter()
        .copied()
        .filter(|&w| w < current_weight)
        .collect();

    if candidates.is_empty() {
        return 400;
    }

    nearest_in(&candidates, 400).unwrap_or(400)
}

fn nearest_in(weights: &[u16], target: u16) -> Option<u16> {
    let mut sorted = weights.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    match_weight(&sorted, target)
}

#[cfg(test)]
mod tests {
    use super::{find_bold_target, find_unbold_target, match_weight};

    #[test]
    fn mid_range_exact_400() {
        assert_eq!(match_weight(&[400, 700], 400), Some(400));
    }

    #[test]
    fn mid_range_exact_500() {
        assert_eq!(match_weight(&[300, 500], 500), Some(500));
    }

    #[test]
    fn mid_range_ascending_to_500() {
        assert_eq!(match_weight(&[400, 500], 450), Some(500));
    }

    #[test]
    fn mid_range_prefers_ascending_to_500_over_lighter() {
        assert_eq!(match_weight(&[300, 500], 400), Some(500));
    }

    #[test]
    fn mid_range_then_lighter() {
        assert_eq!(match_weight(&[200, 700], 450), Some(200));
    }

    #[test]
    fn mid_range_falls_back_to_above_500() {
        assert_eq!(match_weight(&[600, 800], 450), Some(600));
    }

    #[test]
    fn boundary_400_prefers_lighter() {
        assert_eq!(match_weight(&[200, 600], 400), Some(200));
    }

    #[test]
    fn boundary_500_prefers_lighter() {
        assert_eq!(match_weight(&[300, 700], 500), Some(300));
    }

    #[test]
    fn below_400_prefers_lighter() {
        assert_eq!(match_weight(&[100, 500], 300), Some(100));
    }

    #[test]
    fn below_400_exact() {
        assert_eq!(match_weight(&[100, 300, 500], 300), Some(300));
    }

    #[test]
    fn below_400_falls_back_to_heavier() {
        assert_eq!(match_weight(&[300, 500], 200), Some(300));
    }

    #[test]
    fn below_400_picks_closest_lighter() {
        assert_eq!(match_weight(&[200, 400], 350), Some(200));
    }

    #[test]
    fn above_500_prefers_heavier() {
        assert_eq!(match_weight(&[400, 900], 600), Some(900));
    }

    #[test]
    fn above_500_exact() {
        assert_eq!(match_weight(&[400, 700, 900], 700), Some(700));
    }

    #[test]
    fn above_500_falls_back_to_lighter() {
        assert_eq!(match_weight(&[400, 600], 800), Some(600));
    }

    #[test]
    fn above_500_picks_closest_heavier() {
        assert_eq!(match_weight(&[400, 800], 700), Some(800));
    }

    #[test]
    fn empty_weights() {
        assert_eq!(match_weight(&[], 400), None);
    }

    #[test]
    fn single_weight() {
        assert_eq!(match_weight(&[700], 400), Some(700));
    }

    #[test]
    fn all_standard_weights_target_400() {
        let w = [100, 200, 300, 400, 500, 600, 700, 800, 900];
        assert_eq!(match_weight(&w, 400), Some(400));
    }

    #[test]
    fn all_standard_weights_target_350() {
        let w = [100, 200, 300, 400, 500, 600, 700, 800, 900];
        assert_eq!(match_weight(&w, 350), Some(300));
    }

    #[test]
    fn all_standard_weights_target_750() {
        let w = [100, 200, 300, 400, 500, 600, 700, 800, 900];
        assert_eq!(match_weight(&w, 750), Some(800));
    }

    #[test]
    fn find_bold_target_prefers_700_when_available() {
        assert_eq!(
            find_bold_target(400, &[100, 300, 400, 500, 700, 900]),
            Some(700)
        );
    }

    #[test]
    fn find_bold_target_picks_nearest_bold_candidate() {
        assert_eq!(find_bold_target(400, &[400, 800, 900]), Some(800));
    }

    #[test]
    fn find_bold_target_uses_heavier_even_below_700() {
        assert_eq!(find_bold_target(400, &[400, 500]), Some(500));
    }

    #[test]
    fn find_bold_target_none_when_no_heavier() {
        assert_eq!(find_bold_target(900, &[400, 700, 900]), None);
    }

    #[test]
    fn find_bold_target_none_when_already_heaviest() {
        assert_eq!(find_bold_target(400, &[400]), None);
    }

    #[test]
    fn find_bold_target_from_300() {
        assert_eq!(find_bold_target(300, &[100, 300, 400, 700]), Some(700));
    }

    #[test]
    fn find_unbold_target_prefers_400() {
        assert_eq!(
            find_unbold_target(700, &[100, 300, 400, 500, 700, 900]),
            400
        );
    }

    #[test]
    fn find_unbold_target_picks_nearest_to_400() {
        assert_eq!(find_unbold_target(700, &[100, 300, 700]), 300);
    }

    #[test]
    fn find_unbold_target_defaults_to_400_when_no_lighter() {
        assert_eq!(find_unbold_target(100, &[100, 700]), 400);
    }

    #[test]
    fn find_unbold_target_from_900() {
        assert_eq!(find_unbold_target(900, &[400, 700, 900]), 400);
    }
}
