
use std::fmt;

pub enum CLIError {
    TooFewArguments,
    InvalidSubcommand(String),
    StdInUnreadable,
    DecodingError,
}

pub type Result<T> = std::result::Result<T, CLIError>;

impl std::fmt::Debug for CLIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::TooFewArguments =>
                write!(f, "Not enough arguments provided"),

            Self::InvalidSubcommand(cmd) =>
                write!(f, "Invalid subcommand provided: \"{}\"", cmd),

            Self::StdInUnreadable =>
                write!(f, "Unable to read STDIN"),

            Self::DecodingError =>
                write!(f, "An error occured while decoding the data"),
        }
    }
}