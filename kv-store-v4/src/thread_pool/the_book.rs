use crate::Result;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// 来自The Book的最后一章
pub struct TheBookThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

enum Message {
    NewJob(Job),
    Terminate,
}
type Job = Box<dyn FnOnce() + Send + 'static>;

impl super::ThreadPool for TheBookThreadPool {
    fn new(size: u32) -> Result<Self> {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size as usize);
        for id in 0..size {
            // creeate some threads and store them in the vec
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Ok(Self { workers, sender })
    }

    fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

struct Worker {
    id: u32,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: u32, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            // loop行
            // reason: lock 方法返回的 MutexGuard 在 let job 语句结束之后立刻就被丢弃了。这确保了 recv 调用过程中持有锁，而在 job() 调用前锁就被释放了，这就允许并发处理多个请求了。
            loop {
                // let job = receiver.lock().unwrap().recv().expect("Failed recv");
                // println!("Worker {} got a job; executing.", id);
                // job();

                let message = receiver.lock().unwrap().recv().unwrap();
                match message {
                    Message::NewJob(job) => {
                        // println!("Worker {} got a job; executing.", id);
                        job();
                    }
                    Message::Terminate => {
                        // println!("Worker {} was told to terminate.", id);
                        break;
                    }
                }
            }
            // while不行, 慢请求仍然会导致阻塞
            // reason: Mutex结构体没有共有的unlock()方法, 因为锁的所有权依赖于lock()方法返回的LockResult<MutexGuard<T>>中MutexGuard<T>的生命周期
            //         因为 while 表达式中的值在整个块一直处于作用域中，job() 调用的过程中其仍然持有锁，这意味着其他 worker 不能接收任务。
            // while let Ok(job) = receiver.lock().unwrap().recv() {
            //     println!("Worker {} got a job; executing.", id);
            //     job()
            // }
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

// Graceful Shutdown and Cleanup
impl Drop for TheBookThreadPool {
    fn drop(&mut self) {
        // println!("Sending terminate message to all workers.");
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }
        // println!("Shutting down all workers.");
        for worker in &mut self.workers {
            // println!("Shutting down worker {}", worker.id);
            worker.thread.take().map(|thread| thread.join().unwrap());
        }
    }
}
