#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("invalid font data: {0}")]
    InvalidFont(String),

    #[error("encoding failed: {0}")]
    EncodingFailed(String),
}
