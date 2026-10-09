//! Markings of a synchronous product, interned in one arena.
//!
//! A marking is a sorted list of packed `(place, tokens)` entries, one `u64`
//! each: the place in the high 32 bits, the count in the low 32 bits. Sorting
//! the `u64`s sorts by place. Alignment markings hold few tokens, so the list
//! is short and hashes cheaply. Every marking is stored once and named by a
//! dense `u32` id, so the search keeps ids instead of markings.

use std::hash::BuildHasher;

use hashbrown::HashTable;
use rustc_hash::FxBuildHasher;

/// One `(place, tokens)` entry of a marking.
pub(crate) type Packed = u64;

/// Packs a place and its token count.
pub(crate) fn pack(place: u32, tokens: u32) -> Packed {
    (u64::from(place) << 32) | u64::from(tokens)
}

/// The place of a packed entry.
pub(crate) fn place(entry: Packed) -> u32 {
    (entry >> 32) as u32
}

/// The token count of a packed entry.
pub(crate) fn tokens(entry: Packed) -> u32 {
    entry as u32
}

/// Writes into `out` the marking `m` changed by `delta`. Both are sorted by
/// place; places that end with zero tokens are dropped. The move must be
/// enabled, so no count goes below zero.
pub(crate) fn apply_delta(m: &[Packed], delta: &[(u32, i64)], out: &mut Vec<Packed>) {
    out.clear();
    let (mut i, mut j) = (0, 0);
    while i < m.len() || j < delta.len() {
        let mp = m.get(i).map(|&e| place(e));
        let dp = delta.get(j).map(|&(p, _)| p);
        match (mp, dp) {
            (Some(a), Some(b)) if a == b => {
                let n = i64::from(tokens(m[i])) + delta[j].1;
                debug_assert!(n >= 0, "move fired while not enabled");
                if n > 0 {
                    out.push(pack(a, count(n)));
                }
                i += 1;
                j += 1;
            }
            (Some(a), Some(b)) if a < b => {
                out.push(m[i]);
                i += 1;
            }
            (Some(_), None) => {
                out.push(m[i]);
                i += 1;
            }
            (_, Some(b)) => {
                let n = delta[j].1;
                debug_assert!(n >= 0, "move fired while not enabled");
                if n > 0 {
                    out.push(pack(b, count(n)));
                }
                j += 1;
            }
            (None, None) => unreachable!(),
        }
    }
}

fn count(n: i64) -> u32 {
    u32::try_from(n).expect("a place holds at most u32::MAX tokens")
}

/// An arena of distinct markings with dense ids.
#[derive(Debug, Default)]
pub(crate) struct MarkingStore {
    data: Vec<Packed>,
    starts: Vec<usize>,
    table: HashTable<u32>,
}

impl MarkingStore {
    pub(crate) fn new() -> Self {
        Self {
            data: Vec::new(),
            starts: vec![0],
            table: HashTable::new(),
        }
    }

    /// The number of distinct markings stored.
    pub(crate) fn len(&self) -> usize {
        self.starts.len() - 1
    }

    /// The marking with id `id`.
    pub(crate) fn get(&self, id: u32) -> &[Packed] {
        let i = id as usize;
        &self.data[self.starts[i]..self.starts[i + 1]]
    }

    /// The id of `m`, storing it if it is new. Returns the id and whether the
    /// marking was new.
    pub(crate) fn intern(&mut self, m: &[Packed]) -> (u32, bool) {
        let hash = FxBuildHasher.hash_one(m);
        let Self {
            data,
            starts,
            table,
        } = self;
        let slice = |id: u32| {
            let i = id as usize;
            &data[starts[i]..starts[i + 1]]
        };
        if let Some(&id) = table.find(hash, |&id| slice(id) == m) {
            return (id, false);
        }
        let id = u32::try_from(starts.len() - 1).expect("fewer than 2^32 markings");
        data.extend_from_slice(m);
        starts.push(data.len());
        let (data, starts) = (&*data, &*starts);
        table.insert_unique(hash, id, |&other| {
            let i = other as usize;
            FxBuildHasher.hash_one(&data[starts[i]..starts[i + 1]])
        });
        (id, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_gives_dense_ids() {
        let mut store = MarkingStore::new();
        let a = [pack(0, 1), pack(3, 2)];
        let b = [pack(1, 1)];
        assert_eq!(store.intern(&a), (0, true));
        assert_eq!(store.intern(&b), (1, true));
        assert_eq!(store.intern(&a), (0, false));
        assert_eq!(store.intern(&[]), (2, true));
        assert_eq!(store.get(0), &a);
        assert_eq!(store.get(2), &[] as &[Packed]);
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn delta_merges_and_drops_zeros() {
        let m = [pack(0, 1), pack(2, 1), pack(5, 3)];
        let mut out = Vec::new();
        apply_delta(&m, &[(0, -1), (1, 2), (5, -1), (7, 1)], &mut out);
        assert_eq!(out, [pack(1, 2), pack(2, 1), pack(5, 2), pack(7, 1)]);
    }
}
