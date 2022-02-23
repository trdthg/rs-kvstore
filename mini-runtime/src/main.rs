use std::{
    ops::Add,
    pin::Pin,
    thread::{yield_now, JoinHandle},
};

mod timer_future;

use {
    futures::{
        future::{BoxFuture, FutureExt},
        task::{waker_ref, ArcWake},
    },
    std::{
        future::Future,
        sync::mpsc::{sync_channel, Receiver, SyncSender},
        sync::{Arc, Mutex},
        task::{Context, Poll},
        time::Duration,
    },
    timer_future::TimerFuture,
};

struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

impl Executor {
    fn run(&self) {
        let mut count = 0;
        // 不断尝试接受future
        while let Ok(task) = self.ready_queue.recv() {
            println!("{count}");
            count += 1;
            // 这里使用take拿走task对future的所有权，如果future是Some()就表示任务未完成
            // 尝试poll以下，如果完成，就不归还了None，如果没完成，就把future的所有权归还给task
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                // task实现了ArcWake，自己就是一个waker，能够唤醒自己
                // 这里创建了一个waker的引用
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                // 带着waker，poll一下
                if future.as_mut().poll(context).is_pending() {
                    // 没有完成，归还task对future的所有权, 并执行下一个任务
                    // 当前任务在`有结果`之后会被再次唤醒(调用wake方法)，并进入这个队列中
                    // - 如何判断有没有进展？
                    //   使用新线程轮询fd | socket
                    //   IO多路复用机制，mio库😎
                    // - 但是wake方法是谁调用？
                    //   操作系统🍀
                    // - IO多路复用的大致流程
                    //   blocker存储future事件 [{ id: xxx, signals: xxx }]
                    //   每当发生IO事件(blocker发现有数据可以读取), 就把事件分发到Waker里，Waker会调用wake方法🤩
                    // - 总结
                    //   只需要额外一个执行器线程，就能够管理这些future
                    *future_slot = Some(future);
                }
            }
        }
    }
}

struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    /// 用于将传来的future封装为task，并send到任务队列中
    fn spawn<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let future = f.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.send(task).expect("任务对列满了");
    }
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,
    task_sender: SyncSender<Arc<Task>>,
}
impl ArcWake for Task {
    /// 在poll之前，需要把task送进工作队列中(即调用wake方法)，task能够把自己推到工作队列中
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self.task_sender.send(cloned).expect("任务对列已满")
    }
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
    const MAX_QUEUE_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = sync_channel(MAX_QUEUE_TASKS);
    (Executor { ready_queue }, Spawner { task_sender })
}

fn main() {
    let (executor, spawner) = new_executor_and_spawner();
    let deadline = std::time::Instant::now().add(Duration::from_secs(2));
    // 生成一个任务
    spawner.spawn(async move {
        println!("howdy1!");
        // 创建定时器Future，并等待它完成
        // TimerFuture::new(Duration::new(2, 0)).await;
        while std::time::Instant::now() < deadline {
            yield_now();
        }
        println!("end!");
    });
    // 生成一个任务
    spawner.spawn(async move {
        println!("howdy2!");
        // 创建定时器Future，并等待它完成
        // TimerFuture::new(Duration::new(2, 0)).await;
        while std::time::Instant::now() < deadline {
            yield_now();
        }
        println!("end!2");
    });

    // drop掉任务，这样执行器就知道任务已经完成，不会再有新的任务进来
    drop(spawner);

    // 运行执行器直到任务队列为空
    // 任务运行后，会先打印`howdy!`, 暂停2秒，接着打印 `done!`
    executor.run();
}

#[test]
fn sleep_with_new_thread() {
    // 同步 - 非阻塞 (轮询实现)
    let state1 = std::sync::Arc::new(std::sync::Mutex::new(false));
    let state2 = std::sync::Arc::new(std::sync::Mutex::new(false));
    let c1 = state1.clone();
    let c2 = state2.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        *c1.lock().unwrap() = true;
        println!("done1");
    });
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        *c2.lock().unwrap() = true;
        println!("done2");
    });
    loop {
        if state1.lock().unwrap().to_owned() == true && state2.lock().unwrap().to_owned() == true {
            break;
        }
    }
}

#[test]
fn sleep_with_no_new_thread() {
    // 同步 - 非阻塞 (yield实现)
    let now = std::time::Instant::now().add(Duration::from_secs(2));
    fn aa(deadline: std::time::Instant, msg: String) {
        while std::time::Instant::now() < deadline {
            yield_now();
        }
        // 通过不断yield交替执行，不会阻塞当前线程
        println!("done {msg:?}");
    }
    aa(now, "aaa".to_string());
    aa(now, "bbb".to_string());
}

#[test]
fn async_sleep_with_new_thread() {
    // 异步 - 非阻塞 (线程实现)
    let now = std::time::Instant::now().add(Duration::from_secs(2));
    fn aa(msg: String) -> JoinHandle<()> {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            // 这行打印相当于callback，callback在新线程内，不会阻塞当前线程
            println!("done {msg:?}");
        })
    }
    let a = aa("aaa".to_string());
    let b = aa("bbb".to_string());
    a.join().unwrap();
    b.join().unwrap();
}
