use crate::layout::elements::ExternalElementData;
use crate::model::{LayoutMode, Mark, MarkType, TextAlign};
use crate::types::{PointerStyle, Rect, WritingSystem};
use serde::Serialize;
use tsify::Tsify;

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ExternalElement {
    pub page_idx: usize,
    pub node_id: String,
    pub bounds: Rect,
    pub data: ExternalElementData,
    pub is_selected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SelectionStats {
    pub block_count: usize,
    pub paragraph_count: usize,
    pub uniform_align: Option<TextAlign>,
    pub uniform_line_height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LinkOverlay {
    pub page_idx: usize,
    pub href: String,
    pub bounds: Vec<Rect>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
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
    },

    #[serde(rename_all = "camelCase")]
    ActiveMarksChanged {
        uniform_marks: Vec<Mark>,
        mixed_marks: Vec<MarkType>,
    },

    FontsRequired {
        fonts: Vec<(String, u16)>,
    },

    WritingSystemRequired {
        systems: Vec<WritingSystem>,
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
}

