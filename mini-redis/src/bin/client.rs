use std::time::Duration;

use tokio::sync::oneshot;

enum Command<T>
where
    T: ToString,
{
    Get {
        key: T,
        sender: oneshot::Sender<crate::Result<Option<bytes::Bytes>>>,
    },
    Set {
        key: T,
        value: T,
        expaire: Option<Duration>,
        sender: oneshot::Sender<mini_redis::Result<()>>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let tx1 = tx.clone();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = tx1
            .send(Command::Set {
                key: "a",
                value: "bytes".into(),
                expaire: Some(Duration::from_secs(2)),
                sender: tx,
            })
            .await
        {
            panic!("[ERROR] error occurs: {e}");
        }
        let res = rx.await;
        panic!("[INFO] get : {res:?}");
    });
    tokio::spawn(async move {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = tx2
            .send(Command::Get {
                key: "a",
                sender: tx,
            })
            .await
        {
            panic!("[ERROR] error occurs: {e}");
        }
        let res = rx.await;
        println!("[INFO] get : {res:?}");
    });
    drop(tx);
    let mut client = client::connect("127.0.0.1:6379").await?;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Set {
                key,
                value,
                expaire,
                sender,
            } => {
                let res = client.set(key, value.as_bytes().into()).await;
                sender.send(res).unwrap();
            }
            Command::Get { key, sender } => {
                let res = client.get(key).await;
                sender.send(res).unwrap();
            }
            _ => unimplemented!(),
        }
    }
    Ok(())
}
