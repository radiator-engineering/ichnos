//! Fixed-horizon sequential alignment (pm4py's `approx_fixed_horizon`,
//! after van Dongen, Carmona, Chatain and Taymouri, 2017).
//!
//! Each round enumerates the moves of the synchronous product up to
//! `horizon` steps ahead and commits the prefix whose cost plus the
//! marking-equation estimate of the rest is lowest. The horizon grows when
//! no prefix qualifies or when the estimate doubles. When the rounds do
//! not reach the final marking, the method falls back to the exact search.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Instant;

use rustc_hash::FxHashMap;

use super::super::marking::{Packed, apply_delta, place, tokens};
use super::super::result::Move;
use super::super::state_equation::StateEquation;
use super::super::sync_product::{ModelPart, MoveSet, SyncProduct};
use super::FixedHorizonFallback;
use super::net::{Net, Step};
use super::search::{Query, Stats, search};
use crate::error::Result;

pub(super) struct Settings {
    pub(super) horizon: usize,
    pub(super) min_progress: usize,
    pub(super) max_horizon: usize,
    pub(super) max_prefix_states: usize,
    pub(super) max_iterations: usize,
    pub(super) max_expansions: usize,
}

/// The result of the method.
pub(super) struct Outcome<'l> {
    pub(super) steps: Vec<Step<'l>>,
    pub(super) cost: u64,
    pub(super) stats: Stats,
    pub(super) lp_solved: usize,
    pub(super) committed_horizons: Vec<usize>,
    pub(super) fallback: Option<FixedHorizonFallback>,
}

/// The prefix chosen by one round (pm4py's `_PrefixSolution`).
struct Prefix {
    marking: Vec<Packed>,
    path: Vec<u32>,
    prefix_cost: u64,
    tail_cost: u64,
    progress: usize,
    stats: Stats,
    lp_solved: usize,
}

