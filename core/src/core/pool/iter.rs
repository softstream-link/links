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
    pub fn current(&self) -> usize {
        if self.current == self.start {
            self.end
        } else {
            self.current - 1
        }
    }
}

#[cfg(test)]
mod test {
    use crate::unittest::setup;
    use log::info;

    use super::*;
    #[test]
    fn test_infinite_range() {
        setup::log::configure();
        let mut range = CycleRange::new(0..2);
        info!("range: {:?}", range);

        let (mut next, mut current) = (range.next(), range.current());
        info!("next: {}, current: {}", next, current);
        assert_eq!(next, 0);
        assert_eq!(next, current);
        (next, current) = (range.next(), range.current());
        info!("next: {}, current: {}", next, current);
        assert_eq!(next, 1);
        assert_eq!(next, current);
        (next, current) = (range.next(), range.current());
        info!("next: {}, current: {}", next, current);
        assert_eq!(next, 0);
        assert_eq!(next, current);
        (next, current) = (range.next(), range.current());
        info!("next: {}, current: {}", next, current);
        assert_eq!(next, 1);
        assert_eq!(next, current);
    }
}
