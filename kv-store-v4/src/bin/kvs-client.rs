use std::{net::SocketAddr, process::exit};

use clap::{Parser, AppSettings, Subcommand};

use kvs::{KvsClient, Result};
use log::info;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const ADDRESS_FORMAT: &str = "IP:PORT";

#[derive(Parser, Debug)]
#[clap(name = "kvs-client", about = "A kv store", long_about = "this is long about", author, version)]
#[clap(global_setting(AppSettings::DisableHelpSubcommand))]
struct Opt {
    #[clap(name = "subcommand", subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(name("get"), about("Get the string value of a given string key"), version)]
    // #[clap(global_setting(AppSettings::DisableHelpFlag))]
    #[clap(global_setting(AppSettings::DisableVersionFlag))]
    Get {
        #[clap(name = "key", help = "A String Key")]
        key: String,
        #[clap(
            long,
            help = "Sets the server address",
            value_name = ADDRESS_FORMAT,
            default_value = DEFAULT_LISTENING_ADDRESS,
        )]
        addr: SocketAddr,
    },
    #[clap(name = "set", about = "Set the value of a string key to a string")]
    Set {
        #[clap(name = "KEY", help = "A string key")]
        key: String,
        #[clap(name = "VALUE", help = "The string value of the string key")]
        value: String,
        #[clap(
            long,
            help = "Sets the server address",
            value_name = ADDRESS_FORMAT,
            default_value = DEFAULT_LISTENING_ADDRESS,
        )]
        addr: SocketAddr,
    },
    #[clap(name = "rm", about = "Remove a given string key")]
    Remove {
        #[clap(name = "KEY", help = "A String Key")]
        key: String,
        #[clap(
            long,
            help = "Sets the server address",
            value_name = ADDRESS_FORMAT,
            default_value = DEFAULT_LISTENING_ADDRESS,
        )]
        addr: SocketAddr,
    },
}

fn main() {
    let opt = Opt::parse();
    if let Err(e) = run(opt) {
        eprintln!("{}", e);
        exit(1);
    }
}

fn run(opt: Opt) -> Result<()> {
    match opt.command {
        Command::Get { key, addr } => {
            let mut client = KvsClient::connect(addr)?;
            if let Some(value) = client.get(key)? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        },
        Command::Set { key, value, addr } => {
            let mut client = KvsClient::connect(addr)?;
            client.set(key, value)?;
        },
        Command::Remove { key, addr } => {
            let mut client = KvsClient::connect(addr)?;
            client.remove(key)?;
        },
    }
    Ok(())
}