use std::collections::HashMap;

use bytes::Bytes;
use mini_redis::*;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;
type ShardedDb = Arc<Vec<Mutex<HashMap<String, Vec<u8>>>>>;

fn new_sharded_db(num_shards: usize) -> ShardedDb {
    let mut db = Vec::with_capacity(num_shards);
    for _ in 0..num_shards {
        db.push(Mutex::new(HashMap::new()));
    }
    Arc::new(db)
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    // 单个db
    // let db = Arc::new(Mutex::new(HashMap::new()));

    // 分片db
    let db = new_sharded_db(9);

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        let dbc = db.clone();
        tokio::spawn(async move {
            handle_connection(stream, dbc).await;
        });
    }
}

async fn handle_connection(stream: TcpStream, shared_db: ShardedDb) {
    let mut connection = Connection::new(stream);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        // tokio::time::sleep(Duration::from_secs(2)).await;
        println!("{frame}");
        let response = match Command::from_frame(frame).unwrap() {
            Command::Set(cmd) => {
                // 使用一种hash算法选中db分片
                let mut db = shared_db[cmd.key().len() % shared_db.len()].lock().unwrap();

                // 值被存储为 `Vec<u8>` 的形式
                db.insert(cmd.key().to_string(), cmd.value().to_vec().into());
                Frame::Simple("OK".to_string())
            }
            Command::Get(cmd) => {
                let db = shared_db[cmd.key().len() % shared_db.len()].lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    // `Frame::Bulk` 期待数据的类型是 `Bytes`， 该类型会在后面章节讲解，
                    // 此时，你只要知道 `&Vec<u8>` 可以使用 `into()` 方法转换成 `Bytes` 类型
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };
        connection.write_frame(&response).await.unwrap();
    }
}
