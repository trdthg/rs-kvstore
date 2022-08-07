use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use log::{debug, error, info, warn};
use serde_json::Deserializer;

use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::thread_pool::ThreadPool;
use crate::{KvsEngine, Result};

pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    engine: E,
    pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    pub fn new(engine: E, pool: P) -> Self {
        KvsServer { engine, pool }
    }

    pub fn run(self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let engine = self.engine.clone();
            self.pool.spawn(move || match stream {
                // 对于每一个连接，我们都搞一个线程去处理
                Ok(stream) => {
                    if let Err(e) = serve(engine, stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => {
                    error!("Connection failed: {}", e);
                }
            })
        }
        Ok(())
    }
}

pub fn serve<E: KvsEngine>(engine: E, tcp: TcpStream) -> Result<()> {
    let peer_addr = tcp.peer_addr()?;
    let reader = BufReader::new(&tcp);
    let mut writer = BufWriter::new(&tcp);
    let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();
    warn!("{}", peer_addr);
    macro_rules! send_resp {
        ($resp:expr) => {{
            let resp = $resp;
            serde_json::to_writer(&mut writer, &resp)?;
            writer.flush()?;
            debug!("Response sent to {}: {:?}", peer_addr, resp);
            warn!("Response sent to {}: {:?}", peer_addr, resp);
        }};
    }

    for req in req_reader {
        let req = req?;
        debug!("Receive request from {}: {:?}", peer_addr, req);
        match req {
            Request::Get { key } => send_resp!(match engine.get(key) {
                Ok(value) => GetResponse::Ok(value),
                Err(e) => GetResponse::Err(format!("{}", e)),
            }),
            Request::Set { key, value } => send_resp!(match engine.set(key, value) {
                Ok(_) => SetResponse::Ok(()),
                Err(e) => SetResponse::Err(format!("{}", e)),
            }),
            Request::Remove { key } => send_resp!(match engine.remove(key) {
                Ok(_) => RemoveResponse::Ok(()),
                Err(e) => RemoveResponse::Err(format!("{}", e)),
            }),
        }
    }
    Ok(())
}
