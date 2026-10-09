//! pm4py's `approx_utils.search_alignment`: a best-first search on the
//! model itself, from any marking, over a fragment of the trace.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Instant;

use rustc_hash::FxHashMap;

use super::super::marking::Packed;
use super::net::{Net, Step};

/// The limits and goal of one search.
pub(super) struct Query<'a, 'l> {
    /// The activities of the trace fragment.
    pub(super) labels: &'a [&'l str],
    /// The log move cost of each event of the fragment.
    pub(super) log_costs: &'a [u64],
    /// The index of the fragment's first event in the trace.
    pub(super) offset: usize,
    /// The marking the search starts from.
    pub(super) start: &'a [Packed],
    /// `true`: end in the final marking and return one result. `false`:
    /// any marking will do once the fragment is consumed.
    pub(super) to_final: bool,
    /// The most results to return when `to_final` is `false`.
    pub(super) max_results: usize,
    /// Stop after taking this many states off the queue.
    pub(super) max_expansions: usize,
    /// Model moves allowed after the fragment is consumed, when
    /// `to_final` is `false`.
    pub(super) max_post_model_moves: usize,
    pub(super) deadline: Option<Instant>,
}

/// Search counters, pm4py's `visited_states`, `queued_states` and
/// `traversed_arcs`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Stats {
    pub(super) visited: usize,
    pub(super) queued: usize,
    pub(super) traversed: usize,
}

impl std::ops::AddAssign for Stats {
    fn add_assign(&mut self, o: Self) {
        self.visited += o.visited;
        self.queued += o.queued;
        self.traversed += o.traversed;
    }
}

/// One path found by the search.
#[derive(Debug, Clone)]
pub(super) struct Found<'l> {
    pub(super) steps: Vec<Step<'l>>,
    pub(super) marking: Vec<Packed>,
    pub(super) cost: u64,
    /// The future cost estimate of the end marking (0 with `to_final`).
    pub(super) lower_bound: u64,
}

struct Entry {
    index: usize,
    marking: Vec<Packed>,
    cost: u64,
    node: u32,
    post_model_moves: usize,
}

/// A step and the node before it: paths share their prefixes.
struct Node<'l> {
    parent: u32,
    step: Step<'l>,
}

const ROOT: u32 = u32::MAX;

/// Heap key: pm4py's tuple `(cost + h, -index, len(path), counter)`. The
/// counter is unique, so the order is total and pops are deterministic.
type Key = Reverse<(u64, Reverse<usize>, usize, u64, u32)>;

/// A cost estimate of the rest of an alignment from a marking.
type Estimate<'a> = &'a mut dyn FnMut(&[Packed]) -> u64;

