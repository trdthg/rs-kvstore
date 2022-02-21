use std::{env::current_dir, fs, net::SocketAddr, process::exit, str::FromStr};

use clap::{ArgEnum, Parser};
use log::{error, info, warn, LevelFilter};

use kvs::{
    thread_pool::{NaiveThreadPool, RayonThreadPool, ThreadPool},
    KvStore, KvsEngine, KvsError, KvsServer, Result, SledKvsEngine,
};

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const PORT_FORMAT: &str = "IP:PORT";
const DEFAULT_ENGINE: Engine = Engine::kvs;

#[derive(Parser, Debug)]
#[clap(name = "kvs-server", author, version, about, long_about = None)]
struct Opt {
    #[clap(
        short,
        long,
        value_name = PORT_FORMAT,
        default_value = DEFAULT_LISTENING_ADDRESS,
        help = "Sets the listening address",
    )]
    addr: SocketAddr,
    #[clap(
        arg_enum,
        long,
        help = "Sets the storage engine",
        value_name = "ENGINE_NAME"
    )]
    engine: Option<Engine>,
}

#[allow(non_camel_case_types)]
#[derive(ArgEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum Engine {
    kvs,
    sled,
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Engine::kvs => write!(f, "kvs"),
            Engine::sled => write!(f, "sled"),
        }
    }
}

impl FromStr for Engine {
    type Err = KvsError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "kvs" => Ok(Engine::kvs),
            "sled" => Ok(Engine::sled),
            _ => Err(KvsError::NoSuchEngine),
        }
    }
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let mut opt = Opt::parse();
    let res = current_engine().and_then(|curr_engine| {
        // 用户没有输入，尝试从文件中找出
        if opt.engine.is_none() {
            opt.engine = curr_engine;
        }
        // 用户输入的引擎与文件中已有的不相同，不能随便切换引擎
        if curr_engine.is_some() && opt.engine != curr_engine {
            error!("Wrong engine");
            exit(1);
        }
        run(opt)
    });
    if let Err(e) = res {
        error!("{}", e);
        exit(1);
    }
}

fn run(opt: Opt) -> Result<()> {
    let engine = opt.engine.unwrap_or(DEFAULT_ENGINE);
    info!("kvs-server {}", env!("CARGO_PKG_VERSION"));
    info!("Storage engine: {}", engine);
    info!("Listening on {}", opt.addr);

    // write engine to engine file
    fs::write(current_dir()?.join("engine"), format!("{}", engine))?;

    let pool = RayonThreadPool::new(num_cpus::get() as u32)?;

    match engine {
        Engine::kvs => run_with_engine(KvStore::open(current_dir()?)?, pool, opt.addr),
        Engine::sled => run_with_engine(
            SledKvsEngine::new(sled::open(current_dir()?)?),
            pool,
            opt.addr,
        ),
    }
}

fn run_with_engine<E: KvsEngine, P: ThreadPool>(
    engine: E,
    pool: P,
    addr: SocketAddr,
) -> Result<()> {
    let mut server = KvsServer::new(engine, pool);
    server.run(addr)
}

fn current_engine() -> Result<Option<Engine>> {
    // 尝试从engine文件中读取选择的engine类型
    let engine = current_dir()?.join("engine");
    if !engine.exists() {
        return Ok(None);
    }
    // 将字符串转换为枚举类型
    match fs::read_to_string(engine)?.parse() {
        Ok(engine) => Ok(Some(engine)),
        Err(e) => {
            warn!("The content of engine file is invalid: {}", e);
            Ok(None)
        }
    }
}
