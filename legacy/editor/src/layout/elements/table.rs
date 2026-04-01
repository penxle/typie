use crate::layout::elements::{SplitEdges, Wrapper, WrapperPadding};
use crate::model::{NodeId, TABLE_BORDER_WIDTH, TableAlign, TableBorderStyle};
use crate::types::Size;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
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

impl Hash for TableBorderElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.size.hash(state);
        self.node_id.hash(state);
        self.border_style.hash(state);
        self.align.hash(state);
        self.rows.hash(state);
        self.cols.hash(state);
        hash_f32_slice(&self.row_heights, state);
        hash_f32_slice(&self.col_widths, state);
        self.split_edges.hash(state);
        self.offset.to_bits().hash(state);
        self.x_offset.to_bits().hash(state);
        self.start_row_index.hash(state);
        self.total_rows.hash(state);
    }
}

fn hash_f32_slice<H: Hasher>(values: &[f32], state: &mut H) {
    values.len().hash(state);
    for value in values {
        value.to_bits().hash(state);
    }
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

impl Wrapper for TableBorderElement {
    fn padding(&self) -> WrapperPadding {
        WrapperPadding {
            top: TABLE_BORDER_WIDTH,
            bottom: TABLE_BORDER_WIDTH,
            left: 0.0,
            right: 0.0,
        }
    }

    fn prevent_empty_on_page_break(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCellElement {
    pub size: Size,
    pub node_id: NodeId,
}

impl TableCellElement {
    pub fn new(size: Size, node_id: NodeId) -> Self {
        Self { size, node_id }
    }
}
