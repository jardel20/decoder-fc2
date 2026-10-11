use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid or unsupported FC2: {0}")]
    Format(String),
    #[error("range offset={offset} length={length} exceeds buffer size={size}")]
    Bounds {
        offset: usize,
        length: usize,
        size: usize,
    },
    #[error("scientific export blocked: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, Error>;
