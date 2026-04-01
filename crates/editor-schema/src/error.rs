#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("{0}")]
    InvalidContent(String),
}
