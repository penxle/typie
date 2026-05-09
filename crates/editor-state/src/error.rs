use editor_crdt::CrdtError;
use editor_model::ModelError;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum StateError {
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Crdt(#[from] CrdtError),
}
