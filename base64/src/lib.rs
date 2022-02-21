mod alphabet;
mod encoder;
mod decoder;
mod errors;

pub use errors::{CLIError, Result};
pub use  alphabet::*;
pub use encoder::encode;
pub use decoder::decode;
