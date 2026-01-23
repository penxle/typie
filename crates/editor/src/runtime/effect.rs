use crate::{
    model::NodeId,
    state::Position,
    types::{PointerStyle, WritingSystem},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    DocChanged,
    NodeChanged { node_id: NodeId },    // 노드와 조상 invalidate
    SubtreeChanged { node_id: NodeId }, // 노드와 자손 invalidate
    SelectionChanged,
    PendingMarksChanged,
    SettingsChanged,
    LayoutChanged,
    PreeditChanged { node_id: Option<NodeId> },
    FontUsageChanged { family: String, weight: u16 },
    WritingSystemsUsageChanged { systems: Vec<WritingSystem> },
    ExternalElementChanged,
    PointerStyleChanged { style: PointerStyle },
    DropTargetChanged { target: Option<Position> },
    StructureChanged, // 구조적 변경 (노드 추가 / 삭제)
    ExitedDocumentStart,
    SearchStateChanged,
}
