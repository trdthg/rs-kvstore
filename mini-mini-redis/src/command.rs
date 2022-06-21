use crate::{Frame, Result};

pub enum Command {
    Get { key: String },
    Set { key: String, value: String },
}

impl Command {
    pub fn from_frame(frame: Frame) -> Result<Self> {
        let mut parser = Parser::new();
        let command_name = parser.parse(frame)?;
        Self {

        }
    }
}