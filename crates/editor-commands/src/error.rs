use editor_model::NodeId;
use editor_transaction::StepError;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Step(#[from] StepError),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("node not found: {0:?}")]
    NodeNotFound(NodeId),
    #[error("node has no parent: {0:?}")]
    NoParent(NodeId),
    #[error("corrupted document: {0}")]
    Corrupted(String),
    #[error("expected element node, got {0:?}")]
    ExpectedElementNode(NodeId),
}

impl CommandError {
    pub fn orphan_child(child: NodeId, parent: NodeId) -> Self {
        Self::Corrupted(format!(
            "node {child:?} not found in children of {parent:?}"
        ))
    }
}

pub type CommandResult = Result<bool, CommandError>;
