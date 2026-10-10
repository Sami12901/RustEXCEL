use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("System clipboard error: {0}")]
    System(String),

    #[error("TSV parse error: {0}")]
    Parse(String),

    #[error("Range is empty")]
    EmptyRange,

    #[error("Invalid coordinate: {0}")]
    InvalidCoordinate(String),
}
