use crate::NodeType;

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("{0}")]
    InvalidContent(String),
    #[error("{node_type:?} not allowed in context {path:?}")]
    ContextViolation {
        node_type: NodeType,
        path: Vec<NodeType>,
    },
    #[error("document roots must be exactly [Root], got {roots:?}")]
    RootViolation { roots: Vec<NodeType> },
}
