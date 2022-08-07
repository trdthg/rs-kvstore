use clap::{App, AppSettings, Arg};
use kvs::{KvStore, KvsError, Result};
use std::env::current_dir;
use std::process::exit;


fn main() -> Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("set")
                .about("Set the value of a string key to a string")
                .arg(Arg::new("KEY").help("A string key").required(true))
                .arg(
                    Arg::new("VALUE")
                        .help("The string value of the key")
                        .required(true),
                ),
        )
        .subcommand(
            App::new("get")
                .about("Get the string value of a given string key")
                .arg(Arg::new("KEY").help("A string key").required(true)),
        )
        .subcommand(
            App::new("rm")
                .about("Remove a given key")
                .arg(Arg::new("KEY").help("A string key").required(true)),
        )
        .get_matches();
    match matches.subcommand() {
        Some(("set", args)) => {
            let key = args.value_of("KEY").unwrap();
            let value = args.value_of("VALUE").unwrap();
            let mut store = KvStore::open(current_dir()?)?;
            store.set(key.to_string(), value.to_string())?;
        },
        Some(("get", matches)) => {
            let key = matches.value_of("KEY").unwrap();

            let mut store = KvStore::open(current_dir()?)?;
            if let Some(value) = store.get(key.to_string())? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        }
        Some(("rm", matches)) => {
            let key = matches.value_of("KEY").unwrap();

            let mut store = KvStore::open(current_dir()?)?;
            match store.remove(key.to_string()) {
                Ok(()) => {}
                Err(KvsError::KeyNotFound) => {
                    println!("Key not found");
                    exit(1);
                }
                Err(e) => return Err(e),
            }
        }
        _ => unreachable!()
    }
    Ok(())
}