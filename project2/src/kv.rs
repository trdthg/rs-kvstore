use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::ops::Range;
use std::path::{PathBuf, Path};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};

use serde_derive::{Deserialize, Serialize};
use serde_json::{Deserializer};

use crate::{KvsError, Result};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

// 优化空间
// 索引加载慢
// 压缩操作可以使用异步子线程
// 判断是否超过阈值这个过程现在在set中，会阻塞写入操作，可以优化为定时检查

/// Hash索引，追加式日志写入
/// 优点：
/// - 顺序写入性能好，比随机写入快
/// - 处理并发和崩溃简单，如果发生写入崩溃，只需要丢弃文件末尾的数据
/// - 可以通过合并日志文件减少文件碎片
/// 缺点：
/// - 不支持快速区间查询
/// - Hash索引需要把Key全部放入内存

/// The `KvStore` stores key/value pairs
///
/// Key/value pairs are persisted to disk in log files. Log files are named after
/// monotonically increasing generation numbers with a `log` extension name.
/// A `BreeMap` in member stores the keys and the value locations for fast query.
///
/// 基于日志的KV数据库，分段日志，超过阈值之后能够压缩
///
/// set能向日志末尾追加一行数据，性能非常好，但是查询就相对较慢，为了加快查询效率，这里使用了哈希索引加快查询速度
/// 但是维护索引也会影响写入性能，所以需要适当的权衡，维护适当的索引能够加快查询速度，但也会损失一部分写入性能
/// ```rust
/// use kvs::{KvStore, Result};
/// fn try_main() -> Result<()> {
///     let mut a = 1;
///     Ok(())
/// }
/// ```
pub struct KvStore {
    /// directory for the log and other data.
    /// 存储日志的目录
    /// `PathBuf`与`Path`内部维护的是`OsString`，关系就好比`String`和`Str`，具体可以看看源码
    path: PathBuf,

    /// map generation number to the file reader.
    /// 键是文件的版本号
    /// 值就是该文件对应的Reader
    readers: HashMap<u64, BufReaderWithPos<File>>,

    /// writer of the current log.
    ///
    /// 每次启动是就会新建一个log文件，Writer只负责向这个新的文件写入
    writer: BufWriterWithPos<File>,

    /// 最新的版本号
    current_gen: u64,

    /// 索引，键对应数据的键，值代表了插入操作是在那个版本发生的，插入文件的位置是哪里
    index: BTreeMap<String, CommandPos>,

    /// the number of bytes representing "stale" commands that could be deleted during a compaction.
    ///
    /// 用于压缩，保存可以被稳定删除的Command的数量
    uncompacted: u64,

}

impl KvStore {
    /// Opens a `KvStore` with the given path.
    ///
    /// This will create a new directory if the given one does not exist.
    ///
    /// # Errors
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore>{
        // 加载日志目录
        let path = path.into();
        fs::create_dir_all(&path)?;

        let mut readers = HashMap::new();
        let mut index = BTreeMap::new();

        // 初始化现有的所有日志文件，按照大小排序
        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        // 为每个日志创建一个Reader，顺便统计总可压缩数量
        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }
        // 获取最新的版本号(还有即将创建的，所以要+1)
        let current_gen = gen_list.last().unwrap_or(&0) + 1;

        // 为新的文件new一个Writer
        let writer = new_log_file(&path, current_gen, &mut readers)?;

