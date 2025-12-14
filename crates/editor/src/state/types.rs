use crate::model::{Mark, NodeId};

#[derive(Debug, Clone, PartialEq)]
pub struct Preedit {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub marks: Option<Vec<Mark>>,
}
