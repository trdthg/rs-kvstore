# client-server编写基本规范

## client
- 通过clap构建脚本
- 通过某项参数直接match为不同函数
```rust
// 一般格式
#[derive(Parser, Debug)]
#[clap(name = "kvs-client", about = "A kv store", long_about = "this is long about", author, version)]
struct Opt {
    #[clap(name = "subcommand", subcommand)]
    command: Command,
}

#[derive(SubCommand)]
enum Command {
    #[clap(name = "get")]
    Get {
        #[clap(name = "key", help = "A String Key")]
        key: String,
        #[clap(
            long,
            help = "Sets the server address",
            value_name = ADDRESS_FORMAT,
            default_value = DEFAULT_LISTENING_ADDRESS,
        )]
        value: String
    }
}
```
- 读取参数，建立连接(TcpStream::connect())，
- 发送命令
```rust
// 使用serde
serde_json::to_writer(&mut some_writer, &some_struct)?;
some_writer,.flush()?;
```
- 读取结果
```rust
match Serialize::deseralize(&mut some_reader)? {

}
```

## server
- 建立server(TcpListening::bind(addr))
- 监听每个链接并处理
```rust
for stream in listening.incoming() {
    match stream {
        Ok(stream) => {
            let reader = BufReader::new(stream);
            let writer = BufWriter::new(stream);
            // 一个stream就对应一个连接，所以还要循环每个链接发出的请求
            let req_reader = Deserializer::from_reader(reader).into_iter::<>();
            for req in req_reader {
                let req = req?;
                match req {
                    Request::Get {} => {}
                    Request::Set {} => {}
                }
            }
        },
        Err(e) => {}
    }
}
```