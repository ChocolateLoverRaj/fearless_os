use core::{cmp::min, ops::Range};

pub fn subtract_range(a: Range<u64>, b: Range<u64>) -> heapless::Vec<Range<u64>, 2> {
    // Scenario: no overlap
    if a.end <= b.start || b.end <= a.start {
        return [a].into();
    }

    // Scenario: completely subtracts (b covers all of a)
    if a.start >= b.start && a.end <= b.end {
        return [].into();
    }

    // Scenario: cut out middle (b is strictly inside a)
    if b.start > a.start && b.end < a.end {
        return [a.start..b.start, b.end..a.end].into();
    }

    // Scenario: trim left (b overlaps the start of a)
    if b.start <= a.start {
        return [b.end..a.end].into();
    }

    // Scenario: trim right (b overlaps the end of a)
    return [a.start..b.start].into();
}

pub struct SubtractRangesIterator<T> {
    iterator: T,
    range: Range<u64>,
}

impl<T: Iterator<Item = Range<u64>>> SubtractRangesIterator<T> {
    pub fn new(range: Range<u64>, iterator: T) -> Self {
        Self { iterator, range }
    }
}

impl<T: Iterator<Item = Range<u64>>> Iterator for SubtractRangesIterator<T> {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(used_range) = self.iterator.next() {
            if used_range.end <= self.range.start {
                continue;
            }
            let range_to_return = self.range.start..min(self.range.end, used_range.start);
            self.range = used_range.end..self.range.end;
            if !range_to_return.is_empty() {
                return Some(range_to_return);
            }
        }
        if self.range.is_empty() {
            return None;
        }
        let range_to_return = self.range.clone();
        self.range = self.range.end..self.range.end;
        return Some(range_to_return);
    }
}
