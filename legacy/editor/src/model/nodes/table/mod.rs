mod cell;
mod node;
mod row;

pub use cell::*;
pub use node::*;
pub use row::*;

pub const TABLE_BORDER_WIDTH: f32 = 1.0;

pub const TABLE_CELL_PADDING: f32 = 8.0;

pub const MIN_CELL_WIDTH: f32 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableWidthModel {
    pub col_count: usize,
    pub content_width: f32,
}

impl TableWidthModel {
    pub fn new(col_count: usize, content_width: f32) -> Self {
        Self {
            col_count,
            content_width,
        }
    }

    pub fn border_width(&self) -> f32 {
        TABLE_BORDER_WIDTH * (self.col_count as f32 + 1.0)
    }

    pub fn min_table_width(&self) -> f32 {
        if self.col_count == 0 {
            return 0.0;
        }
        MIN_CELL_WIDTH * self.col_count as f32 + self.border_width()
    }

    pub fn target_table_width(&self, proportion: f32) -> f32 {
        self.content_width.max(0.0) * proportion.clamp(0.0, 1.0)
    }

    pub fn inner_width_from_table_width(&self, table_width: f32) -> f32 {
        (table_width - self.border_width()).max(0.0)
    }

    pub fn calculate_col_widths(
        &self,
        custom_widths: Option<&[f32]>,
        table_inner_width: f32,
    ) -> Vec<f32> {
        if self.col_count == 0 {
            return Vec::new();
        }

        let ratios = if let Some(widths) = custom_widths {
            let mut values = Vec::with_capacity(self.col_count);
            for col_idx in 0..self.col_count {
                let raw = widths.get(col_idx).copied().unwrap_or(0.0);
                values.push(if raw.is_finite() { raw.max(0.0) } else { 0.0 });
            }
            values
        } else {
            vec![1.0 / self.col_count as f32; self.col_count]
        };

        let target_inner_width = table_inner_width.max(0.0);
        let min_total_width = MIN_CELL_WIDTH * self.col_count as f32;
        if target_inner_width <= min_total_width {
            return vec![MIN_CELL_WIDTH; self.col_count];
        }
        let ratio_sum = ratios.iter().sum::<f32>();
        if ratio_sum <= f32::EPSILON {
            return vec![target_inner_width / self.col_count as f32; self.col_count];
        }

        let mut widths = vec![MIN_CELL_WIDTH; self.col_count];
        let mut active = ratios
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(idx, ratio)| (ratio > 0.0).then_some((ratio, idx)))
            .collect::<Vec<_>>();

        if active.is_empty() {
            return vec![target_inner_width / self.col_count as f32; self.col_count];
        }

        active.sort_by(|(left, _), (right, _)| left.partial_cmp(right).unwrap());

        let mut constrained_count = self.col_count - active.len();
        let mut constrained_ratio_sum = 0.0f32;
        let total_ratio_sum = active.iter().map(|(ratio, _)| *ratio).sum::<f32>();
        let mut first_unconstrained = 0usize;
        let mut scale = 0.0f32;

        while first_unconstrained < active.len() {
            let unconstrained_ratio_sum = total_ratio_sum - constrained_ratio_sum;
            if unconstrained_ratio_sum <= f32::EPSILON {
                break;
            }

            let remaining_width =
                (target_inner_width - constrained_count as f32 * MIN_CELL_WIDTH).max(0.0);
            scale = remaining_width / unconstrained_ratio_sum;

            let (smallest_ratio, _) = active[first_unconstrained];
            if scale * smallest_ratio >= MIN_CELL_WIDTH {
                break;
            }

            constrained_ratio_sum += smallest_ratio;
            constrained_count += 1;
            first_unconstrained += 1;
        }

        for (ratio, idx) in active.into_iter().skip(first_unconstrained) {
            widths[idx] = (scale * ratio).max(MIN_CELL_WIDTH);
        }

