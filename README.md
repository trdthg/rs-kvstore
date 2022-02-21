# Some Kv Store 😈

all the code are learned from [Talent-Plan Path 5](https://github.com/pingcap/talent-plan/tree/master/courses/rust)

## Project-1: Base on hashmap

- it's just a hashmap.

## Project-2: Log Structure with Hash index

**Features:**

- data are stored in files like 1.log, 2.log ... n.log.
- `n` means generation: every time the store open, it will create a new log file, and the new `struct Command` will be seralized and stored in the new file.
- index is maintenced in a BTreeMap, when the store open, it will read all log file and create index(key, CommandPos).
- compact logs when reach the threadhold.

## Project-3: Client & Server

**Feature:**

- comunicate with a custom protocal.
- use Trait to make pluggable kv storage engines.
- add sled as another KvStore Engine.
- use failure to handle error.

## Project-4: Currency & ThreadPool

**Feature:**
- use lock-free reader between threads
- use Arc<Mutex<>> for writer
- implement a easy threadpool

## 🚧 Project-5: Async

## Other implement for play & fun 😀

- Base64
- Segment-Tree
