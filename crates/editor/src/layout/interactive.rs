use crate::model::NodeId;

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionKind {
    Toggle { node_id: NodeId },
    CycleCalloutVariant { node_id: NodeId },
}

impl InteractionKind {
    pub fn allow_in_read_only(&self) -> bool {
        match self {
            InteractionKind::Toggle { .. } => true,
            InteractionKind::CycleCalloutVariant { .. } => false,
        }
    }
}

pub trait Interactive {
    fn interaction_kind(&self) -> InteractionKind;
}
