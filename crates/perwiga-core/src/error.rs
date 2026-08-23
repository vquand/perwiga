use thiserror::Error;

pub type Result<T> = std::result::Result<T, PerwigaError>;

#[derive(Debug, Error)]
pub enum PerwigaError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("{0} was not found")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Filesystem(#[from] std::io::Error),
    #[error("feed parse error: {0}")]
    FeedParse(String),
    #[error("network error: {0}")]
    Network(String),
}