/// Runs the search. `future` is the cost estimate of a marking, used to
/// rank states only when `to_final` is `false`.
pub(super) fn search<'l>(
    net: &Net,
    q: &Query<'_, 'l>,
    future: Option<Estimate<'_>>,
) -> (Vec<Found<'l>>, Stats) {
    let mut zero = |_: &[Packed]| 0;
    let future: &mut dyn FnMut(&[Packed]) -> u64 = match future {
        Some(f) if !q.to_final => f,
        _ => &mut zero,
    };
    let n = q.labels.len();
    let mut entries: Vec<Entry> = Vec::new();
    let mut nodes: Vec<Node<'l>> = Vec::new();
    let mut heap: BinaryHeap<Key> = BinaryHeap::new();
    let mut best: FxHashMap<(usize, Vec<Packed>), u64> = FxHashMap::default();
    let mut counter = 0u64;
    let initial_h = future(q.start);
    entries.push(Entry {
        index: 0,
        marking: q.start.to_vec(),
        cost: 0,
        node: ROOT,
        post_model_moves: 0,
    });
    heap.push(Reverse((initial_h, Reverse(0), 0, counter, 0)));
    best.insert((0, q.start.to_vec()), 0);
    let mut results: Vec<Found<'l>> = Vec::new();
    let mut result_markings: Vec<Vec<Packed>> = Vec::new();
    let mut stats = Stats {
        queued: 1,
        ..Stats::default()
    };
    let mut enabled = Vec::new();

    while stats.visited < q.max_expansions {
        if q.deadline.is_some_and(|d| Instant::now() > d) {
            break;
        }
        let Some(Reverse((_, _, path_len, _, e))) = heap.pop() else {
            break;
        };
        let (index, cost, node, post_model_moves) = {
            let en = &entries[e as usize];
            (en.index, en.cost, en.node, en.post_model_moves)
        };
        let marking = std::mem::take(&mut entries[e as usize].marking);
        if best.get(&(index, marking.clone())) != Some(&cost) {
            continue;
        }
        stats.visited += 1;

        let consumed = index == n;
        if consumed && (!q.to_final || marking == net.final_marking) {
            if !result_markings.contains(&marking) {
                let lower_bound = if q.to_final { 0 } else { future(&marking) };
                results.push(Found {
                    steps: path(&nodes, node),
                    marking: marking.clone(),
                    cost,
                    lower_bound,
                });
                result_markings.push(marking.clone());
                if results.len() >= q.max_results {
                    break;
                }
            }
            if q.to_final {
                break;
            }
        }

        net.enabled(&marking, &mut enabled);
        let mut push = |index: usize,
                        marking: Vec<Packed>,
                        cost: u64,
                        step: Step<'l>,
                        post_model_moves: usize,
                        counter: u64,
                        stats: &mut Stats| {
            let key = (index, marking);
            if best.get(&key).is_some_and(|&b| cost >= b) {
                return;
            }
            let (index, marking) = key;
            best.insert((index, marking.clone()), cost);
            let h = future(&marking);
            nodes.push(Node { parent: node, step });
            let id = u32::try_from(entries.len()).expect("search fits u32 states");
            entries.push(Entry {
                index,
                marking,
                cost,
                node: u32::try_from(nodes.len() - 1).expect("search fits u32 states"),
                post_model_moves,
            });
            heap.push(Reverse((
                cost + h,
                Reverse(index),
                path_len + 1,
                counter,
                id,
            )));
            stats.queued += 1;
        };

        if !consumed {
            let label = q.labels[index];
            for &t in &enabled {
                if net.label(t) == Some(label) {
                    stats.traversed += 1;
                    let sync = net.transitions[t as usize].sync_cost;
                    counter += 1;
                    let step = Step {
                        log: Some(label),
                        log_index: Some(q.offset + index),
                        transition: Some(t),
                        cost: sync,
                    };
                    push(
                        index + 1,
                        net.fire(t, &marking),
                        cost + sync,
                        step,
                        0,
                        counter,
                        &mut stats,
                    );
                }
            }
            stats.traversed += 1;
            let log_cost = q.log_costs[index];
            counter += 1;
            let step = Step {
                log: Some(label),
                log_index: Some(q.offset + index),
                transition: None,
                cost: log_cost,
            };
            push(
                index + 1,
                marking.clone(),
                cost + log_cost,
                step,
                0,
                counter,
                &mut stats,
            );
        }

        if !consumed || q.to_final || post_model_moves < q.max_post_model_moves {
            for &t in &enabled {
                stats.traversed += 1;
                let move_cost = net.transitions[t as usize].model_cost;
                counter += 1;
                let step = Step {
                    log: None,
                    log_index: None,
                    transition: Some(t),
                    cost: move_cost,
                };
                let post = if consumed { post_model_moves + 1 } else { 0 };
                push(
                    index,
                    net.fire(t, &marking),
                    cost + move_cost,
                    step,
                    post,
                    counter,
                    &mut stats,
                );
            }
        }
    }

    results.sort_by_key(|r| (r.cost + r.lower_bound, r.cost));
    (results, stats)
}

/// The steps from the root to `node`.
fn path<'l>(nodes: &[Node<'l>], mut node: u32) -> Vec<Step<'l>> {
    let mut steps = Vec::new();
    while node != ROOT {
        let n = &nodes[node as usize];
        steps.push(n.step);
        node = n.parent;
    }
    steps.reverse();
    steps
}
