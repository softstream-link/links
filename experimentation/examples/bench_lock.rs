use std::{
    hint::black_box,
    // sync::atomic::AtomicU64,
    sync::Arc,
    // thread,
    time::Instant,
};

// static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    // atomic_store()
    spin_mutex()
}

// fn atomic_store() {
//     black_box(&A); // New!
//     let n = 1_000_000;

//     thread::spawn(|| {
//         // New!
//         loop {
//             // black_box(A.load(Relaxed));
//             A.store(0, Relaxed); // New!
//         }
//     });

//     let start = Instant::now();
//     for _ in 0..n {
//         black_box(A.load(Relaxed)); // New!
//     }
//     println!("{:?}", start.elapsed() / n);
// }
use spin::Mutex;
fn spin_mutex() {
    let n = 1_000_000;
    // let me = Arc::new(spin::Mutex::new(0_u32));
    // let me = spin::Mutex::new(0_u32);
    // let me = SpinMutex::<_>::new(0_u32);
    // let me = Arc::new(SpinMutex::<_>::new(0_u32));
    let me = Arc::new(Mutex::new(0_u32));
    // black_box(&me); // New!
    let start = Instant::now();
    let mut res = 0;
    for i in 0..n {
        res = black_box({
            let mut guard = me.lock();
            *guard = i + 1;
            *guard
        }); // New!
    }

    println!("{:?}", start.elapsed() / n);
    println!("res: {res}");
}
