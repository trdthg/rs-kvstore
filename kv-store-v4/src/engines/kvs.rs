use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_skiplist::SkipMap;
use log::error;
use serde_derive::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::{KvsEngine, KvsError, Result};

const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// The `KvStore` stores key/value pairs
/// 这次是一个共享的引擎，每个引擎都有一个 reader 和 writer
/// 结构体的所有属性都使用 Arc 包裹
/// 读写分为了两个结构体，各自封装了各自需要的属性
/// 如果是重复的树形，就用 Arc::clone 共享
#[derive(Clone)]
pub struct KvStore {
    path: Arc<PathBuf>,

    /// 封装了之前的逻辑：内部每一个日志就对应一个 reader
    reader: KvStoreReader,

    /// 每次启动是就会新建一个 log 文件，Writer 只负责向这个新的文件写入
    // writer: BufWriterWithPos<File>,
    writer: Arc<Mutex<KvStoreWriter>>,

    /// 索引：这次使用 crossbeam 提供的 skipmap 实现无锁并发
    // index: BTreeMap<String, CommandPos>,
    index: Arc<SkipMap<String, CommandPos>>,
    // 都被封装进了 writer
    // current_gen: u64,
    // uncompacted: u64,
}

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        // 加载日志目录
        let path = Arc::new(path.into());
        fs::create_dir_all(&*path)?;

        let mut readers = BTreeMap::new();
        let index = Arc::new(SkipMap::new());

        // 初始化现有的所有日志文件，按照大小排序
        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        // 为每个日志创建一个 Reader，顺便统计总可压缩数量
        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &*index)?;
            readers.insert(gen, reader);
        }

        // 获取最新的版本号 (还有即将创建的，所以要 +1)
        let current_gen = gen_list.last().unwrap_or(&0) + 1;

        // 反正就是一个原子的 u64
        let safe_point = Arc::new(AtomicU64::new(0));

        let reader = KvStoreReader {
            path: Arc::clone(&path),
            safe_point,
            readers: RefCell::new(readers),
        };

        let writer = new_log_file(&path, current_gen)?;

        // 为新的文件 new 一个 Writer
        let writer = KvStoreWriter {
            reader: reader.clone(),
            writer,
            current_gen,
            uncompacted,
            path: Arc::clone(&path),
            index: Arc::clone(&index),
        };

        Ok(KvStore {
            path,
            reader,
            writer: Arc::new(Mutex::new(writer)),
            index,
        })
    }
}

impl KvsEngine for KvStore {
    fn set(&self, key: String, value: String) -> Result<()> {
        self.writer.lock().unwrap().set(key, value)
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            if let Command::Set { value, .. } = self.reader.read_command(*cmd_pos.value())? {
                Ok(Some(value))
            } else {
                Err(KvsError::UnexpectedCommandType)
            }
        } else {
            Ok(None)
        }
    }

    fn remove(&self, key: String) -> Result<()> {
        self.writer.lock().unwrap().remove(key)
    }
}

/// 每一个`Kvstore`都有自己的 reader，用户使用在多个线程中使用各自的 store 去并发读取
///
///
struct KvStoreReader {
    path: Arc<PathBuf>,
    // generation of the latest compaction file
    safe_point: Arc<AtomicU64>,
    readers: RefCell<BTreeMap<u64, BufReaderWithPos<File>>>,
}

impl KvStoreReader {
    // 关闭经过压缩后**多余**的 handle(&reader)，只在 compact 时 safe_point 才会被设置为 compaction_gen，正常调用
    fn close_stable_handles(&self) {
        let mut readers = self.readers.borrow_mut();
        while !readers.is_empty() {
            let first_gen = *readers.keys().next().unwrap();
            // 当压缩后会将压缩日志 id 的保存进 pointer
            // self.reader.safe_point.store(compaction_gen, Ordering::SeqCst);
            // 所以这里当走到压缩日志处就会 break，应该剩下的是压缩日志和新的写入日志
            if self.safe_point.load(Ordering::SeqCst) <= first_gen {
                break;
            }
            readers.remove(&first_gen);
        }
    }

    fn read_and<F, R>(&self, cmd_pos: CommandPos, f: F) -> Result<R>
    where
        F: FnOnce(io::Take<&mut BufReaderWithPos<File>>) -> Result<R>,
    {
        // read 之前确保老版本被删除
        self.close_stable_handles();
        let mut readers = self.readers.borrow_mut();
        // 判断 readers 里有没有一些日志没有加载进来 (比如压缩日志)
        if !readers.contains_key(&cmd_pos.gen) {
            let reader = BufReaderWithPos::new(File::open(log_path(&self.path, cmd_pos.gen))?)?;
            readers.insert(cmd_pos.gen, reader);
        }
        let reader = readers.get_mut(&cmd_pos.gen).unwrap();
        reader.seek(SeekFrom::Start(cmd_pos.pos))?;
        let cmd_reader = reader.take(cmd_pos.len);
        f(cmd_reader)
    }

    fn read_command(&self, cmd_pos: CommandPos) -> Result<Command> {
        self.read_and(cmd_pos, |reader| Ok(serde_json::from_reader(reader)?))
    }
}

