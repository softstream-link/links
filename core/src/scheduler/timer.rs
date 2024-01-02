use super::task::{Task, TimerTaskStatus};
use crate::asserted_short_name;
use log::{debug, info, log_enabled, warn};
use std::{
    collections::BinaryHeap,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread::{park, park_timeout, Builder, JoinHandle},
    time::Duration,
};

#[derive(Debug)]
enum Operation {
    Execute(Task),
    Stop,
}

/// Class that allows scheduling tasks to be executed at repeating interval.
pub struct Timer {
    tx_task: Sender<Operation>,
    jh_executor: JoinHandle<()>,
}
impl Timer {
    /// Create new Timer instance with given name. Name is used for logging purposes.
    pub fn new(name: &str) -> Self {
        let (tx_task, rx_task) = channel();
        let jh_executor = Executor::new(name, rx_task).spawn();
        Timer { tx_task, jh_executor }
    }
    pub fn schedule<T: FnMut() -> TimerTaskStatus + Send + 'static>(&self, name: &str, repeat: Duration, task: T) {
        let task = Box::new(task);
        let task_schedule = Task::new(name, repeat, task);

        self.tx_task.send(Operation::Execute(task_schedule)).unwrap();
        self.jh_executor.thread().unpark();
    }

    /// Drops all scheduled tasks and stops the executor thread until new task is scheduled.
    pub fn stop(self) {
        self.tx_task.send(Operation::Stop).unwrap();
        self.jh_executor.thread().unpark();
        self.jh_executor.join().unwrap();
    }
}

/// This class runs in a separate thread and is responsible for executing tasks passed to it via rx_task channel.
struct Executor {
    name: String,
    rx_task: Receiver<Operation>,
    tasks: BinaryHeap<Task>,
}
impl Executor {
    pub fn new(name: &str, rx_task: Receiver<Operation>) -> Self {
        Executor {
            name: name.to_owned(),
            rx_task,
            tasks: BinaryHeap::new(),
        }
    }
    pub fn spawn(self) -> JoinHandle<()> {
        Builder::new()
            .name(self.name.to_string())
            .spawn({
                let mut e = self;
                move || Executor::run(&mut e)
            })
            .unwrap()
    }
    fn run(&mut self) {
        loop {
            // first check for new schedules as they never run and by design executed to be executed first time immediately

            use Operation::{Execute, Stop};
            match self.rx_task.try_recv() {
                Ok(Execute(task)) => {
                    if log_enabled!(log::Level::Info) {
                        info!("Adding Operation::Execute({})", task);
                    }
                    self.tasks.push(task);
                }
                Ok(Stop) => {
                    if log_enabled!(log::Level::Info) {
                        info!("{:?} {} and dropping all schedules tasks", Stop, asserted_short_name!("Executor", Self));
                    }
                    for task in self.tasks.drain() {
                        if log_enabled!(log::Level::Info) {
                            info!("Dropping task: {}", task);
                            drop(task);
                        }
                    }
                    break;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    warn!("{} rx channel is disconnected, looks like you dropped tx. Terminating executing thread", asserted_short_name!("Executor", Self));
                    for task in self.tasks.drain() {
                        warn!("Dropping task: {}", task);
                    }
                    break;
                }
            }

            // now execute all tasks already scheduled for execution
            use TimerTaskStatus::{Completed, RetryAfter, Terminate};
            match self.tasks.pop() {
                Some(mut task) => {
                    let now = chrono::Utc::now();
                    if task.execute_at() > &now {
                        // execute date is in the future, park until the next execution date
                        let timeout = (*task.execute_at() - now).to_std().unwrap();
                        if log_enabled!(log::Level::Debug) {
                            debug!("Not due task: {}. Parking thread for: {:?}", task, timeout);
                        }
                        // return task to the queue
                        self.tasks.push(task);
                        park_timeout(timeout);
                    } else {
                        // execute date is due
                        if log_enabled!(log::Level::Debug) {
                            debug!("Executing task: {} ", task);
                        }
                        match task.execute() {
                            Completed => {
                                if log_enabled!(log::Level::Debug) {
                                    debug!("{:?} task: {}", Completed, task);
                                }
                                task.reschedule();
                                self.tasks.push(task);
                            }
                            Terminate => {
                                if log_enabled!(log::Level::Info) {
                                    info!("{:?} task: {}, will no longer schedule. Task is getting dropped", Terminate, task);
                                }
                                drop(task);
                            }
                            RetryAfter(retry_after) => {
                                if log_enabled!(log::Level::Debug) {
                                    debug!("{:?} task: {}", RetryAfter(retry_after), task);
                                }
                                task.reschedule_with_interval(retry_after);
                                self.tasks.push(task);
                            }
                        }
                    }
                }
                None => {
                    debug!("Executor has no tasks to run and will park thread until new task added.");
                    park();
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use more_asserts::assert_lt;

    use super::*;
    use crate::unittest::setup;
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Instant,
    };

    #[test]
    fn test_timer() {
        setup::log::configure_level(log::LevelFilter::Debug);
        let timer = Timer::new("unittest");

        static TASK1_REMAINING_ITERATIONS: AtomicU32 = AtomicU32::new(5);
        static TASK2_REMAINING_ITERATIONS: AtomicU32 = AtomicU32::new(3);
        static REPEAT_INTERVAL: Duration = Duration::from_millis(100);

        timer.schedule("task1", REPEAT_INTERVAL, || {
            let iteration_remaining = TASK1_REMAINING_ITERATIONS.fetch_sub(1, Ordering::Relaxed) - 1;
            info!("task1, iteration {}", iteration_remaining + 1);
            if iteration_remaining == 0 {
                TimerTaskStatus::Terminate
            } else {
                TimerTaskStatus::Completed
            }
        });

        timer.schedule("task2", REPEAT_INTERVAL, || {
            let iteration_remaining = TASK2_REMAINING_ITERATIONS.fetch_sub(1, Ordering::Relaxed) - 1;
            info!("task2, iterations_remaining {}", iteration_remaining);
            if iteration_remaining == 0 {
                TimerTaskStatus::Terminate
            } else {
                TimerTaskStatus::Completed
            }
        });

        let now = Instant::now();
        while TASK1_REMAINING_ITERATIONS.load(Ordering::Relaxed) > 0 {}
        let elapsed = now.elapsed();
        timer.stop();
        let mut expected_completion = REPEAT_INTERVAL * 5;
        expected_completion = expected_completion + expected_completion / 10; // 10% tolerance
        info!("elapsed: {:?}", elapsed);
        info!("expected_completion: {:?}", expected_completion);
        assert_lt!(elapsed, expected_completion);

        assert_eq!(TASK1_REMAINING_ITERATIONS.load(Ordering::Relaxed), 0);
        assert_eq!(TASK2_REMAINING_ITERATIONS.load(Ordering::Relaxed), 0);
    }
}
