#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("{msg}")]
    General { msg: String },

    #[error(transparent)]
    Step(#[from] editor_transaction::StepError),

    #[error(transparent)]
    Command(#[from] editor_commands::CommandError),
}
