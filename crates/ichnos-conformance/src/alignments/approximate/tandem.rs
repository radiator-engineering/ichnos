//! Tandem-repeat reduction (pm4py's `approx_tandem_repeats`, after Reißner,
//! Armas-Cervantes and La Rosa, 2022).
//!
//! A run of three or more copies of a block is cut to its first and last
//! copy. The reduced trace is aligned exactly, then the removed copies come
//! back: as a replay of the first copy's moves when those moves bring the
//! model back to the marking they started from, and as log moves otherwise.

use std::cmp::Reverse;
use std::time::Instant;

use super::net::{Net, Step};
use super::search::{Query, Stats, search};

/// A run of `repetitions` copies of `period`, starting at `original_start`
/// in the trace and at `reduced_start` in the reduced trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TandemRepeat<'l> {
    pub(super) original_start: usize,
    pub(super) reduced_start: usize,
    pub(super) period: Vec<&'l str>,
    pub(super) repetitions: usize,
}

/// Whether `block` is not a power of a shorter block.
fn is_primitive(block: &[&str]) -> bool {
    let size = block.len();
    (1..size).all(|p| size % p != 0 || block.chunks(p).any(|c| c != &block[..p]))
}

/// pm4py's `reduce_tandem_repeats`: greedily, from left to right, the run
/// that removes the most events (then the longest run, then the shortest
/// period) is cut to two copies. Returns the reduced trace, the trace index
/// of each kept event and the runs.
pub(super) fn reduce<'l>(labels: &[&'l str]) -> (Vec<&'l str>, Vec<usize>, Vec<TandemRepeat<'l>>) {
    let mut reduced = Vec::new();
    let mut kept = Vec::new();
    let mut repeats = Vec::new();
    let mut index = 0;
    while index < labels.len() {
        // pm4py's key: events removed, run length, `-len`.
        let mut best: Option<((usize, usize, Reverse<usize>), usize, usize)> = None;
        for len in 1..=(labels.len() - index) / 3 {
            let period = &labels[index..index + len];
            if !is_primitive(period) {
                continue;
            }
            let mut reps = 1;
            while index + (reps + 1) * len <= labels.len()
                && &labels[index + reps * len..index + (reps + 1) * len] == period
            {
                reps += 1;
            }
            if reps < 3 {
                continue;
            }
            let key = ((reps - 2) * len, reps * len, Reverse(len));
            if best.as_ref().is_none_or(|b| key > b.0) {
                best = Some((key, len, reps));
            }
        }
        let Some((_, len, reps)) = best else {
            reduced.push(labels[index]);
            kept.push(index);
            index += 1;
            continue;
        };
        repeats.push(TandemRepeat {
            original_start: index,
            reduced_start: reduced.len(),
            period: labels[index..index + len].to_vec(),
            repetitions: reps,
        });
        for i in (index..index + len).chain(index + (reps - 1) * len..index + reps * len) {
            reduced.push(labels[i]);
            kept.push(i);
        }
        index += reps * len;
    }
    (reduced, kept, repeats)
}

/// The result of the method, before it becomes a [`TraceAlignment`].
///
/// [`TraceAlignment`]: super::super::TraceAlignment
pub(super) struct Outcome<'l> {
    pub(super) steps: Vec<Step<'l>>,
    pub(super) stats: Stats,
    pub(super) reduced_trace_length: usize,
    pub(super) tandem_repeats: usize,
    pub(super) removed_events: usize,
    pub(super) model_loop_expansions: usize,
}

