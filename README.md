# Some Kv Store 😈

all the code are learned from
[Talent-Plan Path 5](https://github.com/pingcap/talent-plan/tree/master/courses/rust)

## Project-1: Base on hashmap

- it's just a hashmap.

## Project-2: Log Structure with Hash index

**Features:**

- data are stored in files like _1.log_, _2.log_ ... _n.log_. `n` means
  generation: every time the store open, it will create a new log file, and the
  new `struct Command` will be seralized and stored in the new file.
- index: index is maintenced in a `BTreeMap`, when the store open, it will read
  all log file and create a btreemap with key and `CommandPos`.
- compact: compact logs when reach the threadhold.
- peristence: use `serde_json` crate

## Project-3: Client & Server

**Feature:**

- comunicate with a custom protocal.
- use Trait to make pluggable kv storage engines.
- add sled as another KvStore Engine.
- use failure to handle error.

## Project-4: Currency & ThreadPool

**Feature:**

- use lock-free reader between threads
- use `Arc<Mutex<T>>` for writer
- implement a easy threadpool

```rs
pub trait KvsEngine: Clone + Send + 'static {

    fn set(&self, key: String, value: String) -> Result<()>;

    fn get(&self, key: String) -> Result<Option<String>>;

    fn remove(&self, key: String) -> Result<()>;

}
```

## 🚧 Project-5: Async

## Other implement for play & fun 😀

- Base64
- Segment-Tree
