use std::ops::Range;

#[derive(Debug)]
pub(crate) struct CycleRange {
    start: usize,
    end: usize,
    current: usize,
}
impl CycleRange {
    pub fn new(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end - 1,
            current: range.start,
        }
    }
    pub fn next(&mut self) -> usize {
        let current = self.current;
        if self.current == self.end {
            self.current = self.start;
        } else {
            self.current += 1;
        }

        current
    }
}

#[cfg(test)]
mod test {
    use links_network_core::unittest::setup;
    use log::info;

    use super::*;
    #[test]
    fn test_infinite_range() {
        setup::log::configure();
        let mut range = CycleRange::new(0..2);
        info!("range: {:?}", range);
        let i = range.next();
        info!("i: {}", i);
        assert_eq!(i, 0);
        let i = range.next();
        info!("i: {}", i);
        assert_eq!(i, 1);
        let i = range.next();
        info!("i: {}", i);
        assert_eq!(i, 0);
        let i = range.next();
        info!("i: {}", i);
        assert_eq!(i, 1);
    }
}
