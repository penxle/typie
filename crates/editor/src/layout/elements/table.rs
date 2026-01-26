use crate::model::NodeId;
use crate::model::TableBorderStyle;
use crate::model::TableAlign;
use crate::types::Size;

#[derive(Debug, Clone)]
pub struct TableBorderElement {
    pub size: Size,
    pub node_id: NodeId,
    pub border_style: TableBorderStyle,
    pub align: TableAlign,
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
        align: TableAlign,
        rows: usize,
        cols: usize,
        row_heights: Vec<f32>,
        col_widths: Vec<f32>,
    ) -> Self {
        Self {
            size,
            node_id,
            border_style,
            align,
            rows,
            cols,
            row_heights,
            col_widths,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableCellElement {
    pub size: Size,
    pub node_id: NodeId,
}

impl TableCellElement {
    pub fn new(size: Size, node_id: NodeId) -> Self {
        Self { size, node_id }
    }
}
