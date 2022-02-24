## 解决锁的方法

1. 使用消息传递
2. 将锁进行分片

## await中使用锁
如果 tokio::pawn 一个任务来执行下面的函数的话，会报错, 需要提前释放锁(锁没有实现Send)
```rs
use std::sync::{Mutex, MutexGuard};

async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
    *lock += 1;

    do_something_async().await;
} // 锁在这里超出作用域
```
1. 让锁提前超出作用域
```rs
// 下面的代码可以工作！
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    {
        let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
        *lock += 1;
    } // lock在这里超出作用域 (被释放)

    do_something_async().await;
}
```
2. 重构代码：在 .await 期间不持有锁

之前的代码其实也是为了在 .await 期间不持有锁，但是我们还有更好的实现方式，例如，你可以把 Mutex 放入一个结构体中，并且只在该结构体的非异步方法中使用该锁:
```rs
use std::sync::Mutex;

struct CanIncrement {
    mutex: Mutex<i32>,
}
impl CanIncrement {
    // 该方法不是 `async`
    fn increment(&self) {
        let mut lock = self.mutex.lock().unwrap();
        *lock += 1;
    }
}
```
3. 使用 Tokio 提供的异步锁

Tokio 提供的锁最大的优点就是：它可以在 .await 执行期间被持有，而且不会有任何问题。但是代价就是，这种异步锁的性能开销会更高，因此如果可以，使用之前的两种方法来解决会更好。
```rs

use tokio::sync::Mutex; // 注意，这里使用的是 Tokio 提供的锁
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    let mut lock = mutex.lock().await;
    *lock += 1;

    do_something_async().await;
} // 锁在这里被释放
```