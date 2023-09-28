pub(self) mod iter;
use std::{
    fmt::{Debug, Display},
    io::{Error, ErrorKind},
    num::NonZeroUsize,
};

use slab::{IntoIter, Slab};

use self::iter::CycleRange;

/// A round robin pool of elements
#[derive(Debug)]
pub struct RoundRobinPool<T: Debug+Display> {
    elements: Slab<T>,
    slab_keys: CycleRange,
    last_used: usize,
}
impl<T: Debug+Display> RoundRobinPool<T> {
    pub fn with_capacity(max_capacity: NonZeroUsize) -> Self {
        Self {
            elements: Slab::with_capacity(max_capacity.get()),
            slab_keys: CycleRange::new(0..max_capacity.get()),
            last_used: 0,
        }
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.elements.capacity()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
    #[inline(always)]
    pub fn has_capacity(&self) -> bool {
        self.elements.len() < self.elements.capacity()
    }
    #[inline(always)]
    pub fn next(&mut self) -> Option<&mut T> {
        for _ in 0..self.elements.capacity() {
            let key = self.slab_keys.next();
            if self.elements.contains(key) {
                self.last_used = key;
                return Some(&mut self.elements[key]);
            }
        }
        None
    }
    #[inline(always)]
    pub fn add(&mut self, element: T) -> Result<(), Error> {
        if !self.has_capacity() {
            return Err(Error::new(
                ErrorKind::OutOfMemory,
                format!("RoundRobinPool at max capacity: {}, element: {} will be dropped", self.len(), element),
            ));
        }

        let _key = self.elements.insert(element);
        Ok(())
    }
    #[inline(always)]
    pub fn remove_last_used(&mut self) -> T {
        self.elements.remove(self.last_used)
    }
    #[inline(always)]
    pub fn clear(&mut self) {
        self.elements.clear();
    }
    #[inline(always)]
    pub fn into_iter(self) -> IntoIter<T> {
        self.elements.into_iter()
    }
}

impl<T: Debug+Display> Display for RoundRobinPool<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RoundRobinPool<len: {} of cap: {} [{}]>",
            self.elements.len(),
            self.elements.capacity(),
            self.elements
                .iter()
                .map(|(_, element)| format!("{}", element))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}
impl<T: Debug+Display> Default for RoundRobinPool<T> {
    /// Creates a new [RoundRobinPool] with a max_connections of 1
    fn default() -> Self {
        Self::with_capacity(NonZeroUsize::new(1).unwrap())
    }
}

#[cfg(test)]
mod test {
    use log::info;

    use super::*;
    use crate::unittest::setup;
    #[test]
    fn test_pool() {
        setup::log::configure();

        // empty
        let mut pool = RoundRobinPool::<String>::with_capacity(NonZeroUsize::new(3).unwrap());
        assert_eq!(pool.len(), 0);
        assert_eq!(pool.is_empty(), true);
        assert_eq!(pool.has_capacity(), true);

        // add
        pool.add("One".to_owned()).unwrap();
        info!("pool: {}", pool);
        assert_eq!(pool.len(), 1);

        // add
        pool.add("Two".to_owned()).unwrap();
        info!("pool: {}", pool);
        assert_eq!(pool.len(), 2);

        // round robin
        let one = pool.next().unwrap();
        assert_eq!(one, &"One".to_owned());
        let two = pool.next().unwrap();
        assert_eq!(two, &"Two".to_owned());

        let one = pool.next().unwrap();
        assert_eq!(one, &"One".to_owned());
        let two = pool.next().unwrap();
        assert_eq!(two, &"Two".to_owned());

        // remove last
        pool.remove_last_used();
        info!("pool: {}", pool);
        assert_eq!(pool.len(), 1);

        // always ONE
        let one = pool.next().unwrap();
        assert_eq!(one, &"One".to_owned());
        let one = pool.next().unwrap();
        assert_eq!(one, &"One".to_owned());

        // max capacity
        pool.add("Two".to_owned()).unwrap();
        pool.add("Three".to_owned()).unwrap();
        info!("pool: {}", pool);
        assert_eq!(pool.len(), 3);
        let err = pool.add("Four".to_owned()).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::OutOfMemory);
        info!("err: {}", err);
    }
}
