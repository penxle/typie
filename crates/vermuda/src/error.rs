use thiserror::Error;

#[derive(Error, Debug)]
pub enum VermudaError {
    #[error("Virtualization error: {0}")]
    Virtualization(String),

    #[error("VM operation failed: {0}")]
    OperationFailed(String),

    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VermudaError>;

impl VermudaError {
    pub fn virtualization<S: Into<String>>(msg: S) -> Self {
        Self::Virtualization(msg.into())
    }

    pub fn operation_failed<S: Into<String>>(msg: S) -> Self {
        Self::OperationFailed(msg.into())
    }

    pub fn validation_failed<S: Into<String>>(msg: S) -> Self {
        Self::ValidationFailed(msg.into())
    }

    pub fn resource_unavailable<S: Into<String>>(msg: S) -> Self {
        Self::ResourceUnavailable(msg.into())
    }
}
