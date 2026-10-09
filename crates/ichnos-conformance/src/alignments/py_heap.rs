//! Python's `heapq` algorithm, step for step.
//!
//! pm4py's process-tree search changes the keys of entries already in its
//! heap and never restores the heap order, so which entry it pops next
//! depends on the exact sift steps of `heapq`. Ports that must return the
//! same alignment use this heap instead of `std::collections::BinaryHeap`.

/// A min-heap of `T` ordered by a caller-supplied "less than", with the
/// sift steps of CPython's `heapq` (`heappush`, `heappop`).
#[derive(Debug, Clone, Default)]
pub(crate) struct PyHeap<T> {
    items: Vec<T>,
}

impl<T: Copy> PyHeap<T> {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// `heapq.heappush`.
    pub(crate) fn push(&mut self, item: T, lt: impl Fn(T, T) -> bool) {
        self.items.push(item);
        let last = self.items.len() - 1;
        self.sift_down(0, last, &lt);
    }

    /// `heapq.heappop`.
    pub(crate) fn pop(&mut self, lt: impl Fn(T, T) -> bool) -> Option<T> {
        let last = self.items.pop()?;
        if self.items.is_empty() {
            return Some(last);
        }
        let top = std::mem::replace(&mut self.items[0], last);
        self.sift_up(0, &lt);
        Some(top)
    }

    /// `heapq._siftdown`: moves the entry at `pos` towards the root.
    fn sift_down(&mut self, start: usize, mut pos: usize, lt: &impl Fn(T, T) -> bool) {
        let item = self.items[pos];
        while pos > start {
            let parent = (pos - 1) >> 1;
            if lt(item, self.items[parent]) {
                self.items[pos] = self.items[parent];
                pos = parent;
            } else {
                break;
            }
        }
        self.items[pos] = item;
    }

    /// `heapq._siftup`: moves the smaller child up until `pos` is a leaf,
    /// then sifts the entry down from there.
    fn sift_up(&mut self, mut pos: usize, lt: &impl Fn(T, T) -> bool) {
        let end = self.items.len();
        let start = pos;
        let item = self.items[pos];
        let mut child = 2 * pos + 1;
        while child < end {
            let right = child + 1;
            if right < end && !lt(self.items[child], self.items[right]) {
                child = right;
            }
            self.items[pos] = self.items[child];
            pos = child;
            child = 2 * pos + 1;
        }
        self.items[pos] = item;
        self.sift_down(start, pos, lt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_order() {
        let mut h = PyHeap::new();
        for v in [5, 3, 8, 1, 9, 2, 7] {
            h.push(v, |a, b| a < b);
        }
        let mut out = Vec::new();
        while let Some(v) = h.pop(|a, b| a < b) {
            out.push(v);
        }
        assert_eq!(out, [1, 2, 3, 5, 7, 8, 9]);
    }

    /// Ties pop in the order CPython's heapq gives: pairs compared on the
    /// first field only.
    #[test]
    fn ties_follow_heapq() {
        let lt = |a: (u8, u8), b: (u8, u8)| a.0 < b.0;
        let mut h = PyHeap::new();
        for v in [(1, 0), (1, 1), (0, 2), (1, 3), (1, 4)] {
            h.push(v, lt);
        }
        let mut out = Vec::new();
        while let Some(v) = h.pop(lt) {
            out.push(v.1);
        }
        // CPython's heapq gives the order [(0,2),(1,1),(1,0),(1,3),(1,4)]
        // after the pushes and pops 2, 0, 4, 1, 3.
        assert_eq!(out, [2, 0, 4, 1, 3]);
    }
}
