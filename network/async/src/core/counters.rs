use std::{
    fmt::{Display, Formatter},
    time::Duration
};

use tokio::time::Instant;

use links_network_core::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct EventIntervalTracker {
    con_id: ConId,
    last_occurrence: Option<Instant>,
    expected_interval: Duration,
    tolerance_factor: f64,
}
impl EventIntervalTracker {
    pub fn new(con_id: ConId, recv_interval: Duration, miss_factor: f64) -> Self {
        Self {
            con_id,
            last_occurrence: None,
            expected_interval: recv_interval,
            tolerance_factor: miss_factor,
        }
    }
    // pub fn new_ref(recv_interval: Duration, miss_factor: f64) -> Arc<Mutex<Self>> {
    //     Arc::new(Mutex::new(Self::new(recv_interval, miss_factor)))
    // }
    pub fn occurred(&mut self) {
        self.last_occurrence = Some(Instant::now());
    }
    pub fn is_within_tolerance_factor(&self) -> bool {
        match self.last_occurrence {
            None => false,
            Some(last_occurrence) => {
                let elapsed = Instant::now() - last_occurrence;
                elapsed.as_secs_f64()
                    < self.expected_interval.as_secs_f64() * self.tolerance_factor
            }
        }
    }
}
impl Display for EventIntervalTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} EventIntervalTracker {{ is_on_time: {}, last_occurrence: {:?}, expected_interval: {:?}, tolerance_factor: {:?},  }}", 
            self.con_id,
            self.is_within_tolerance_factor(),
            match self.last_occurrence{
                None => "None".to_owned(),
                Some(last_occurrence) => format!("{:?}", last_occurrence.elapsed())
            }, 
            self.expected_interval, 
            self.tolerance_factor,
        ) 
    }
}


#[cfg(test)]
mod test {

    use super::*;
    use links_network_core::unittest::setup;
    use log::info;
    #[test]
    fn test_expired() {
        setup::log::configure();
        let mut tracker = EventIntervalTracker::new(ConId::default(), Duration::from_secs_f64(0.100), 2.5);
        info!("tracker: {}", tracker);
        assert_eq!(tracker.is_within_tolerance_factor(), false);
        tracker.occurred();
        info!("tracker: {}", tracker);
        assert_eq!(tracker.is_within_tolerance_factor(), true);
        std::thread::sleep(Duration::from_secs_f64(0.240));
        info!("tracker: {}", tracker);
        assert_eq!(tracker.is_within_tolerance_factor(), true);
        std::thread::sleep(Duration::from_secs_f64(0.010));
        info!("tracker: {}", tracker);
        assert_eq!(tracker.is_within_tolerance_factor(), false);
        
    }
}
