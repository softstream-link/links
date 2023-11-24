use std::{
    io::{Error, ErrorKind},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
        Arc,
    },
};

use crate::asserted_short_name;

#[derive(Debug, Clone)]
pub struct AcceptorConnectionGate {
    max_count: NonZeroUsize,
    cur_count: Arc<AtomicUsize>,
}
impl AcceptorConnectionGate {
    pub fn new(max_count: NonZeroUsize) -> Self {
        Self { max_count, cur_count: Arc::new(AtomicUsize::new(0)) }
    }
    pub fn get_max_count(&self) -> usize {
        self.max_count.get()
    }
    pub fn get_cur_count(&self) -> usize {
        self.cur_count.load(Relaxed)
    }
    pub fn increment(&self) -> Result<(), Error> {
        // load current count, note this can theoretically change,
        // if the existing connection gets dropped at the same time by calling decrement
        let mut cur_count = self.cur_count.load(Relaxed);
        loop {
            if cur_count >= self.max_count.get() {
                return Err(Error::new(ErrorKind::OutOfMemory, format!("{} cur_count: {} reached max: {}", asserted_short_name!("AcceptorConnectionGate", Self), cur_count, self.max_count.get())));
            }
            match self.cur_count.compare_exchange_weak(cur_count, cur_count + 1, Relaxed, Relaxed) {
                Ok(_) => return Ok(()),
                Err(new_cur_count) => cur_count = new_cur_count,
            }
        }
    }
    pub fn decrement(&self) {
        self.cur_count.fetch_sub(1, Relaxed);
    }
    pub fn get_new_connection_barrier(&self) -> RemoveConnectionBarrierOnDrop {
        RemoveConnectionBarrierOnDrop {
            completed: Arc::new(AtomicBool::new(false)),
            cur_count: self.cur_count.clone(),
        }
    }
}

/// Event a single clone copy drop will decrement the connection count for the acceptor. Please be careful and only clone
/// where required and don't accidentally drop the connection unless you are the connection it counts as active is itself dropped/shutdown.
#[derive(Debug, Clone)]
pub struct RemoveConnectionBarrierOnDrop {
    completed: Arc<AtomicBool>,
    cur_count: Arc<AtomicUsize>,
}
impl Drop for RemoveConnectionBarrierOnDrop {
    fn drop(&mut self) {
        if self.completed.compare_exchange(false, true, Relaxed, Relaxed).is_ok() {
            self.cur_count.fetch_sub(1, Relaxed);
        }
    }
}

#[cfg(test)]
mod test {
    use log::info;

    use crate::unittest::setup;

    use super::*;
    use std::num::NonZeroUsize;

    #[test]
    fn test_max_connection_tracker() {
        setup::log::configure_compact(log::LevelFilter::Info);
        let tracker = AcceptorConnectionGate::new(NonZeroUsize::new(2).unwrap());
        assert!(matches!(tracker.increment(), Ok(())));
        assert!(matches!(tracker.increment(), Ok(())));
        let res = tracker.increment();
        info!("res: {:?}", res);
        assert!(res.is_err());
        info!("tracker: {:?}", tracker);
        assert_eq!(tracker.get_cur_count(), 2);

        tracker.decrement();
        info!("tracker: {:?}", tracker);
        assert_eq!(tracker.get_cur_count(), 1);
    }

    #[test]
    fn test_once_decrementor() {
        setup::log::configure_compact(log::LevelFilter::Info);
        let tracker = AcceptorConnectionGate::new(NonZeroUsize::new(2).unwrap());

        assert!(matches!(tracker.increment(), Ok(())));
        assert!(matches!(tracker.increment(), Ok(())));

        assert!(matches!(tracker.increment(), Err(_)));
        info!("tracker: {:?}", tracker);
        assert_eq!(tracker.get_cur_count(), 2);

        let once_dec = tracker.get_new_connection_barrier();
        let second_once_dec = once_dec.clone();

        drop(once_dec);
        drop(second_once_dec);
        info!("tracker: {:?}", tracker);
        assert_eq!(tracker.get_cur_count(), 1); // despite decrementing twice
    }
}
