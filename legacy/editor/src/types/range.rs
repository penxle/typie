use std::ops::{Add, RangeBounds};

#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl Range {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl RangeBounds<usize> for Range {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Excluded(&self.end)
    }
}

impl Add<usize> for Range {
    type Output = Range;

    fn add(self, other: usize) -> Self::Output {
        Self {
            start: self.start + other,
            end: self.end + other,
        }
    }
}
