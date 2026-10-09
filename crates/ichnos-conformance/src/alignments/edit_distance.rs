//! Alignments of each trace against the closest trace of another log
//! (pm4py's `algo/conformance/alignments/edit_distance`, variant
//! `edit_distance`).
//!
//! For each trace, pm4py picks the model-log variant with the smallest
//! Levenshtein distance, then aligns the two sequences with the matching
//! blocks of Python's `difflib.SequenceMatcher`. Every event outside a
//! matching block is a log move, every model activity outside one a model
//! move, each costing 10000. `difflib` does not find a longest common
//! subsequence, so the cost can exceed `10000 ×` the insert/delete distance;
//! ichnos ports `difflib`'s algorithm to give the same cost.

use std::collections::HashMap;

use ichnos_core::{EventKeys, EventLog};

use super::costs::STD_LOG_MOVE_COST;
use super::result::{LogAlignment, SequenceAlignment, SequenceMove};
use crate::error::{Error, Result};

/// The variants of a model log, prepared for edit-distance alignments.
#[derive(Debug, Clone)]
pub struct EditDistanceAligner {
    ids: HashMap<String, u32>,
    models: Vec<Vec<u32>>,
    names: Vec<Vec<String>>,
    /// Model indices sorted by length, ties in model order.
    by_length: Vec<usize>,
    shortest: u64,
}

impl EditDistanceAligner {
    /// Prepares the model traces, given as activity names. Duplicates are
    /// kept once. Fails with [`Error::EmptyModelLog`] when there are none.
    pub fn new<S: AsRef<str>>(model_traces: &[Vec<S>]) -> Result<Self> {
        let mut ids: HashMap<String, u32> = HashMap::new();
        let mut models: Vec<Vec<u32>> = Vec::new();
        let mut names: Vec<Vec<String>> = Vec::new();
        for t in model_traces {
            let encoded: Vec<u32> = t
                .iter()
                .map(|a| {
                    let next = u32::try_from(ids.len()).expect("activity count fits u32");
                    *ids.entry(a.as_ref().to_owned()).or_insert(next)
                })
                .collect();
            if !models.contains(&encoded) {
                models.push(encoded);
                names.push(t.iter().map(|a| a.as_ref().to_owned()).collect());
            }
        }
        if models.is_empty() {
            return Err(Error::EmptyModelLog);
        }
        let mut by_length: Vec<usize> = (0..models.len()).collect();
        by_length.sort_by_key(|&m| models[m].len());
        let shortest = models.iter().map(Vec::len).min().unwrap_or(0) as u64;
        Ok(Self {
            ids,
            models,
            names,
            by_length,
            shortest,
        })
    }

    /// Aligns one trace against its closest model trace.
    ///
    /// When several model traces are closest, pm4py takes the first in an
    /// order that depends on Python's string hashing. ichnos takes, among
    /// the traces at the smallest distance, the one whose length is closest
    /// to the trace, then the shorter one, then the first given. Different
    /// choices can give different costs.
    pub fn align<S: AsRef<str>>(&self, trace: &[S]) -> SequenceAlignment {
        // Activities outside the model get an id no model trace uses.
        let unknown = u32::try_from(self.ids.len()).expect("activity count fits u32");
        let encoded: Vec<u32> = trace
            .iter()
            .map(|a| self.ids.get(a.as_ref()).copied().unwrap_or(unknown))
            .collect();
        let m = closest(&encoded, &self.models, &self.by_length);
        let blocks = matching_blocks(&encoded, &self.models[m]);
        let (moves, cost) = moves_from_blocks(&blocks, encoded.len(), &self.names[m]);
        let len = encoded.len() as u64;
        let unfit = cost / STD_LOG_MOVE_COST;
        let fitness = if unfit == 0 {
            1.0
        } else if len + self.shortest > 0 {
            1.0 - unfit as f64 / (len + self.shortest) as f64
        } else {
            0.0
        };
        SequenceAlignment {
            moves,
            cost,
            fitness,
            best_worst_cost: (len + self.shortest) * STD_LOG_MOVE_COST,
            visited_states: 0,
            closed_states: 0,
        }
    }

    /// Aligns every trace of `log`, once per variant.
    pub fn align_log(
        &self,
        log: &EventLog,
        keys: &EventKeys,
    ) -> Result<LogAlignment<SequenceAlignment>> {
        let variants = log.variants(keys)?;
        let alignments = variants
            .iter()
            .map(|v| Some(self.align(&variants.names(v).collect::<Vec<_>>())))
            .collect();
        Ok(LogAlignment::new(variants, alignments))
    }
}

/// Aligns every trace of `log` against the closest variant of `model_log`
/// (pm4py's `conformance_diagnostics_alignments(log, model_log)`). See
/// [`EditDistanceAligner::align`] for ties.
///
/// Fails with [`Error::EmptyModelLog`] when `model_log` has no traces.
pub fn align_log_edit_distance(
    log: &EventLog,
    model_log: &EventLog,
    keys: &EventKeys,
) -> Result<LogAlignment<SequenceAlignment>> {
    let model = model_log.variants(keys)?;
    let traces: Vec<Vec<&str>> = model.iter().map(|v| model.names(v).collect()).collect();
    EditDistanceAligner::new(&traces)?.align_log(log, keys)
}

/// pm4py's `argmin_levenshtein`: candidates in order of length difference to
/// the trace, stopping once the length difference alone reaches the best
/// distance found.
fn closest(trace: &[u32], models: &[Vec<u32>], by_length: &[usize]) -> usize {
    if let Some(m) = models.iter().position(|m| m == trace) {
        return m;
    }
    let mut order = by_length.to_vec();
    order.sort_by_key(|&m| models[m].len().abs_diff(trace.len()));
    let mut best = (order[0], usize::MAX);
    for m in order {
        let diff = models[m].len().abs_diff(trace.len());
        if diff >= best.1 {
            break;
        }
        let d = levenshtein(trace, &models[m]);
        if d < best.1 {
            best = (m, d);
        }
    }
    best.0
}

