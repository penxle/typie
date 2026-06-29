use editor_crdt::CrdtError;

use crate::projected_state::SpineError;

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error(transparent)]
    Crdt(#[from] CrdtError),
    #[error("{0:?}")]
    Spine(SpineError),
}

impl From<SpineError> for StateError {
    fn from(e: SpineError) -> Self {
        match e {
            SpineError::Crdt(c) => StateError::Crdt(c),
            other => StateError::Spine(other),
        }
    }
}
