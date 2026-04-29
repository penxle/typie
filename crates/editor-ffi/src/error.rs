#[derive(Debug, thiserror::Error)]
pub enum FfiError {
    #[error("deserialization failed: {0}")]
    Deserialization(String),

    #[error("serialization failed: {0}")]
    Serialization(String),

    #[error("surface creation failed: {0}")]
    Surface(String),

    #[error("lock poisoned")]
    LockPoisoned,
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
#[cfg_attr(feature = "uniffi", uniffi(flat_error))]
pub enum EditorError {
    #[error("{msg}")]
    General { msg: String },

    #[error(transparent)]
    Core(#[from] editor_core::EditorError),

    #[error(transparent)]
    Model(#[from] editor_model::ReconstructError),

    #[error(transparent)]
    Resource(#[from] editor_resource::ResourceError),

    #[error(transparent)]
    Ffi(#[from] FfiError),

    #[cfg(feature = "wasm-server")]
    #[error(transparent)]
    Server(#[from] editor_server::ServerError),
}
