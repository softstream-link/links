use chrono::{DateTime, Utc};
use std::{
    cmp::Reverse,
    fmt::{Debug, Display},
    time::Duration,
};

use crate::asserted_short_name;

#[derive(Debug, PartialEq)]
pub enum TimerTaskStatus {
    Completed,
    Terminate,
    RetryAfter(Duration),
}

pub type Executable = Box<dyn FnMut() -> TimerTaskStatus + Send + 'static>;

pub struct Task {
    name: String,
    execute_at: DateTime<Utc>,
    interval: Duration,
    executable: Executable,
}
impl Task {
    pub fn new(name: &str, interval: Duration, task: Executable) -> Self {
        Task {
            name: name.to_owned(),
            execute_at: chrono::Utc::now(),
            interval,
            executable: task,
        }
    }
    /// Execute the task
    pub fn execute(&mut self) -> TimerTaskStatus {
        (self.executable)()
    }
    /// Reschedule the task to be executed at the next standard interval
    pub fn reschedule(&mut self) {
        self.execute_at += self.interval;
    }
    /// Reschedule the task to be executed at the next custom interval
    pub fn reschedule_with_interval(&mut self, interval: Duration) {
        self.execute_at += interval;
    }
    /// Get [DateTime] of the next scheduled execution
    pub fn execute_at(&self) -> &DateTime<Utc> {
        &self.execute_at
    }
}
impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(asserted_short_name!("Task", Self)).field("name", &self.name).field("execute_at", &self.execute_at).field("interval", &self.interval).finish()
    }
}
impl Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(asserted_short_name!("Task", Self))
            .field("name", &self.name)
            .field("execute_at", &self.execute_at)
            .field("interval", &self.interval)
            .field("executable", &"dyn FnMut() -> TaskStatus")
            .finish()
    }
}
impl Ord for Task {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Reverse(self.execute_at).cmp(&Reverse(other.execute_at))
    }
}
impl PartialOrd for Task {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Reverse(self.execute_at).cmp(&Reverse(other.execute_at)))
    }
}
impl PartialEq for Task {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.execute_at == other.execute_at
    }
}
impl Eq for Task {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use log::info;
    use std::collections::BinaryHeap;

    #[test]
    fn test_task_schedule() {
        setup::log::configure_compact(log::LevelFilter::Info);

        // ensure the BinaryHeap implements a min-heap, meaning a TaskSchedule with the earlier date will be popped first

        let mut schedules = BinaryHeap::new();
        let interval = Duration::from_secs(1);
        schedules.push(Task::new("Task1", interval, Box::new(|| TimerTaskStatus::Completed)));
        schedules.push(Task::new("Task2", interval, Box::new(|| TimerTaskStatus::Completed)));
        schedules.push(Task::new("Task3", interval, Box::new(|| TimerTaskStatus::Completed)));

        let task = schedules.pop().unwrap();
        info!("task: {}", task);
        assert_eq!(task.name, "Task1");

        let task = schedules.pop().unwrap();
        info!("task: {}", task);
        assert_eq!(task.name, "Task2");

        let mut task = schedules.pop().unwrap();
        info!("task: {}", task);
        assert_eq!(task.name, "Task3");

        // reschedule the task
        let last_execution = task.execute_at;
        task.reschedule();
        info!("task: {:?}", task);
        assert_eq!(task.execute_at, last_execution + interval);
    }
}