impl Clone for KvStoreReader {
    fn clone(&self) -> Self {
        // Self { path: Arc::clone(&self.path), safe_point: Arc::clone(&self.safe_point), readers: Arc::clone(&self.readers) }
        Self {
            path: Arc::clone(&self.path),
            safe_point: Arc::clone(&self.safe_point),
            readers: RefCell::new(BTreeMap::new()),
        }
    }
}

struct KvStoreWriter {
    reader: KvStoreReader,
    writer: BufWriterWithPos<File>,
    current_gen: u64,
    uncompacted: u64,
    path: Arc<PathBuf>,
    index: Arc<SkipMap<String, CommandPos>>,
}

/// writer 本身被 mutex 包裹，不需要 mut，调用 writer 前🔓
impl KvStoreWriter {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::set(key, value);
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?;
        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self.index.get(&key) {
                self.uncompacted += old_cmd.value().len;
            }
            self.index
                .insert(key, (self.current_gen, pos..self.writer.pos).into());
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::remove(key);
            let pos = self.writer.pos;
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;

            if let Command::Remove { key } = cmd {
                let old_cmd = self.index.remove(&key).ok_or(KvsError::KeyNotFound)?;
                // 原本有的 Insert 也被压缩
                self.uncompacted += old_cmd.value().len;
                // 新的写入的长度，这个长度是序列化实际写入的长度
                self.uncompacted += self.writer.pos - pos;
            }

            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }

            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    // 向索引中写入数据，但是不会更新 reader，read 会判断有没有 reader，没有再加上
    fn compact(&mut self) -> Result<()> {
        // 这个是压缩版本
        let compaction_gen = self.current_gen + 1;
        let mut compaction_writer = new_log_file(&self.path, compaction_gen)?;

        // 压缩玩要新开一个日志
        self.current_gen += 2;
        self.writer = new_log_file(&self.path, self.current_gen)?;

        // 新建压缩日志
        let mut new_pos = 0;
        for entry in self.index.iter() {
            // let len = self.reader.read_and(*entry.value(), |entry_reader| {
            //     Ok(io::copy(&mut entry_reader, &mut self.writer)?)
            // });
            let cmd = self.reader.read_command(*entry.value())?;
            serde_json::to_writer(&mut compaction_writer, &cmd)?;
            self.index.insert(
                entry.key().clone(),
                (compaction_gen, new_pos..self.writer.pos).into(),
            );
            new_pos += self.writer.pos - new_pos;
        }
        self.writer.flush()?;

        // 关闭之前版本的 handle
        // 这个过程只有在 compact 时发生，刚开始为 0，压缩时 store 为 compaction_gen
        self.reader
            .safe_point
            .store(compaction_gen, Ordering::SeqCst);
        self.reader.close_stable_handles();

        // 删除 handle 对应的日志文件
        // 先拿到之前的所有版本号
        let stable_gens = sorted_gen_list(&self.path)?
            .into_iter()
            .filter(|&gen| gen < compaction_gen);
        for stable_gen in stable_gens {
            let file_path = log_path(&self.path, stable_gen);
            if let Err(e) = fs::remove_file(&file_path) {
                error!("{:?} cannot be deleted: {}", file_path, e);
            }
        }
        self.uncompacted = 0;
        Ok(())
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
/// load 会加载所有日志的索引
/// 我们这里没有单独的索引文件，因为数据文件使用 serde 序列化了 Command 结构体，而是直接从数据文件中遍历所有数据组合出索引文件
/// 加载一个日志文件，向索引树中添加所有的操作纪录 (Key, CommandPos)
/// 返回可以压缩的数量
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &SkipMap<String, CommandPos>,
) -> Result<u64> {
    // 加载某个版本的日志文件
    let mut pos = reader.seek(SeekFrom::Start(0))?;

    // 反序列化为 Stream 流
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
    let mut uncompacted = 0;

    // 以此向索引中添加 command 记录，并统计可压缩数量
    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            // 如果是插入就将 key 加入到索引
            Command::Set { key, .. } => {
                // 如果有重复插入的动作就意味着日志可以被压缩的数量 +1
                if let Some(old_cmd) = index.get(&key) {
                    uncompacted += old_cmd.value().len;
                }
                index.insert(key, (gen, pos..new_pos).into());
            }
            // 如果是删除就将 key 从索引删除
            Command::Remove { key } => {
                // set set set remove

                // 如果有删除的动作，就意味着日志被压缩的数量 +1
                // 这里 +1 值的是上一次 set
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.value().len;
                }
                // 还要加上 remove 自身
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos;
    }
    Ok(uncompacted)
}

fn new_log_file(path: &Path, gen: u64) -> Result<BufWriterWithPos<File>> {
    let path = log_path(path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    Ok(writer)
}

/// Command 相关
#[derive(Deserialize, Serialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

impl Command {
    fn set(key: String, value: String) -> Self {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Self {
        Command::Remove { key }
    }
}

/// Represents the position and length of a json-seralizaed command in the log file
///
/// 保存了一条 Command 在日志中的位置
/// pos 是位置，len 是长度
#[derive(Debug, Clone, Copy)]
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

/// IO 相关
///
/// 重要的就是封装的 reader 和 pos，pos 保存了每次 read，write，seek 操作后的位置
/// 因为 read 和 write 后会根据返回的大小更新 pos 的位置
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

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
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

impl<W: Write + Seek> BufWriterWithPos<W> {
    pub fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
