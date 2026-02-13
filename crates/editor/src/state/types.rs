use crate::model::NodeId;

#[derive(Debug, Clone, PartialEq)]
pub struct Preedit {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
}