impl Prefix {
    fn objective(&self) -> u64 {
        self.prefix_cost + self.tail_cost
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn align<'l>(
    net: &Net,
    model: &ModelPart,
    names: &[&str],
    labels: &[&'l str],
    log_costs: &[u64],
    s: &Settings,
    deadline: Option<Instant>,
) -> Result<Option<Outcome<'l>>> {
    let sp = SyncProduct::new(model, labels, log_costs, MoveSet::All);
    let order = move_order(&sp, labels, names);
    let mut equation = StateEquation::new(&sp);

    let mut current = sp.initial.clone();
    let mut path: Vec<u32> = Vec::new();
    let mut remaining = labels.len();
    let mut previous_estimate: Option<u64> = None;
    let mut horizon = s.horizon;
    let mut progress = s.min_progress.min(remaining);
    let mut committed = Vec::new();
    let mut stats = Stats::default();
    let mut lp_solved = 0;
    let mut fallback = None;

    let mut finished = false;
    for _ in 0..s.max_iterations {
        if current == sp.final_marking {
            finished = true;
            break;
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            fallback = Some(FixedHorizonFallback::Timeout);
            finished = true;
            break;
        }
        let Some(prefix) = solve_prefix(
            &sp,
            &order,
            &mut equation,
            &current,
            horizon,
            progress,
            s.max_prefix_states,
            deadline,
        )?
        else {
            if horizon < s.max_horizon {
                horizon += 1;
                progress = (progress + 1).min(remaining);
                continue;
            }
            fallback = Some(FixedHorizonFallback::NoPrefixSolution);
            finished = true;
            break;
        };
        stats += prefix.stats;
        lp_solved += prefix.lp_solved;
        if prefix.marking != sp.final_marking
            && previous_estimate.is_some_and(|e| prefix.objective() >= 2 * e)
            && horizon < s.max_horizon
        {
            horizon += 1;
            progress = (progress + 1).min(remaining);
            continue;
        }
        if prefix.path.is_empty() {
            fallback = Some(FixedHorizonFallback::PrefixMadeNoProgress);
            finished = true;
            break;
        }
        path.extend(&prefix.path);
        current = prefix.marking;
        remaining = remaining.saturating_sub(prefix.progress);
        previous_estimate = Some(prefix.tail_cost);
        committed.push(horizon);
        progress = s.min_progress.min(remaining);
    }
    // Python's `for ... else`: the rounds ran out without a `break`.
    if !finished {
        fallback = Some(FixedHorizonFallback::MaximumIterations);
    }
    if current != sp.final_marking && fallback.is_none() {
        fallback = Some(FixedHorizonFallback::IncompleteProductPath);
    }

    if fallback.is_some() {
        let q = Query {
            labels,
            log_costs,
            offset: 0,
            start: &net.initial,
            to_final: true,
            max_results: 1,
            max_expansions: s.max_expansions,
            max_post_model_moves: 0,
            deadline,
        };
        let (mut found, st) = search(net, &q, None);
        if found.is_empty() {
            return Ok(None);
        }
        stats += st;
        let f = found.swap_remove(0);
        return Ok(Some(Outcome {
            steps: f.steps,
            cost: f.cost,
            stats,
            lp_solved,
            committed_horizons: committed,
            fallback,
        }));
    }

    let steps: Vec<Step<'l>> = path
        .iter()
        .map(|&m| {
            let cost = sp.cost[m as usize];
            match sp.moves[m as usize] {
                Move::Sync { event, transition } => Step {
                    log: Some(labels[event]),
                    log_index: Some(event),
                    transition: Some(net.index_of(transition)),
                    cost,
                },
                Move::Log { event } => Step {
                    log: Some(labels[event]),
                    log_index: Some(event),
                    transition: None,
                    cost,
                },
                Move::Model { transition } => Step {
                    log: None,
                    log_index: None,
                    transition: Some(net.index_of(transition)),
                    cost,
                },
            }
        })
        .collect();
    let cost = steps.iter().map(|s| s.cost).sum();
    Ok(Some(Outcome {
        steps,
        cost,
        stats,
        lp_solved,
        committed_horizons: committed,
        fallback: None,
    }))
}

/// The moves of the product in pm4py's expansion order: by cost, then by
/// the name of the product transition as Python prints the tuple, such as
/// `('t_a_0', '>>')`.
fn move_order(sp: &SyncProduct, labels: &[&str], names: &[&str]) -> Vec<u32> {
    let trace_name = |e: usize| format!("t_{}_{}", labels[e], e);
    let keys: Vec<(u64, String)> = (0..sp.len())
        .map(|m| {
            let (a, b) = match sp.moves[m] {
                Move::Sync { event, transition } => {
                    (trace_name(event), names[transition.index()].to_owned())
                }
                Move::Log { event } => (trace_name(event), ">>".to_owned()),
                Move::Model { transition } => {
                    (">>".to_owned(), names[transition.index()].to_owned())
                }
            };
            (sp.cost[m], format!("({}, {})", py_repr(&a), py_repr(&b)))
        })
        .collect();
    let mut order: Vec<u32> = (0..sp.len() as u32).collect();
    order.sort_by(|&a, &b| keys[a as usize].cmp(&keys[b as usize]));
    order
}

/// Python's `repr` of a string.
fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

struct Entry {
    marking: Vec<Packed>,
    progress: usize,
    node: u32,
}

const ROOT: u32 = u32::MAX;

/// Heap key of the prefix search: pm4py's tuple `(cost, -progress, depth,
/// counter)`, then the entry.
type HeapKey = Reverse<(u64, Reverse<usize>, usize, u64, u32)>;

/// pm4py's `_solve_prefix`: a cheapest-first enumeration of the product's
/// runs of at most `horizon` moves. Each popped run that reaches the final
/// marking, or moves on at least `min_progress` events, is a candidate,
/// scored by its cost plus the marking-equation cost of the rest.
#[allow(clippy::too_many_arguments)]
fn solve_prefix(
    sp: &SyncProduct,
    order: &[u32],
    equation: &mut StateEquation,
    start: &[Packed],
    horizon: usize,
    min_progress: usize,
    max_states: usize,
    deadline: Option<Instant>,
) -> Result<Option<Prefix>> {
    let mut entries = vec![Entry {
        marking: start.to_vec(),
        progress: 0,
        node: ROOT,
    }];
    let mut nodes: Vec<(u32, u32)> = Vec::new();
    let mut heap: BinaryHeap<HeapKey> = BinaryHeap::new();
    heap.push(Reverse((0, Reverse(0), 0, 0, 0)));
    let mut best: FxHashMap<(Vec<Packed>, usize), u64> = FxHashMap::default();
    best.insert((start.to_vec(), 0), 0);
    let mut tails: FxHashMap<Vec<Packed>, Option<u64>> = FxHashMap::default();
    let mut chosen: Option<Prefix> = None;
    let mut chosen_key = (0, Reverse(0), 0);
    let mut stats = Stats {
        queued: 1,
        ..Stats::default()
    };
    let mut lp_solved = 0;
    let mut counter = 0u64;
    let mut next = Vec::new();

    while stats.visited < max_states && deadline.is_none_or(|d| Instant::now() <= d) {
        let Some(Reverse((cost, _, depth, _, e))) = heap.pop() else {
            break;
        };
        let marking = std::mem::take(&mut entries[e as usize].marking);
        let (progress, node) = (entries[e as usize].progress, entries[e as usize].node);
        if best.get(&(marking.clone(), depth)) != Some(&cost) {
            continue;
        }
        stats.visited += 1;

        let is_final = marking == sp.final_marking;
        if is_final || (depth > 0 && progress >= min_progress) {
            let tail = if is_final {
                Some(0)
            } else if let Some(&t) = tails.get(&marking) {
                t
            } else {
                let t = equation
                    .solve(&marking)?
                    .map(|est| est.objective.round_ties_even().max(0.0) as u64);
                tails.insert(marking.clone(), t);
                lp_solved += 1;
                t
            };
            if let Some(tail_cost) = tail {
                let key = (cost + tail_cost, Reverse(progress), depth);
                if chosen.is_none() || key < chosen_key {
                    chosen_key = key;
                    chosen = Some(Prefix {
                        marking: marking.clone(),
                        path: path(&nodes, node),
                        prefix_cost: cost,
                        tail_cost,
                        progress,
                        stats: Stats::default(),
                        lp_solved: 0,
                    });
                }
            }
        }

        if depth >= horizon || is_final {
            continue;
        }
        for &m in order {
            let enabled = sp.pre(m as usize).iter().all(|&(p, w)| {
                marking
                    .binary_search_by_key(&p, |&x| place(x))
                    .is_ok_and(|i| tokens(marking[i]) >= w)
            });
            if !enabled {
                continue;
            }
            stats.traversed += 1;
            apply_delta(&marking, sp.delta(m as usize), &mut next);
            let new_cost = cost + sp.cost[m as usize];
            let new_progress =
                progress + usize::from(!matches!(sp.moves[m as usize], Move::Model { .. }));
            let key = (next.clone(), depth + 1);
            if best.get(&key).is_some_and(|&b| new_cost >= b) {
                continue;
            }
            best.insert(key, new_cost);
            counter += 1;
            nodes.push((node, m));
            let id = u32::try_from(entries.len()).expect("prefix search fits u32 states");
            entries.push(Entry {
                marking: next.clone(),
                progress: new_progress,
                node: u32::try_from(nodes.len() - 1).expect("prefix search fits u32 states"),
            });
            heap.push(Reverse((
                new_cost,
                Reverse(new_progress),
                depth + 1,
                counter,
                id,
            )));
            stats.queued += 1;
        }
    }
    Ok(chosen.map(|mut c| {
        c.stats = stats;
        c.lp_solved = lp_solved;
        c
    }))
}

/// The moves from the root to `node`.
fn path(nodes: &[(u32, u32)], mut node: u32) -> Vec<u32> {
    let mut out = Vec::new();
    while node != ROOT {
        let (parent, m) = nodes[node as usize];
        out.push(m);
        node = parent;
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::py_repr;

    #[test]
    fn python_repr() {
        assert_eq!(py_repr("a b"), "'a b'");
        assert_eq!(py_repr("it's"), "\"it's\"");
        assert_eq!(py_repr("'\""), "'\\'\"'");
        assert_eq!(py_repr("a\\b\n"), "'a\\\\b\\n'");
    }
}