fn levenshtein(a: &[u32], b: &[u32]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            cur[j + 1] = (prev[j + 1] + 1)
                .min(cur[j] + 1)
                .min(prev[j] + usize::from(x != y));
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// A matching block: `a[i..i + size] == b[j..j + size]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Block {
    i: usize,
    j: usize,
    size: usize,
}

/// `difflib.SequenceMatcher(None, a, b).get_matching_blocks()`, without
/// the final sentinel block. `autojunk` is on, as in pm4py: when `b` has 200
/// or more items, items that fill more than 1% of `b` are not used to seed
/// matches.
fn matching_blocks(a: &[u32], b: &[u32]) -> Vec<Block> {
    let mut b2j: HashMap<u32, Vec<usize>> = HashMap::new();
    for (j, &x) in b.iter().enumerate() {
        b2j.entry(x).or_default().push(j);
    }
    if b.len() >= 200 {
        let ntest = b.len() / 100 + 1;
        b2j.retain(|_, js| js.len() <= ntest);
    }
    let mut queue = vec![(0, a.len(), 0, b.len())];
    let mut blocks = Vec::new();
    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        let m = longest_match(a, b, &b2j, alo, ahi, blo, bhi);
        if m.size > 0 {
            blocks.push(m);
            if alo < m.i && blo < m.j {
                queue.push((alo, m.i, blo, m.j));
            }
            if m.i + m.size < ahi && m.j + m.size < bhi {
                queue.push((m.i + m.size, ahi, m.j + m.size, bhi));
            }
        }
    }
    blocks.sort();
    let mut merged: Vec<Block> = Vec::new();
    for blk in blocks {
        match merged.last_mut() {
            Some(last) if last.i + last.size == blk.i && last.j + last.size == blk.j => {
                last.size += blk.size;
            }
            _ => merged.push(blk),
        }
    }
    merged
}

/// `SequenceMatcher.find_longest_match` with no junk.
fn longest_match(
    a: &[u32],
    b: &[u32],
    b2j: &HashMap<u32, Vec<usize>>,
    alo: usize,
    ahi: usize,
    blo: usize,
    bhi: usize,
) -> Block {
    let (mut besti, mut bestj, mut best) = (alo, blo, 0);
    // Length of the match ending at a[i - 1], b[j], keyed by j.
    let mut j2len: HashMap<usize, usize> = HashMap::new();
    for (i, x) in a.iter().enumerate().take(ahi).skip(alo) {
        let mut next = HashMap::new();
        for &j in b2j.get(x).map(Vec::as_slice).unwrap_or(&[]) {
            if j < blo {
                continue;
            }
            if j >= bhi {
                break;
            }
            let k = j
                .checked_sub(1)
                .and_then(|p| j2len.get(&p))
                .copied()
                .unwrap_or(0)
                + 1;
            next.insert(j, k);
            if k > best {
                (besti, bestj, best) = (i + 1 - k, j + 1 - k, k);
            }
        }
        j2len = next;
    }
    // Extend over items that did not seed matches (autojunk's popular ones).
    while besti > alo && bestj > blo && a[besti - 1] == b[bestj - 1] {
        besti -= 1;
        bestj -= 1;
        best += 1;
    }
    while besti + best < ahi && bestj + best < bhi && a[besti + best] == b[bestj + best] {
        best += 1;
    }
    Block {
        i: besti,
        j: bestj,
        size: best,
    }
}

/// pm4py's moves from matching blocks: log moves, then model moves, then
/// the block's synchronous moves.
fn moves_from_blocks(blocks: &[Block], a_len: usize, b: &[String]) -> (Vec<SequenceMove>, u64) {
    let (mut i, mut j) = (0, 0);
    let mut moves = Vec::new();
    let mut cost = 0;
    let end = Block {
        i: a_len,
        j: b.len(),
        size: 0,
    };
    for blk in blocks.iter().chain(std::iter::once(&end)) {
        while i < blk.i {
            moves.push(SequenceMove::Log { event: i });
            cost += STD_LOG_MOVE_COST;
            i += 1;
        }
        while j < blk.j {
            moves.push(SequenceMove::Model {
                activity: Some(b[j].as_str().into()),
            });
            cost += STD_LOG_MOVE_COST;
            j += 1;
        }
        for _ in 0..blk.size {
            moves.push(SequenceMove::Sync { event: i });
            i += 1;
            j += 1;
        }
    }
    (moves, cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(s: &str) -> Vec<u32> {
        s.bytes().map(u32::from).collect()
    }

    /// Values checked against CPython's `difflib`.
    #[test]
    fn matching_blocks_follow_difflib() {
        let blocks = |a: &str, b: &str| -> Vec<(usize, usize, usize)> {
            matching_blocks(&seq(a), &seq(b))
                .into_iter()
                .map(|b| (b.i, b.j, b.size))
                .collect()
        };
        assert_eq!(blocks("abxcd", "abcd"), [(0, 0, 2), (3, 2, 2)]);
        assert_eq!(blocks("abcd", "dcba"), [(0, 3, 1)]);
        assert_eq!(blocks("qabxcd", "abycdf"), [(1, 0, 2), (4, 3, 2)]);
        assert_eq!(blocks("", "abc"), []);
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein(&seq("kitten"), &seq("sitting")), 3);
        assert_eq!(levenshtein(&seq(""), &seq("abc")), 3);
    }
}
