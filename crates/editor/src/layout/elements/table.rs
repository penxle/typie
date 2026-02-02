use crate::layout::elements::{SplitEdges, WrapperPadding};
use crate::model::NodeId;
use crate::model::TableAlign;
use crate::model::TableBorderStyle;
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
    pub split_edges: SplitEdges,
    pub offset: f32,
    pub x_offset: f32,
    pub start_row_index: usize,
    pub total_rows: usize,
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
        split_edges: SplitEdges,
        offset: f32,
        x_offset: f32,
        start_row_index: usize,
        total_rows: usize,
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
            split_edges,
            offset,
            x_offset,
            start_row_index,
            total_rows,
        }
    }
}

impl crate::layout::elements::Wrapper for TableBorderElement {
    fn padding(&self) -> WrapperPadding {
        WrapperPadding {
            top: super::super::super::model::TABLE_BORDER_WIDTH,
            bottom: super::super::super::model::TABLE_BORDER_WIDTH,
            left: 0.0,
            right: 0.0,
        }
    }

    fn prevent_empty_on_page_break(&self) -> bool {
        true
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
