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
    PendingMarksChanged,
    SettingsChanged,
    LayoutChanged,
    PreeditChanged {
        node_id: Option<NodeId>,
    },
    FontDetected {
        family: String,
        weight: u16,
        codepoints: Vec<u32>,
    },
    CodepointDetected {
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
    SearchStateChanged,
    TextReplacementApplied {
        undo_state: super::text_replacement::ReplacementUndoState,
    },
    HtmlPasted {
        text: String,
        from: Position,
        to: Position,
    },
}
