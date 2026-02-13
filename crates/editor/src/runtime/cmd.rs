use crate::layout::elements::ExternalElementData;
use crate::types::{Rect, TextBound};

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalElement {
    pub page_idx: usize,
    pub node_id: String,
    pub bounds: Rect,
    pub data: ExternalElementData,
    pub is_selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkOverlay {
    pub page_idx: usize,
    pub href: String,
    pub bounds: Vec<TextBound>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionHandleBounds {
    pub page_idx: usize,
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableOverlay {
    pub page_idx: usize,
    pub table_id: String,
    pub bounds: Rect,
    pub border_style: String,
    pub align: String,
    pub col_widths: Vec<f32>,
    pub col_positions: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub row_positions: Vec<f32>,
    pub start_row_index: usize,
    pub total_rows: usize,
    pub is_focused: bool,
}
