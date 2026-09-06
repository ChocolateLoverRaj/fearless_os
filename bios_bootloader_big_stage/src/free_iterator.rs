use core::{iter::Peekable, ops::Range};

struct FreeIteratorData {
    free_start: u64,
    usable_end: u64,
}

/// Calculates free memory based off of usable memory and how much of that usable memory has already been used. Does it in O(N) time. Scales linear to total elements in A and B.
pub struct FreeIterator<A: Iterator<Item = Range<u64>>, B: Iterator<Item = Range<u64>>> {
    usable: A,
    used: Peekable<B>,
    data: Option<FreeIteratorData>,
}

impl<A: Iterator<Item = Range<u64>>, B: Iterator<Item = Range<u64>>> FreeIterator<A, B> {
    /// Both iterators must be sorted by start address and non-overlapping.
    pub fn new(mut usable: A, used: B) -> Self {
        Self {
            data: usable.next().map(|range| FreeIteratorData {
                free_start: range.start,
                usable_end: range.end,
            }),
            usable,
            used: used.peekable(),
        }
    }
}

impl<A: Iterator<Item = Range<u64>>, B: Iterator<Item = Range<u64>>> Iterator
    for FreeIterator<A, B>
{
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        let data = loop {
            let data = self.data.as_mut()?;
            if data.usable_end > data.free_start {
                break data;
            }
            self.data = self.usable.next().map(|range| FreeIteratorData {
                free_start: range.start,
                usable_end: range.end,
            });
        };
        loop {
            if let Some(used) = self.used.peek() {
                if used.end <= data.free_start {
                    self.used.next().unwrap();
                    continue;
                }
                if used.start < data.usable_end {
                    let free = data.free_start..used.start;
                    data.free_start = used.end;
                    self.used.next().unwrap();
                    if !free.is_empty() {
                        break Some(free);
                    }
                } else {
                    let free = data.free_start..data.usable_end;
                    self.data = self.usable.next().map(|range| FreeIteratorData {
                        free_start: range.start,
                        usable_end: range.end,
                    });
                    break Some(free);
                }
            } else {
                let free = data.free_start..data.usable_end;
                self.data = self.usable.next().map(|range| FreeIteratorData {
                    free_start: range.start,
                    usable_end: range.end,
                });
                break Some(free);
            }
        }
    }
}
