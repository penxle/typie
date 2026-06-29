use editor_crdt::Dot;
use editor_transaction::StepError;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Step(#[from] StepError),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("node not found: {0:?}")]
    NodeNotFound(Dot),
    #[error("node has no parent: {0:?}")]
    NoParent(Dot),
    #[error("corrupted document: {0}")]
    Corrupted(String),
    #[error("expected element node, got {0:?}")]
    ExpectedElementNode(Dot),
}

impl CommandError {
    pub fn orphan_child(child: Dot, parent: Dot) -> Self {
        Self::Corrupted(format!(
            "node {child:?} not found in children of {parent:?}"
        ))
    }
}

pub type CommandResult = Result<bool, CommandError>;
