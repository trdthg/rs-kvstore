use std::process::exit;

use clap::{App, AppSettings, Arg};

fn main() {

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        // .setting(AppSettings::DisableVersionFlag)
        .subcommand(App::new("set")
            .about("Set the value of a string key to string")
            .arg(Arg::new("KEY").help("A string key").required(true))
            .arg(Arg::new("VALUE").help("A string value of the key").required(true))
        )
        .subcommand(App::new("get")
            .about("Get the value of a string key")
            .arg(Arg::new("KEY").help("A string key").required(true))
        )
        .subcommand(App::new("rm")
            .about("Remove a given string key")
            .arg(Arg::new("KEY").help("A string key").required(true))
        )
        .get_matches();

        let mut map = kvs::KvStore::new();

        match matches.subcommand() {
            Some(("set", args)) => {
                // if let (Some(key), Some(value)) = (args.value_of("KEY"), args.value_of("VALUE")) {
                //     map.set(key.to_string(), value.to_string());
                // }

            eprintln!("unimplemented");
            exit(1);
            },
            Some(("get", args)) => {

            eprintln!("unimplemented");
            exit(1);
                // if let Some(key) = args.value_of("KEY") {
                //     let value = map.get(key.to_string());
                //     println!("{}", value.unwrap_or("None".to_owned()));
                // }
            },
            Some(("rm", args)) => {

            eprintln!("unimplemented");
            exit(1);
                // if let Some(key) = args.value_of("KEY") {
                //     map.remove(key.to_owned());
                // }
            },
            _ => unreachable!()
        }
}

#[test]
fn test() {
    let res = App::new("myprog")
    .setting(AppSettings::DisableHelpSubcommand)
    // Normally, creating a subcommand causes a `help` subcommand to automatically
    // be generated as well
    .subcommand(App::new("test"))
    .try_get_matches_from(vec![
        "myprog", "help"
    ]);
assert!(res.is_err());
assert_eq!(res.unwrap_err().kind, clap::ErrorKind::UnknownArgument);
}