// use std::{
//     collections::VecDeque,
//     sync::{Arc, Mutex},
// };

// // use rayon::{ThreadPool, ThreadPoolBuilder};

// pub struct Executor {
//     pool: ThreadPool,
//     tasks: Mutex<VecDeque<Arc<dyn NonBlockingIoFn>>>,
// }
// // type NonBlockingIoFn = Fn()+Send+Sync+'static;
// pub trait NonBlockingIoFn: Send+Sync+'static {
//     fn execute(&self) -> bool;
// }

// impl Executor {
//     pub fn new() -> Self {
//         Self {
//             pool: ThreadPoolBuilder::new().num_threads(1).build().unwrap(),
//             tasks: Mutex::new(VecDeque::new()),
//         }
//     }
//     pub fn new_ref() -> Arc<Self> {
//         Arc::new(Self::new())
//     }
//     pub fn add_task_to_sequence<F>(&self, f: F)
//     where F: NonBlockingIoFn {
//         let mut tasks = self.tasks.lock().unwrap();
//         tasks.push_back(Arc::new(f));
//     }

//     pub fn run(&self) {
//         let tasks = self.tasks.lock().unwrap();
//         for task in tasks.iter() {
//             let result = self.pool.install({
//                 let task = task.clone();
//                 move || task.execute()
//             });
//         }
//     }
// }

// #[cfg(test)]
// mod test {

//     use super::{Executor, NonBlockingIoFn};

//     struct Sleep;
//     impl Sleep {
//         pub fn new() -> Self {
//             Self {}
//         }
//     }
//     impl NonBlockingIoFn for Sleep {
//         fn execute(&self) -> bool {
//             std::thread::sleep(std::time::Duration::from_millis(100));
//             true
//         }
//     }
//     #[test]
//     fn test_executor() {
//         let exe = Executor::new_ref();
//         let task = Sleep::new();
//         exe.add_task_to_sequence(task);
//     }
// }
