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
                values.push(widths.get(col_idx).copied().unwrap_or(0.0));
            }
            values
        } else {
            vec![1.0 / self.col_count as f32; self.col_count]
        };

        let base_inner_width = table_inner_width.max(0.0);
        ratios
            .into_iter()
            .map(|ratio| (base_inner_width * ratio.max(0.0)).max(MIN_CELL_WIDTH))
            .collect()
    }

    pub fn actual_table_width_for_proportion(&self, proportion: f32, ratio_widths: &[f32]) -> f32 {
        if self.col_count == 0 {
            return 0.0;
        }

        let table_width_floor = self.min_table_width().min(self.content_width.max(0.0));
        let table_width_constraint = self.target_table_width(proportion).max(table_width_floor);
        let table_inner_width = self.inner_width_from_table_width(table_width_constraint);
        let col_widths = self.calculate_col_widths(Some(ratio_widths), table_inner_width);
        col_widths.iter().sum::<f32>() + self.border_width()
    }

    pub fn proportion_for_actual_table_width(
        &self,
        target_width: f32,
        ratio_widths: &[f32],
    ) -> f32 {
        if self.col_count == 0 || self.content_width <= 0.0 {
            return 1.0;
        }

        let min_width = self.actual_table_width_for_proportion(0.0, ratio_widths);
        let max_width = self.actual_table_width_for_proportion(1.0, ratio_widths);
        if target_width <= min_width {
            return 0.0;
        }
        if target_width >= max_width {
            return 1.0;
        }

        let mut lo = 0.0f32;
        let mut hi = 1.0f32;
        for _ in 0..24 {
            let mid = (lo + hi) * 0.5;
            let mid_width = self.actual_table_width_for_proportion(mid, ratio_widths);
            if mid_width < target_width {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) * 0.5
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
        assert_eq!(widths, vec![200.0, 300.0]);
    }

    #[test]
    fn test_calculate_col_widths_custom_no_normalization() {
        let custom = vec![0.4, 0.4];
        let widths = TableWidthModel::new(2, 0.0).calculate_col_widths(Some(&custom), 500.0);
        assert_eq!(widths, vec![200.0, 200.0]);
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