        Ok(KvStore {
            path,
            readers,
            writer,
            current_gen,
            index,
            uncompacted,
        })
    }

    /// Sets the value of a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    ///
    /// # Errors
    /// It propagates I/O or serialization errors during writing the log.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        // 封装下Command
        let cmd = Command::set(key, value);
        // 拿到现在writter的指针位置
        let pos = self.writer.pos;
        // 使用serde序列化并写入Command结构体
        serde_json::to_writer(&mut self.writer, &cmd)?;
        // 清空缓冲区
        self.writer.flush()?;
        // 处理压缩相关，如果覆盖了就+1
        if let Command::Set {key, ..} = cmd {
            if let Some(old_cmd) = self.index.insert(key, (self.current_gen, pos..self.writer.pos).into()) {
                self.uncompacted += old_cmd.len;
            }
        }
        // 如果可压缩数量超过临界值，就执行压缩
        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the given key does not exist.
    ///
    /// # Errors
    ///
    /// It returns `KvsError::UnexpectedCommandType` if the given command type unexpected.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        // 从索引中拿到CommandPos
        if let Some(cmd_pos) = self.index.get(&key) {
            // 从map中拿到对应的Reader
            if let Some(reader) = self.readers.get_mut(&cmd_pos.gen) {
                // 将游标的位置移动到命令所在的起始位置
                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                // 这里相当于又作了一个缓冲区，指定缓冲区的大小就是数据长度的大小
                let cmd_reader = reader.take(cmd_pos.len);
                // 尝试从缓冲区中反序列化出结果
                if let Some(Command::Set { value, .. }) = serde_json::from_reader(cmd_reader)? {
                    Ok(Some(value))
                } else {
                    Err(KvsError::UnExpectedCommandType)
                }
            } else {
                Err(KvsError::ReaderNotFound)
            }
        } else {
            Ok(None)
        }
    }


    /// Removes a given key.
    ///
    /// # Errors
    ///
    /// It returns `KvsError::KeyNotFound` if the given key is not found.
    ///
    /// It propagates I/O or serialization errors during writing the log.
    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::remove(key.clone());
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;
            if let Some(old_cmd) = self.index.remove(&key) {
                self.uncompacted += old_cmd.len
            }
            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }


    /// Clears stale entries in the log.
    ///
    /// 压缩日志
    ///
    /// 压缩能够把大量零散的日志文件整合在一起，并能把冗余的操作也给合并(Set操作是被合并，Remove操作就直接没了)掉
    /// 因为readers中虽然保存了所有数据，但是索引树只记录了当前数据库中拥有的数据对应的最后一次Set Command
    /// 所以在下面遍历索引树拷贝数据的过程中，就把冗余的操作去掉了
    pub fn compact(&mut self) -> Result<()> {
        // 初始化压缩过程需要的数据，压缩单独占据一个版本
        // 启动一个新版本给压缩单独使用
        let compaction_gen = self.current_gen + 1;
        let mut compation_writer = self.new_log_file(compaction_gen)?;
        let mut compation_pos = 0;

        // 更新self的信息，writer会再次启动一个新的版本, 所以self.current_gen +2
        self.current_gen += 2;
        self.writer = self.new_log_file(self.current_gen)?;

        // 开始压缩，遍历索引树的value(CommandPos)，把数据从原来的地方复制到压缩版本里，并更新索引
        for cmd_pos in self.index.values_mut() {
            if let Some(reader) = self.readers.get_mut(&cmd_pos.gen) {

                // 定位游标到Command的起始位置
                // reader.pos在load之后默认情况下为0, 通过read和seek操作后会更新
                if reader.pos != cmd_pos.pos {
                    reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                }

                // 用这条Command的位置信息，使用对应的Reader去读取到记录
                let mut entry_reader = reader.take(cmd_pos.len);
                // 直接从数据拷贝到压缩过程使用的日志文件中
                let len = io::copy(&mut entry_reader, &mut compation_writer)?;

                // 这里是更新self的索引树，如果有重复的操作
                *cmd_pos = (compaction_gen, compation_pos..compation_pos + len).into();

                // 更新压缩版本的指针位置
                compation_pos += len;
            } else {
                return Err(KvsError::ReaderNotFound);
            }
        }

        // 删除之前的日志文件，并把压缩数归零
        let stable_gens: Vec<_> = self
            .readers
            .keys()
            .filter(|&&gen| gen < compaction_gen)
            .cloned()
            .collect();

        for stablegen in stable_gens {
            self.readers.remove(&stablegen);
            fs::remove_file(log_path(&self.path, stablegen))?;
        }

        self.uncompacted = 0;

        Ok(())
    }

    fn new_log_file(&mut self, gen: u64) -> Result<BufWriterWithPos<File>> {
        new_log_file(&self.path, gen, &mut self.readers)
    }
}


/// 读取目录中的所有文件，找出这些文件的版本号，然后排序
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)?
        .flat_map(|res| -> Result<PathBuf> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();
    gen_list.sort_unstable();
    Ok(gen_list)
}

/// just join dir path and file path, the default log file extension if `.log`
fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// Load the whole log file and store value locations in the index map.
/// Return how many butes can be saved after a compation.
///
/// load会加载所有日志的索引
/// 我们这里没有单独的索引文件，因为数据文件使用serde序列化了Command结构体，而是直接从数据文件中遍历所有数据组合出索引文件
/// 加载一个日志文件，向索引树中添加所有的操作纪录(Key, CommandPos)
/// 返回可以压缩的数量
fn load(gen: u64, reader: &mut BufReaderWithPos<File>, index: &mut BTreeMap<String, CommandPos>) -> Result<u64> {

    // 加载某个版本的日志文件
    let mut pos = reader.seek(SeekFrom::Start(0))?;

    // 反序列化为Stream流
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
    let mut uncompacted = 0;

    // 以此向索引中添加command记录，并统计可压缩数量
    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            // 如果是插入就将key加入到索引
            Command::Set {key, ..} => {
                // 如果有重复插入的动作就意味着日志可以被压缩的数量+1
                if let Some(old_cmd) = index.insert(key, (gen, pos..new_pos).into()) {
                    uncompacted += old_cmd.len;
                }
            },
            // 如果是删除就将key从索引删除
            Command::Remove {key} => {
                // set set set remove

                // 如果有删除的动作，就意味着日志被压缩的数量+1
                // 这里+1值的是上一次set
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.len;
                }
                // 还要加上remove自身
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos;
    }
    Ok(uncompacted)
}

fn new_log_file(path: &Path, gen: u64, readers: &mut HashMap<u64, BufReaderWithPos<File>>) -> Result<BufWriterWithPos<File>> {
    let path = log_path(path, gen);
    let writer = BufWriterWithPos::new(OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&path)?
    )?;
    readers.insert(gen, BufReaderWithPos::new(File::open(path)?)?);
    Ok(writer)
}


/// Command 相关
#[derive(Deserialize, Serialize, Debug)]
enum Command {
    Set {key: String, value: String},
    Remove {key: String},
}


impl Command {
    fn set(key: String, value: String) -> Self {
        Command::Set{key, value}
    }

    fn remove(key: String) -> Self {
        Command::Remove{key}
    }
}

/// Represents the position and length of a json-seralizaed command in the log file
///
/// 保存了一条Command在日志中的位置
/// pos是位置，len是长度
#[derive(Debug)]
struct CommandPos {
    gen: u64,
    pos: u64,
    len: u64,
}

impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

/// IO相关
///
/// 重要的就是封装的reader和pos，pos保存了每次read， write，seek操作后的位置
/// 因为read和write后会根据返回的大小更新pos的位置
#[derive(Debug)]
struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl <R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut[u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl <R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

#[derive(Debug)]
struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl <W: Write + Seek> BufWriterWithPos<W> {
    pub fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos
        })
    }
}

impl <W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl <W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}