        let total_width = widths.iter().sum::<f32>();
        let diff = target_inner_width - total_width;
        let tolerance = (target_inner_width.abs() * 1e-6).max(1e-4);
        if diff.abs() > tolerance {
            if let Some(last_idx) = (0..self.col_count).rev().find(|&idx| {
                ratios[idx] > f32::EPSILON
                    && (diff > 0.0 || widths[idx] > MIN_CELL_WIDTH + tolerance)
            }) {
                widths[last_idx] = (widths[last_idx] + diff).max(MIN_CELL_WIDTH);
            }
        }

        widths
    }

    pub fn actual_table_width_for_proportion(&self, proportion: f32) -> f32 {
        if self.col_count == 0 {
            return 0.0;
        }

        self.target_table_width(proportion)
            .max(self.min_table_width())
    }

    pub fn proportion_for_actual_table_width(&self, target_width: f32) -> f32 {
        if self.col_count == 0 || self.content_width <= 0.0 {
            return 1.0;
        }

        let min_width = self.actual_table_width_for_proportion(0.0);
        let max_width = self.actual_table_width_for_proportion(1.0);
        if target_width <= min_width {
            return 0.0;
        }
        if target_width >= max_width {
            return 1.0;
        }
        (target_width / self.content_width.max(0.0)).clamp(0.0, 1.0)
    }

    pub fn inserted_ratio_widths(existing_widths: &[f32], insert_index: usize) -> Vec<f32> {
        let new_col_count = existing_widths.len() + 1;
        let inserted_width = 1.0 / new_col_count as f32;
        let scale = 1.0 - inserted_width;

        let mut next_widths: Vec<f32> = existing_widths.iter().map(|width| width * scale).collect();
        let insert_at = insert_index.min(existing_widths.len());
        next_widths.insert(insert_at, inserted_width);
        next_widths
    }

    pub fn removed_ratio_widths(existing_widths: &[f32], remove_index: usize) -> Vec<f32> {
        if existing_widths.len() <= 1 {
            return Vec::new();
        }

        let remove_at = remove_index.min(existing_widths.len() - 1);
        let mut next_widths = Vec::with_capacity(existing_widths.len() - 1);
        for (idx, width) in existing_widths.iter().copied().enumerate() {
            if idx != remove_at {
                next_widths.push(width.max(0.0));
            }
        }

        let sum = next_widths.iter().sum::<f32>();
        if sum <= 0.0 {
            let fallback = 1.0 / next_widths.len() as f32;
            return vec![fallback; next_widths.len()];
        }

        next_widths.into_iter().map(|width| width / sum).collect()
    }

    pub fn validate_ratio_widths(widths: &[f32], expected_len: usize) -> Option<Vec<f32>> {
        if widths.len() != expected_len {
            return None;
        }
        if widths.is_empty() {
            return Some(Vec::new());
        }
        if widths
            .iter()
            .any(|width| !width.is_finite() || *width < 0.0)
        {
            return None;
        }
        if widths.iter().all(|width| *width == 0.0) {
            return None;
        }
        Some(widths.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_col_widths_default() {
        let widths = TableWidthModel::new(3, 0.0).calculate_col_widths(None, 300.0);
        assert_eq!(widths.len(), 3);
        assert!((widths[0] - 100.0).abs() < 0.001);
        assert!((widths[1] - 100.0).abs() < 0.001);
        assert!((widths[2] - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_col_widths_empty() {
        let widths = TableWidthModel::new(0, 0.0).calculate_col_widths(None, 300.0);
        assert!(widths.is_empty());
    }

    #[test]
    fn test_calculate_col_widths_custom_ratio() {
        let custom = vec![0.4, 0.6];
        let widths = TableWidthModel::new(2, 0.0).calculate_col_widths(Some(&custom), 500.0);
        assert!((widths[0] - 200.0).abs() < 0.001);
        assert!((widths[1] - 300.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_col_widths_fills_target_when_ratio_sum_is_not_one() {
        let custom = vec![0.4, 0.4];
        let widths = TableWidthModel::new(2, 0.0).calculate_col_widths(Some(&custom), 500.0);
        assert_eq!(widths, vec![250.0, 250.0]);
    }

    #[test]
    fn test_calculate_col_widths_redistributes_with_min_width() {
        let custom = vec![0.1, 0.9];
        let widths = TableWidthModel::new(2, 0.0).calculate_col_widths(Some(&custom), 200.0);
        assert!((widths[0] - 40.0).abs() < 0.001);
        assert!((widths[1] - 160.0).abs() < 0.001);
        assert!((widths.iter().sum::<f32>() - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_col_widths_zero_ratios_evenly_distributes_remaining_width() {
        let custom = vec![0.0, 0.0];
        let widths = TableWidthModel::new(2, 0.0).calculate_col_widths(Some(&custom), 300.0);
        assert!((widths[0] - 150.0).abs() < 0.001);
        assert!((widths[1] - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_col_widths_respects_min_width() {
        let widths = TableWidthModel::new(10, 0.0).calculate_col_widths(None, 300.0);
        assert_eq!(widths, vec![MIN_CELL_WIDTH; 10]);
    }

    #[test]
    fn test_min_table_width() {
        let model = TableWidthModel::new(3, 400.0);
        assert!((model.min_table_width() - 124.0).abs() < 0.001);
    }

    #[test]
    fn test_min_table_width_can_exceed_content_width() {
        let model = TableWidthModel::new(10, 200.0);
        assert!(model.min_table_width() > model.content_width);
    }

    #[test]
    fn test_actual_table_width_for_proportion_closed_form() {
        let model = TableWidthModel::new(2, 500.0);
        let min_width = model.min_table_width();
        assert!((model.actual_table_width_for_proportion(0.0) - min_width).abs() < 0.001);
        assert!((model.actual_table_width_for_proportion(0.5) - 250.0).abs() < 0.001);
        assert!((model.actual_table_width_for_proportion(1.0) - 500.0).abs() < 0.001);
    }

    #[test]
    fn test_proportion_for_actual_table_width_closed_form() {
        let model = TableWidthModel::new(2, 500.0);
        let min_width = model.min_table_width();
        assert!((model.proportion_for_actual_table_width(min_width) - 0.0).abs() < 0.001);
        assert!((model.proportion_for_actual_table_width(250.0) - 0.5).abs() < 0.001);
        assert!((model.proportion_for_actual_table_width(999.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_proportion_for_actual_table_width_when_content_is_smaller_than_min() {
        let model = TableWidthModel::new(10, 200.0);
        let min_width = model.min_table_width();
        assert!((model.proportion_for_actual_table_width(min_width) - 0.0).abs() < 0.001);
        assert!((model.proportion_for_actual_table_width(min_width + 1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_inserted_ratio_widths() {
        let next = TableWidthModel::inserted_ratio_widths(&[0.3, 0.7], 1);
        assert_eq!(next.len(), 3);
        assert!((next[0] - 0.2).abs() < 0.001);
        assert!((next[1] - 0.33333334).abs() < 0.001);
        assert!((next[2] - 0.46666667).abs() < 0.001);
    }

    #[test]
    fn test_removed_ratio_widths() {
        let next = TableWidthModel::removed_ratio_widths(&[0.2, 0.3, 0.5], 1);
        assert_eq!(next.len(), 2);
        assert!((next[0] - 0.2857143).abs() < 0.001);
        assert!((next[1] - 0.71428573).abs() < 0.001);
        assert!((next.iter().sum::<f32>() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_removed_ratio_widths_fallback_equal_when_sum_zero() {
        let next = TableWidthModel::removed_ratio_widths(&[0.0, 0.0, 0.0], 1);
        assert_eq!(next, vec![0.5, 0.5]);
    }
}
