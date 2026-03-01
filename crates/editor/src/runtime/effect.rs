use crate::model::{ParagraphNode, Style};
use crate::runtime::text_replacement::ReplacementUndoState;
use crate::state::Selection;
use crate::{model::NodeId, state::Position, types::PointerStyle};

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    DocChanged,
    NodeChanged {
        node_id: NodeId,
    }, // 노드와 조상 invalidate
    SubtreeChanged {
        node_id: NodeId,
    }, // 노드와 자손 invalidate
    SelectionChanged,
    PendingStylesChanged,
    SettingsChanged,
    FullLayoutInvalidation,
    LayoutChanged,
    PreeditChanged {
        node_id: Option<NodeId>,
    },
    FontDetected {
        family: String,
        weight: u16,
        codepoints: Vec<u32>,
    },
    ExternalElementChanged,
    PointerStyleChanged {
        style: PointerStyle,
    },
    DropTargetChanged {
        target: Option<Position>,
    },
    StructureChanged, // 구조적 변경 (노드 추가 / 삭제)
    ExitedDocumentStart,
    TextReplacementApplied {
        undo_state: ReplacementUndoState,
    },
    HtmlPasted {
        selection: Selection,
        text: String,
        styles: Vec<Style>,
        paragraph_attrs: Option<ParagraphNode>,
    },
}

impl Effect {
    pub fn priority(&self) -> u8 {
        match self {
            Effect::DocChanged => 0,
            Effect::NodeChanged { .. } => 1,
            Effect::SubtreeChanged { .. } => 2,
            Effect::SelectionChanged => 3,
            Effect::PendingStylesChanged => 4,
            Effect::SettingsChanged => 5,
            Effect::FullLayoutInvalidation => 6,
            Effect::LayoutChanged => 7,
            Effect::PreeditChanged { .. } => 8,
            Effect::HtmlPasted { .. } => 9,
            Effect::FontDetected { .. } => 10,
            Effect::ExternalElementChanged => 11,
            Effect::PointerStyleChanged { .. } => 12,
            Effect::DropTargetChanged { .. } => 13,
            Effect::StructureChanged => 14,
            Effect::ExitedDocumentStart => 15,
            Effect::TextReplacementApplied { .. } => 16,
        }
    }
}
