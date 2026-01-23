mod cell;
mod node;
mod row;

pub use cell::*;
pub use node::*;
pub use row::*;

pub const TABLE_BORDER_WIDTH: f32 = 1.0;

pub const TABLE_CELL_PADDING: f32 = 8.0;

pub const MIN_CELL_WIDTH: f32 = 40.0;

pub const DEFAULT_CELL_WIDTH: f32 = 80.0;

pub fn calculate_col_widths(col_count: usize, custom_widths: Option<&[f32]>) -> Vec<f32> {
    if col_count == 0 {
        return Vec::new();
    }

    if let Some(widths) = custom_widths {
        widths.to_vec()
    } else {
        vec![DEFAULT_CELL_WIDTH; col_count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_col_widths_default() {
        let widths = calculate_col_widths(3, None);
        assert_eq!(widths.len(), 3);
        assert_eq!(widths[0], DEFAULT_CELL_WIDTH);
        assert_eq!(widths[1], DEFAULT_CELL_WIDTH);
        assert_eq!(widths[2], DEFAULT_CELL_WIDTH);
    }

    #[test]
    fn test_calculate_col_widths_empty() {
        let widths = calculate_col_widths(0, None);
        assert!(widths.is_empty());
    }

    #[test]
    fn test_calculate_col_widths_custom() {
        let custom = vec![100.0, 200.0, 150.0];
        let widths = calculate_col_widths(3, Some(&custom));
        assert_eq!(widths, custom);
    }
}
