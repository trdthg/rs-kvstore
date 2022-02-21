use std::thread;

use super::ThreadPool;
use crate::Result;
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, error};

pub struct SharedQueueThreadPool {
    tx: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {
    /// 重建一个线程池
    ///
    /// 线程池保存一个sender用来分发job,
    /// 每个线程都是一个loop, 并不断判断是否接收到任务
    /// 如果线程panic了, 那么就在drop前进行拦截, 并创建一个新线程
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized,
    {
        let (tx, rx) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();
        for i in 0..threads {
            // 为了能够处理一个线程panic的情况, 我们在panic时直接clone rx并创建一个新线程
            // 但是因为孤儿规则, 需要对Receiver做一次包装
            let rx = TaskReceiver(rx.clone());
            thread::Builder::new().spawn(move || run_tasks(rx))?;
        }
        Ok(SharedQueueThreadPool { tx })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx
            .send(Box::new(job))
            .expect("The thread pool has no thread.");
    }
}

#[derive(Clone)]
struct TaskReceiver(Receiver<Box<dyn FnOnce() + Send + 'static>>);

impl Drop for TaskReceiver {
    fn drop(&mut self) {
        // 如果是panic导致的drop, 就新建一个线程
        if thread::panicking() {
            let rx = self.clone();
            if let Err(e) = thread::Builder::new().spawn(move || run_tasks(rx)) {
                error!("Failed to spawn a thread: {}", e);
            }
        }
    }
}

// 线程会一直loop, 判断是否有新任务
fn run_tasks(rx: TaskReceiver) {
    loop {
        match rx.0.recv() {
            Ok(task) => {
                task();
            }
            Err(_) => debug!("Thread exits because the thread pool is destroyed."),
        }
    }
}
