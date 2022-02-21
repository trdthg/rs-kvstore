# 第四版

## 优化Engine
加上trit限制 `Clone + Send + 'static`， 并将参数全部换为 `&self` (不可变借用)，使用`atomic map`(这里借用了crossbeam的SkipMap完成
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
## 实现ThreadPool

### 分配调用栈
> You've got to have a call stack for that thread to run on. That call stack must be allocated. Allocations are pretty cheap, but not as cheap as no allocation. How that call stack is allocated depends on details of the operating system and runtime, but can involve locks and syscalls. Syscalls again are not that expensive, but they are expensive when we're dealing with Rust levels of performance — reducing syscalls is a common source of easy optimizations. That stack then has to be carefully initialized so that first stack frame contains the appropriate values for the base pointer and whatever else is needed in the stack's initial function prologue.
每一个线程都要有自己的调用堆栈，所以开启新的线程需要先为它分配调用栈
- 虽然为调用栈分配内存不是那么expensive，但是不如不分配，使用已经分配过的调用栈

如何分配取决于 system 和 runtime，但是会涉及到锁和系统调用
- 系统调用不是那么expensive，但是对于rust层面来说还是很expensive，所以减少系统调用是一种普遍的简单的优化方式

所以栈必须被小心的分配，最好是之后需要用到的线程都不用在被分配了.
> In Rust the stack needs to be configured with a guard page to prevent stack overflows, preserving memory safety. That takes two more syscalls, to (though on Linux in particular, those two syscalls are avoided).

That's just setting up the callstack. It's at least another syscall to create the new thread, at which point the kernel must do its own internal accounting for the new thread.
在rust里，栈空间的初始化需要同时配置一个`guard page`，防止stack overflows，这个过程还需要至少两个syscalls，这还仅仅是一个栈的初始化，启动一个新线程有需要至少一个系统调用，内核必须去为这个新线程做一些accounting(内部核算)
在rust里，这些过程可以C的 libthread库完成

### 线程间切换
> Then at some point the OS performs a context switch onto the new stack, and the thread runs. When the thread terminates all that work needs to be undone again.
接着在某个时刻，操作系统会将上下文切换到新的Stack上，并运行新线程，当这个线程终止后，所有之前做的工作会被再次撤销

> With a thread pool, all that setup overhead is only done for a few threads, and subsequent jobs are simply context switches into existing threads in the pool.
拥有一个threadpool，上面所述的所有setup过程的开销都只会在少数几个已有的线程中完成，后续作业只是将上下文切换到已有的线程

### 如何实现
一个queue，用来保存线程，将新的job分配给队列中空闲的线程

- 处理panic的job
    如果一个线程崩溃了，线程池需要有恢复策略
- 处理shutdown
    当线程超出作用域后？？？

## 实现无锁的读
- 读与压缩
- 识别immutable value
- 与其共享，尽量使用clone
- 按照功能(读写)分解struct

