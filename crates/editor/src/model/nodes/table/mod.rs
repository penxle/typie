mod cell;
mod node;
mod row;

pub use cell::*;
pub use node::*;
pub use row::*;

pub const TABLE_BORDER_WIDTH: f32 = 1.0;

pub const TABLE_CELL_PADDING: f32 = 8.0;

pub fn calculate_col_widths(
    max_width: f32,
    col_count: usize,
    custom_widths: Option<&[f32]>,
) -> Vec<f32> {
    if col_count == 0 {
        return Vec::new();
    }

    let total_border = TABLE_BORDER_WIDTH * (col_count as f32 + 1.0);
    let available_width = max_width - total_border;

    if let Some(widths) = custom_widths {
        widths.to_vec()
    } else {
        let col_width = available_width / col_count as f32;
        vec![col_width; col_count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_col_widths_equal() {
        let widths = calculate_col_widths(100.0, 3, None);
        assert_eq!(widths.len(), 3);
        assert!((widths[0] - 32.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_col_widths_empty() {
        let widths = calculate_col_widths(100.0, 0, None);
        assert!(widths.is_empty());
    }
}
