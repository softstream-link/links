// use std::{sync::{Condvar, Mutex, atomic::{AtomicUsize, AtomicU32, AtomicI32}}, thread, collections::VecDeque, time::Duration};
fn main() {}
// fn main() {
//     println!("Hello, world!");
//     let queue = Mutex::new(VecDeque::new());
//     let not_empty = Condvar::new();

//     thread::scope(|s| {
//         s.spawn(|| loop {
//             let mut q = queue.lock().unwrap();
//             let item = loop {
//                 if let Some(item) = q.pop_front() {
//                     break item;
//                 } else {
//                     q = not_empty.wait(q).unwrap();
//                 }
//             };
//             drop(q);
//             dbg!(item);
//         });

//         for i in 0.. {
//             queue.lock().unwrap().push_back(i);
//             not_empty.notify_one();
//             thread::sleep(Duration::from_secs(1));
//         }
//     });
// }

// fn main() {
//     let num_done = AtomicUsize::new(0);

//     let main_thread = thread::current();

//     thread::scope(|s| {
//         // A background thread to process all 100 items.
//         s.spawn(|| {
//             for i in 0..100 {
//                 process_item(i); // Assuming this takes some time.
//                 num_done.store(i + 1, std::sync::atomic::Ordering::Relaxed);
//                 main_thread.unpark(); // Wake up the main thread.
//             }
//         });

//         // The main thread shows status updates.
//         loop {
//             let n = num_done.load(std::sync::atomic::Ordering::Relaxed);
//             if n == 100 { break; }
//             println!("Working.. {n}/100 done");
//             thread::park_timeout(Duration::from_secs(1));
//         }
//     });

//     println!("Done!");
// }

// fn allocate_new_id() -> u32 {
//     static NEXT_ID: AtomicU32 = AtomicU32::new(0);
//     let mut id = NEXT_ID.load(std::sync::atomic::Ordering::Relaxed);
//     loop {
//         assert!(id < 1000, "too many IDs!");
//         match NEXT_ID.compare_exchange_weak(id, id + 1, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed) {
//             Ok(_) => return id,
//             Err(v) => id = v,
//         }
//     }
// }

// static X: AtomicI32 = AtomicI32::new(0);
// static Y: AtomicI32 = AtomicI32::new(0);
// use std::sync::atomic::Ordering::Relaxed;
// fn main() {
//     let a = thread::spawn(|| {
//         let x = X.load(Relaxed);
//         Y.store(x, Relaxed);
//     });
//     let b = thread::spawn(|| {
//         let y = Y.load(Relaxed);
//         X.store(y, Relaxed);
//         X.swap(val, order)
//     });
//     a.join().unwrap();
//     b.join().unwrap();
//     assert_eq!(X.load(Relaxed), 0); // Might fail?
//     assert_eq!(Y.load(Relaxed), 0); // Might fail?
// }
