use crate::model::NodeId;

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionKind {
    Toggle { node_id: NodeId },
    CycleCalloutVariant { node_id: NodeId },
}

pub trait Interactive {
    fn interaction_kind(&self) -> InteractionKind;
}
