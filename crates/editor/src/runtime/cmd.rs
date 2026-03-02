use crate::layout::elements::ExternalElementData;
use crate::model::{NodeId, RemarkId};
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
    pub proportion: f32,
    pub content_width: f32,
    pub min_proportion_width: f32,
    pub max_proportion_width: f32,
    pub col_widths: Vec<f32>,
    pub col_widths_as_px: Vec<f32>,
    pub col_positions: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub row_positions: Vec<f32>,
    pub start_row_index: usize,
    pub total_rows: usize,
    pub is_focused: bool,
    pub show_cell_selector: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InteractiveOverlay {
    pub page_idx: usize,
    pub node_id: NodeId,
    pub kind: u32, // 0 = ToggleFold, 1 = CycleCalloutVariant
    pub bounds: Rect,
    pub passthrough: Option<Rect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemarkOverlay {
    pub page_idx: usize,
    pub node_id: NodeId,
    pub remark_id: RemarkId,
    pub user_id: String,
    pub text: String,
    pub created_at: i64,
    pub bounds: Rect,
}
