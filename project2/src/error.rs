use failure::Fail;
use std::io;

/// Error type for kvs
#[derive(Fail, Debug)]
pub enum KvsError {

    /// IO Error
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),

    /// Serialization Error
    #[fail(display = "{}", _0)]
    Serde(#[cause] serde_json::Error),

    /// Key not found
    #[fail(display = "Key Not Found")]
    KeyNotFound,

    /// Unexpected command
    #[fail(display = "Unexpected command type Error")]
    UnExpectedCommandType,

    #[fail(display = "Reader Not Found")]
    ReaderNotFound,

}

impl From<io::Error> for KvsError {
    fn from(err: io::Error) -> KvsError {
        KvsError::Io(err)
    }
}

impl From<serde_json::Error> for KvsError {
    fn from(err: serde_json::Error) -> KvsError {
        KvsError::Serde(err)
    }
}

pub type Result<T> = std::result::Result<T, KvsError>;