#[derive(Debug, thiserror::Error)]
pub enum CommonError {
    #[error("decompression failed: {0}")]
    Decompression(String),

    #[error("failed to create ICU blob provider: {0}")]
    IcuProvider(String),

    #[error("failed to create segmenter: {0}")]
    IcuSegmenter(String),
}
