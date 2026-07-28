#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("{msg}")]
    General { msg: String },

    #[error(transparent)]
    Step(#[from] editor_transaction::StepError),

    #[error(transparent)]
    Command(#[from] editor_commands::CommandError),

    #[error("empty request batches are not admitted")]
    EmptyRequest,

    #[error("request is not queued: {request_id:?}")]
    RequestNotQueued { request_id: crate::RequestId },
}
