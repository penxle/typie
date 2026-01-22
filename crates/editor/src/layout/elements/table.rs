use crate::model::NodeId;
use crate::model::TableBorderStyle;
use crate::types::Size;

#[derive(Debug, Clone)]
pub struct TableBorderElement {
    pub size: Size,
    pub node_id: NodeId,
    pub border_style: TableBorderStyle,
    pub rows: usize,
    pub cols: usize,
    pub row_heights: Vec<f32>,
    pub col_widths: Vec<f32>,
}

impl TableBorderElement {
    pub fn new(
        size: Size,
        node_id: NodeId,
        border_style: TableBorderStyle,
        rows: usize,
        cols: usize,
        row_heights: Vec<f32>,
        col_widths: Vec<f32>,
    ) -> Self {
        Self {
            size,
            node_id,
            border_style,
            rows,
            cols,
            row_heights,
            col_widths,
        }
    }
}
