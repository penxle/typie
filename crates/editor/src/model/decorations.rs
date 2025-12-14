use crate::model::{Mark, NodeId};

#[derive(Debug, Clone)]
pub struct PreeditDecor {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub marks: Option<Vec<Mark>>,
}

#[derive(Debug, Clone)]
pub struct SelectionDecor {
    pub node_id: NodeId,
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Decorations {
    pub preedit: Option<PreeditDecor>,
}
