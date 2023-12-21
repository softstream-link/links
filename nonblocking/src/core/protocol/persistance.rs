use std::{fmt::Debug, slice::Iter};

use super::state::ProtocolSessionState;
pub trait ProtocolStorage: Debug {
    type Item;
    fn store(&mut self, msg: Self::Item);
    fn iter(&self) -> Iter<'_, Self::Item>;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
}

#[derive(Debug)]
pub struct InMemoryMessageLog<T: Debug> {
    log: Vec<T>,
}
impl<T: Debug> InMemoryMessageLog<T> {
    pub fn new() -> Self {
        Self { log: Vec::new() }
    }
}
impl<T: Debug> Default for InMemoryMessageLog<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Debug> ProtocolStorage for InMemoryMessageLog<T> {
    type Item = T;
    fn store(&mut self, msg: T) {
        self.log.push(msg);
    }
    fn iter(&self) -> Iter<'_, T> {
        self.log.iter()
    }
    fn is_empty(&self) -> bool {
        self.log.is_empty()
    }
    fn len(&self) -> usize {
        self.log.len()
    }
}
impl<T: Debug> From<InMemoryMessageLog<T>> for ProtocolSessionState<InMemoryMessageLog<T>> {
    fn from(value: InMemoryMessageLog<T>) -> Self {
        Self::new(value)
    }
}

// TODO complete this
pub struct FileMessageLog<T> {
    phantom: std::marker::PhantomData<T>,
}

#[cfg(test)]
mod test {
    use links_core::unittest::setup;
    use log::info;

    use crate::prelude::*;

    #[test]
    fn test_in_memory_log() {
        setup::log::configure();
        let mut log = InMemoryMessageLog::<usize>::default();
        log.store(1);
        log.store(2);

        info!("log: {:?}", log);
        assert_eq!(log.len(), 2);

        for i in log.iter() {
            info!("i: {:?}", i);
        }
    }
}
