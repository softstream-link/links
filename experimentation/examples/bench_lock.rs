use std::{hint::black_box, sync::atomic::AtomicU64, sync::atomic::Ordering::Relaxed, time::Instant, thread};

static A: AtomicU64 = AtomicU64::new(0);

fn main() {
    black_box(&A); // New!
    let n = 1_000_000;


    thread::spawn(|| { // New!
        loop {
            // black_box(A.load(Relaxed));
            A.store(0, Relaxed); // New!
        }
    });


    let start = Instant::now();
    for _ in 0..n {
        black_box(A.load(Relaxed)); // New!
    }
    println!("{:?}", start.elapsed()/n);
}
