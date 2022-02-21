use std::{sync::atomic::{AtomicBool, Ordering, AtomicUsize}, thread};

static X: AtomicBool = AtomicBool::new(false);
static Y: AtomicBool = AtomicBool::new(false);
static Z: AtomicUsize = AtomicUsize::new(0);

fn write_x_then_y() {
    X.store(true, Ordering::Relaxed);
    Y.store(true, Ordering::Relaxed);
}

fn read_y_then_x() {
    while !Y.load(Ordering::Relaxed) {}
    if X.load(Ordering::Relaxed) {
        Z.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test() {
    let t1 = thread::spawn(move || {
        write_x_then_y();
    });

    let t2 = thread::spawn(move || {
        read_y_then_x();
    });

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(Z.load(Ordering::SeqCst), 0);
}
