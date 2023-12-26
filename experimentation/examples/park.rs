use std::thread::{self, park, spawn};

fn main() {
    run();
}

#[test]
fn test_park() {
    run();
}

fn run() {
    // static COMPLETED: AtomicBool = AtomicBool::new(false);
    let jh = spawn(|| {
        // let now = Instant::now();
        println!("{:?} burning oil", thread::current().id());
        // while now.elapsed().as_secs() < 2 {
        //     // burn some oil
        // }
        println!("{:?} parking", thread::current().id());
        park();
        println!("{:?} got un-parked", thread::current().id());

        // COMPLETED.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    println!("{:?} unpark spawned thread", thread::current().id());
    jh.thread().unpark();
    println!("{:?} now join spawned thread", thread::current().id());
    jh.join().unwrap();
}