pub(super) fn align<'l>(
    net: &Net,
    labels: &[&'l str],
    log_costs: &[u64],
    max_expansions: usize,
    deadline: Option<Instant>,
) -> Option<Outcome<'l>> {
    let (reduced, kept, repeats) = reduce(labels);
    let reduced_costs: Vec<u64> = kept.iter().map(|&i| log_costs[i]).collect();
    let q = Query {
        labels: &reduced,
        log_costs: &reduced_costs,
        offset: 0,
        start: &net.initial,
        to_final: true,
        max_results: 1,
        max_expansions,
        max_post_model_moves: 0,
        deadline,
    };
    let (mut found, stats) = search(net, &q, None);
    if found.is_empty() {
        return None;
    }
    let reduced_steps = found.swap_remove(0).steps;
    let (mut steps, mut loops) = expand(net, &reduced_steps, &repeats, log_costs, true);
    if !net.validate(labels, &steps) {
        (steps, loops) = expand(net, &reduced_steps, &repeats, log_costs, false);
    }
    Some(Outcome {
        steps,
        stats,
        reduced_trace_length: reduced.len(),
        tandem_repeats: repeats.len(),
        removed_events: repeats
            .iter()
            .map(|r| (r.repetitions - 2) * r.period.len())
            .sum(),
        model_loop_expansions: loops,
    })
}

/// pm4py's `_expand_repeats`: puts the removed copies back, from the last
/// run to the first, after the first copy of each run.
fn expand<'l>(
    net: &Net,
    reduced_steps: &[Step<'l>],
    repeats: &[TandemRepeat<'l>],
    log_costs: &[u64],
    prefer_model_loops: bool,
) -> (Vec<Step<'l>>, usize) {
    let mut steps = reduced_steps.to_vec();
    let mut loops = 0;
    for r in repeats.iter().rev() {
        let len = r.period.len();
        let start = steps
            .iter()
            .position(|s| s.log_index == Some(r.reduced_start))
            .expect("the reduced alignment moves on every event");
        let first_end = 1 + steps
            .iter()
            .position(|s| s.log_index == Some(r.reduced_start + len - 1))
            .expect("the reduced alignment moves on every event");
        let first_copy = steps[start..first_end].to_vec();
        let use_loop = prefer_model_loops && is_model_loop(net, &steps, start, first_end);
        let mut inserted = Vec::new();
        for removed in 0..r.repetitions - 2 {
            let copy_start = r.original_start + (removed + 1) * len;
            if use_loop {
                for s in &first_copy {
                    if s.log.is_none() {
                        inserted.push(*s);
                        continue;
                    }
                    let offset = s.log_index.expect("an event of the reduced trace") - r.reduced_start;
                    let cost = match s.transition {
                        Some(t) => net.transitions[t as usize].sync_cost,
                        None => log_costs[copy_start + offset],
                    };
                    inserted.push(Step {
                        log_index: None,
                        cost,
                        ..*s
                    });
                }
                loops += 1;
            } else {
                for (offset, &label) in r.period.iter().enumerate() {
                    inserted.push(Step {
                        log: Some(label),
                        log_index: None,
                        transition: None,
                        cost: log_costs[copy_start + offset],
                    });
                }
            }
        }
        steps.splice(first_end..first_end, inserted);
    }
    (steps, loops)
}

/// pm4py's `_is_model_loop`: firing `steps[start..end]` weakly returns the
/// model to the marking it had before them.
fn is_model_loop(net: &Net, steps: &[Step<'_>], start: usize, end: usize) -> bool {
    let mut m = net.dense(&net.initial);
    for s in &steps[..start] {
        if let Some(t) = s.transition {
            net.weak_fire(t, &mut m);
        }
    }
    let before = m.clone();
    for s in &steps[start..end] {
        if let Some(t) = s.transition {
            net.weak_fire(t, &mut m);
        }
    }
    m == before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_the_run_that_saves_most() {
        let t = ["x", "a", "b", "a", "b", "a", "b", "a", "b", "c", "c", "c"];
        let (reduced, kept, repeats) = reduce(&t);
        assert_eq!(reduced, ["x", "a", "b", "a", "b", "c", "c"]);
        assert_eq!(kept, [0, 1, 2, 7, 8, 9, 11]);
        assert_eq!(repeats.len(), 2);
        assert_eq!(repeats[0].repetitions, 4);
        assert_eq!(repeats[0].period, ["a", "b"]);
        assert_eq!(repeats[1].reduced_start, 5);
    }

    #[test]
    fn primitive_blocks() {
        assert!(is_primitive(&["a"]));
        assert!(is_primitive(&["a", "b", "a"]));
        assert!(!is_primitive(&["a", "a"]));
        assert!(!is_primitive(&["a", "b", "a", "b"]));
    }
}
