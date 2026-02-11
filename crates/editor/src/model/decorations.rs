use crate::model::{Mark, NodeId};

#[derive(Debug, Clone, PartialEq)]
pub struct PreeditDecor {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub marks: Option<Vec<Mark>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionDecor {
    Text {
        node_id: NodeId,
        start_offset: usize,
        end_offset: usize,
    },
    Cell {
        node_id: NodeId,
    },
    Fold {
        node_id: NodeId,
    },
}

impl SelectionDecor {
    pub fn node_id(&self) -> NodeId {
        match self {
            SelectionDecor::Text { node_id, .. } => *node_id,
            SelectionDecor::Cell { node_id } => *node_id,
            SelectionDecor::Fold { node_id } => *node_id,
        }
    }

    pub fn start_offset(&self) -> usize {
        match self {
            SelectionDecor::Text { start_offset, .. } => *start_offset,
            SelectionDecor::Cell { .. } | SelectionDecor::Fold { .. } => 0,
        }
    }

    pub fn end_offset(&self) -> usize {
        match self {
            SelectionDecor::Text { end_offset, .. } => *end_offset,
            SelectionDecor::Cell { .. } | SelectionDecor::Fold { .. } => usize::MAX,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, SelectionDecor::Text { .. })
    }

    pub fn is_cell(&self) -> bool {
        matches!(self, SelectionDecor::Cell { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingMarksDecor {
    pub node_id: NodeId,
    pub marks: Vec<Mark>,
}

#[derive(Debug, Clone, Default)]
pub struct Decorations {
    pub preedit: Option<PreeditDecor>,
    pub pending_marks: Option<PendingMarksDecor>,
}
