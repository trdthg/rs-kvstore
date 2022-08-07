## V5

## V4

### 优化 Engine

加上 trit 限制 `Clone + Send + 'static`，并将参数全部换为 `&self`
(不可变借用)，使用`atomic map`(这里借用了 crossbeam 的 SkipMap 完成

```rs
pub trait KvsEngine: Clone + Send + 'static {

    fn set(&self, key: String, value: String) -> Result<()>;

    fn get(&self, key: String) -> Result<Option<String>>;

    fn remove(&self, key: String) -> Result<()>;

}

#[derive(Clone)]
pub struct KvStore {
    path: Arc<PathBuf>,
    reader: KvStoreReader,
    writer: Arc<Mutex<KvStoreWriter>>,
    index: Arc<SkipMap<String, CommandPos>>,
}
```

### 实现 ThreadPool

#### 分配调用栈

> You've got to have a call stack for that thread to run on. That call stack
> must be allocated. Allocations are pretty cheap, but not as cheap as no
> allocation. How that call stack is allocated depends on details of the
> operating system and runtime, but can involve locks and syscalls. Syscalls
> again are not that expensive, but they are expensive when we're dealing with
> Rust levels of performance — reducing syscalls is a common source of easy
> optimizations. That stack then has to be carefully initialized so that first
> stack frame contains the appropriate values for the base pointer and whatever
> else is needed in the stack's initial function prologue.
> 每一个线程都要有自己的调用堆栈，所以开启新的线程需要先为它分配调用栈

- 虽然为调用栈分配内存不是那么 expensive，但是不如不分配，使用已经分配过的调用栈

如何分配取决于 system 和 runtime，但是会涉及到锁和系统调用

- 系统调用不是那么 expensive，但是对于 rust 层面来说还是很 expensive，所以减少系统调用是一种普遍的简单的优化方式

所以栈必须被小心的分配，最好是之后需要用到的线程都不用在被分配了。

> In Rust the stack needs to be configured with a guard page to prevent stack
> overflows, preserving memory safety. That takes two more syscalls, to (though
> on Linux in particular, those two syscalls are avoided).

That's just setting up the callstack. It's at least another syscall to create
the new thread, at which point the kernel must do its own internal accounting
for the new thread. 在 rust 里，栈空间的初始化需要同时配置一个`guard page`，防止 stack
overflows，这个过程还需要至少两个 syscalls，这还仅仅是一个栈的初始化，启动一个新线程有需要至少一个系统调用，内核必须去为这个新线程做一些
accounting(内部核算) 在 rust 里，这些过程可以 C 的 libthread 库完成

#### 线程间切换

> Then at some point the OS performs a context switch onto the new stack, and
> the thread runs. When the thread terminates all that work needs to be undone
> again. 接着在某个时刻，操作系统会将上下文切换到新的 Stack 上，并运行新线程，当这个线程终止后，所有之前做的工作会被再次撤销

> With a thread pool, all that setup overhead is only done for a few threads,
> and subsequent jobs are simply context switches into existing threads in the
> pool. 拥有一个 threadpool，上面所述的所有 setup 过程的开销都只会在少数几个已有的线程中完成，后续作业只是将上下文切换到已有的线程

#### 如何实现

一个 queue，用来保存线程，将新的 job 分配给队列中空闲的线程

- 处理 panic 的 job 如果一个线程崩溃了，线程池需要有恢复策略
- 处理 shutdown 当线程超出作用域后？？？

### 无锁的读

- 读与压缩
- 识别 immutable value
- 与其共享，尽量使用 clone
- 按照功能 (读写) 分解 struct

## V3

client-server 编写基本规范

### client

- 通过 clap 构建脚本
- 通过某项参数直接 match 为不同函数

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

- 读取参数，建立连接 (TcpStream::connect())，
- 发送命令

```rust
// 使用 serde
serde_json::to_writer(&mut some_writer, &some_struct)?;
some_writer,.flush()?;
```

- 读取结果

```rust
match Serialize::deseralize(&mut some_reader)? {

}
```

### server

- 建立 server(TcpListening::bind(addr))
- 监听每个链接并处理

```rust
for stream in listening.incoming() {
    match stream {
        Ok(stream) => {
            let reader = BufReader::new(stream);
            let writer = BufWriter::new(stream);
            // 一个 stream 就对应一个连接，所以还要循环每个链接发出的请求
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

## V2

实现了基本的日志处理框架
