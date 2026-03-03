use crate::model::{NodeId, Style};

#[derive(Debug, Clone, PartialEq)]
pub struct PreeditDecor {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionDecor {
    TextRange {
        node_id: NodeId,
        start_offset: usize,
        end_offset: usize,
    },
    Block {
        node_id: NodeId,
    },
}

impl SelectionDecor {
    pub fn node_id(&self) -> NodeId {
        match self {
            SelectionDecor::TextRange { node_id, .. } => *node_id,
            SelectionDecor::Block { node_id } => *node_id,
        }
    }

    pub fn start_offset(&self) -> usize {
        match self {
            SelectionDecor::TextRange { start_offset, .. } => *start_offset,
            SelectionDecor::Block { .. } => 0,
        }
    }

    pub fn end_offset(&self) -> usize {
        match self {
            SelectionDecor::TextRange { end_offset, .. } => *end_offset,
            SelectionDecor::Block { .. } => usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingStylesDecor {
    pub node_id: NodeId,
    pub styles: Vec<Style>,
}

impl Default for PendingStylesDecor {
    fn default() -> Self {
        Self {
            node_id: NodeId::ROOT,
            styles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Decorations {
    pub preedit: Option<PreeditDecor>,
    pub pending_styles: PendingStylesDecor,
}
