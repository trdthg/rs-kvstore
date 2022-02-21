use std::io::{self, Read};

use base64::{Result, CLIError, encode, decode};

fn read_stdin() -> Result<String> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|_| CLIError::StdInUnreadable)?;

    Ok(input.trim().to_string())
}

fn main() -> Result<()> {
    if std::env::args().count() < 2 {
        return Err(CLIError::TooFewArguments);
    }

    let sub_command = std::env::args().nth(1).ok_or(CLIError::TooFewArguments)?;
    let input = read_stdin()?;
    let output = match sub_command.as_str() {
        "encode" => {
            encode(input.as_bytes())
        },
        "decode" => {
            let a = decode(&input).map_err(|_| CLIError::DecodingError)?;
            let b = String::from_utf8(a).map_err(|_| CLIError::DecodingError)?;
            b
        },
        cmd => {
            return Err(CLIError::InvalidSubcommand(cmd.to_owned()))
        }
    };
    println!("{}", output);
    Ok(())
}