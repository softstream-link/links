use std::time::Duration;

pub mod callback;
pub mod sender;
pub mod prelude;

#[inline]
pub fn timeout_selector(priority_1: Option<f64>, priority_2: Option<f64>) -> Duration {
    match priority_1 {
        Some(timeout) => Duration::from_secs_f64(timeout),
        None => match priority_2 {
            Some(timeout) => Duration::from_secs_f64(timeout),
            None => Duration::from_secs(0),
        },
    }
}