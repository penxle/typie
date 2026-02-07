use crate::layout::elements::ExternalElementData;
use crate::model::{LayoutMode, Mark, MarkType, TextAlign};
use crate::state::Position;
use crate::types::{PointerStyle, Rect, TextBound};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct ExternalElement {
    pub page_idx: usize,
    pub node_id: String,
    pub bounds: Rect,
    pub data: ExternalElementData,
    pub is_selected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct SelectionStats {
    pub block_count: usize,
    pub paragraph_count: usize,
    pub uniform_align: Option<TextAlign>,
    pub uniform_line_height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct LinkOverlay {
    pub page_idx: usize,
    pub href: String,
    pub bounds: Vec<TextBound>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct SpellcheckOverlay {
    pub page_idx: usize,
    pub id: String,
    pub bounds: Vec<TextBound>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct AiFeedbackOverlay {
    pub page_idx: usize,
    pub id: String,
    pub bounds: Vec<TextBound>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct SelectionHandleBounds {
    pub page_idx: usize,
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct SearchOverlay {
    pub page_idx: usize,
    pub bounds: Vec<TextBound>,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Cmd {
    DocChanged,

    #[serde(rename_all = "camelCase")]
    SettingsChanged {
        paragraph_indent: f32,
        block_gap: f32,
    },

    #[serde(rename_all = "camelCase")]
    LayoutChanged {
        page_count: usize,
        layout_mode: LayoutMode,
        page_width: f32,
        page_heights: Vec<f32>,
    },

    #[serde(rename_all = "camelCase")]
    CursorChanged {
        page_idx: Option<usize>,
        bounds: Option<Rect>,
        show: bool,
        scroll_to_cursor: bool,
        preceding_char_widths: Option<Vec<f32>>,
    },

    ExternalElementChanged {
        elements: Vec<ExternalElement>,
    },

    PointerStyleChanged {
        style: PointerStyle,
    },

    #[serde(rename_all = "camelCase")]
    SelectionChanged {
        stats: SelectionStats,
        collapsed: bool,
        anchor: Position,
        head: Position,
        from_handle: Option<SelectionHandleBounds>,
        to_handle: Option<SelectionHandleBounds>,
    },

    #[serde(rename_all = "camelCase")]
    ActiveMarksChanged {
        uniform_marks: Vec<Mark>,
        mixed_marks: Vec<MarkType>,
    },

    FontRequired {
        family: String,
        weight: u16,
        codepoints: Vec<u32>,
    },

    FallbackFontRequired {
        codepoints: Vec<u32>,
    },

    RenderRequired,

    EnabledActionsChanged {
        enabled: Vec<String>,
    },

    ExitedDocumentStart,

    PointerModeChanged {
        is_idle: bool,
    },

    #[serde(rename_all = "camelCase")]
    PlaceholderChanged {
        visible: bool,
        bounds: Option<Rect>,
    },

    LinkOverlaysChanged {
        overlays: Vec<LinkOverlay>,
    },

    SpellcheckOverlaysChanged {
        overlays: Vec<SpellcheckOverlay>,
    },

    AiFeedbackOverlaysChanged {
        overlays: Vec<AiFeedbackOverlay>,
    },

    #[serde(rename_all = "camelCase")]
    SearchResultsChanged {
        overlays: Vec<SearchOverlay>,
        total_count: usize,
        current_index: usize,
    },

    TableOverlaysChanged {
        overlays: Vec<TableOverlay>,
    },
}
