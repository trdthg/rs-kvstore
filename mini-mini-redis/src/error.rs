use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("data store disconnected")]
    Io(#[from] std::io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("connect failed: {0}")]
    ConnectError(String),
    #[error("unknown data store error")]
    Unknown,
}
pub type Result<T> = std::result::Result<T, Error>;
