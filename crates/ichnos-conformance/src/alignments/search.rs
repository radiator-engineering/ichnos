//! The one search core behind every Petri net alignment variant.
//!
//! It is A* over the markings of the synchronous product. With
//! [`Heuristic::None`] the estimate is always zero and the search is
//! Dijkstra's algorithm (pm4py's `dijkstra_no_heuristics` and
//! `dijkstra_less_memory`). With [`Heuristic::StateEquation`] the estimate
//! comes from the marking equation (pm4py's `state_equation_a_star`).
//!
//! The search follows pm4py's A*: a child reuses its parent's LP solution
//! minus the move just taken. If that vector is still non-negative, the
//! child's estimate is exact and the child is *trusted*; otherwise the
//! estimate is only a lower bound, and the LP is solved again when the child
//! comes off the queue. The queue orders states by `f = g + h`, then trusted
//! before untrusted, then by smaller `h`, as pm4py's `SearchTuple` does. Ties
//! after that go to the deeper, then the newer state. pm4py breaks them by
//! the iteration order of a set of transitions hashed by object id, which no
//! other program can reproduce, so the moves of an optimal alignment can
//! differ from pm4py's while the cost is the same.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::Instant;

use super::Heuristic;
use super::marking::{MarkingStore, Packed, apply_delta, place, tokens};
use super::state_equation::StateEquation;
use super::sync_product::SyncProduct;
use crate::error::Result;

const NONE: u32 = u32::MAX;

/// Search counters, named as in pm4py's result dictionaries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    pub(crate) visited: usize,
    pub(crate) queued: usize,
    pub(crate) traversed: usize,
    pub(crate) lp_solved: usize,
}

/// An optimal path to the final marking.
#[derive(Debug, Clone)]
pub(crate) struct Found {
    /// The moves, as indices into the product's moves.
    pub(crate) path: Vec<u32>,
    pub(crate) cost: u64,
    pub(crate) stats: Stats,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    marking: u32,
    parent: u32,
    mv: u32,
    g: u64,
    depth: u32,
    /// Index of this node's LP solution in `solutions`, once known.
    x: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Open {
    f: u64,
    trusted: bool,
    h: u64,
    depth: u32,
    node: u32,
}

impl Ord for Open {
    /// `BinaryHeap` pops the greatest entry, so the best state compares
    /// greatest: smallest `f`, trusted, smallest `h`, deepest, newest.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f
            .cmp(&self.f)
            .then(self.trusted.cmp(&other.trusted))
            .then(other.h.cmp(&self.h))
            .then(self.depth.cmp(&other.depth))
            .then(self.node.cmp(&other.node))
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Reusable buffers for expanding a marking.
#[derive(Debug, Default)]
pub(crate) struct Expander {
    dense: Vec<u32>,
    stamp: Vec<u32>,
    round: u32,
    pub(crate) enabled: Vec<u32>,
}

impl Expander {
    pub(crate) fn new(sp: &SyncProduct) -> Self {
        Self {
            dense: vec![0; sp.place_count],
            stamp: vec![0; sp.len()],
            round: 0,
            enabled: Vec::new(),
        }
    }

    /// Fills `self.enabled` with the moves enabled in `m`.
    pub(crate) fn enabled_moves(&mut self, sp: &SyncProduct, m: &[Packed]) {
        self.enabled.clear();
        self.round = self.round.wrapping_add(1);
        if self.round == 0 {
            self.stamp.fill(0);
            self.round = 1;
        }
        for &e in m {
            self.dense[place(e) as usize] = tokens(e);
        }
        for &e in m {
            for &t in sp.consumers(place(e)) {
                let slot = &mut self.stamp[t as usize];
                if *slot == self.round {
                    continue;
                }
                *slot = self.round;
                if sp
                    .pre(t as usize)
                    .iter()
                    .all(|&(p, w)| self.dense[p as usize] >= w)
                {
                    self.enabled.push(t);
                }
            }
        }
        self.enabled.extend_from_slice(&sp.always_enabled);
        for &e in m {
            self.dense[place(e) as usize] = 0;
        }
    }
}

/// Finds a cheapest path from the initial to the final marking of `sp`.
///
/// Returns `Ok(None)` if the final marking is unreachable or the deadline
/// passes first.
pub(crate) fn search(
    sp: &SyncProduct,
    heuristic: Heuristic,
    deadline: Option<Instant>,
) -> Result<Option<Found>> {
    let mut lp = match heuristic {
        Heuristic::StateEquation => Some(StateEquation::new(sp)),
        Heuristic::None => None,
    };
    let mut store = MarkingStore::new();
    let (start, _) = store.intern(&sp.initial);
    let (goal, _) = store.intern(&sp.final_marking);
    let mut best_g: Vec<u64> = vec![u64::MAX; store.len()];
    let mut closed: Vec<bool> = vec![false; store.len()];
    let mut solutions: Vec<Vec<f64>> = Vec::new();
    let mut stats = Stats::default();

    let mut h0 = 0;
    let mut x0 = NONE;
    if let Some(lp) = &mut lp {
        stats.lp_solved += 1;
        let Some(est) = lp.solve(&sp.initial)? else {
            return Ok(None);
        };
        h0 = est.h;
        x0 = 0;
        solutions.push(est.x);
    }
    let mut nodes = vec![Node {
        marking: start,
        parent: NONE,
        mv: NONE,
        g: 0,
        depth: 0,
        x: x0,
    }];
    best_g[start as usize] = 0;
    let mut open = BinaryHeap::from([Open {
        f: h0,
        trusted: true,
        h: h0,
        depth: 0,
        node: 0,
    }]);

    let mut expander = Expander::new(sp);
    let mut current: Vec<Packed> = Vec::new();
    let mut next: Vec<Packed> = Vec::new();
    let mut pops: u32 = 0;
    while let Some(mut cur) = open.pop() {
        pops = pops.wrapping_add(1);
        if pops.is_multiple_of(256) && deadline.is_some_and(|d| Instant::now() > d) {
            return Ok(None);
        }
        let node = nodes[cur.node as usize];
        if closed[node.marking as usize] {
            continue;
        }
        if let Some(lp) = &mut lp {
            if !cur.trusted {
                stats.lp_solved += 1;
                let Some(est) = lp.solve(store.get(node.marking))? else {
                    continue;
                };
                nodes[cur.node as usize].x = to_u32(solutions.len());
                solutions.push(est.x);
                let f = node.g + est.h;
                if f > cur.f {
                    open.push(Open {
                        f,
                        trusted: true,
                        h: est.h,
                        ..cur
                    });
                    continue;
                }
                cur.h = est.h;
                cur.trusted = true;
            } else if node.x == NONE {
                let mut x = solutions[nodes[node.parent as usize].x as usize].clone();
                x[node.mv as usize] -= 1.0;
                nodes[cur.node as usize].x = to_u32(solutions.len());
                solutions.push(x);
            }
        }
        if node.marking == goal {
            return Ok(Some(Found {
                path: path_to(&nodes, cur.node),
                cost: node.g,
                stats,
            }));
        }
        closed[node.marking as usize] = true;
        stats.visited += 1;

        current.clear();
        current.extend_from_slice(store.get(node.marking));
        expander.enabled_moves(sp, &current);
        let x_index = nodes[cur.node as usize].x;
        for &t in &expander.enabled {
            stats.traversed += 1;
            apply_delta(&current, sp.delta(t as usize), &mut next);
            let (id, fresh) = store.intern(&next);
            if fresh {
                best_g.push(u64::MAX);
                closed.push(false);
            }
            if closed[id as usize] {
                continue;
            }
            let g = node.g + sp.cost[t as usize];
            if g >= best_g[id as usize] {
                continue;
            }
            best_g[id as usize] = g;
            stats.queued += 1;
            let (h, trusted) = if lp.is_some() {
                let x = &solutions[x_index as usize];
                let exact = StateEquation::derived_is_exact(x, t as usize);
                (cur.h.saturating_sub(sp.cost[t as usize]), exact)
            } else {
                (0, true)
            };
            let child = to_u32(nodes.len());
            nodes.push(Node {
                marking: id,
                parent: cur.node,
                mv: t,
                g,
                depth: node.depth + 1,
                x: NONE,
            });
            open.push(Open {
                f: g + h,
                trusted,
                h,
                depth: node.depth + 1,
                node: child,
            });
        }
    }
    Ok(None)
}

/// The markings in which the cheapest runs of `sp` first mark the last
/// trace place: the end states of the optimal prefix alignments that
/// align-ETConformance precision needs (pm4py's `precision.utils.__search`).
///
/// `sp` should allow only synchronous and silent moves. Every marking that
/// reaches the stop place at the optimal cost is returned, in the order the
/// search finds them; the list is empty if the stop place is unreachable.
pub(crate) fn stop_markings(sp: &SyncProduct) -> Vec<Vec<Packed>> {
    let mut store = MarkingStore::new();
    let (start, _) = store.intern(&sp.initial);
    let mut best_g: Vec<u64> = vec![0];
    let mut closed: Vec<bool> = vec![false];
    let mut g_of: Vec<u64> = vec![0];
    let mut marking_of: Vec<u32> = vec![start];
    let mut open = BinaryHeap::from([Open {
        f: 0,
        trusted: true,
        h: 0,
        depth: 0,
        node: 0,
    }]);
    let mut depth_of: Vec<u32> = vec![0];
    let mut found = Vec::new();
    let mut optimal: Option<u64> = None;
    let mut expander = Expander::new(sp);
    let mut current: Vec<Packed> = Vec::new();
    let mut next: Vec<Packed> = Vec::new();
    while let Some(cur) = open.pop() {
        if optimal.is_some_and(|o| cur.f > o) {
            break;
        }
        let marking = marking_of[cur.node as usize];
        if closed[marking as usize] {
            continue;
        }
        closed[marking as usize] = true;
        current.clear();
        current.extend_from_slice(store.get(marking));
        if current.last().is_some_and(|&e| place(e) == sp.trace_end) {
            found.push(current.clone());
            optimal = Some(cur.f);
            continue;
        }
        let g = g_of[cur.node as usize];
        let depth = depth_of[cur.node as usize];
        expander.enabled_moves(sp, &current);
        for &t in &expander.enabled {
            apply_delta(&current, sp.delta(t as usize), &mut next);
            let (id, fresh) = store.intern(&next);
            if fresh {
                best_g.push(u64::MAX);
                closed.push(false);
            }
            let child_g = g + sp.cost[t as usize];
            if closed[id as usize] || child_g >= best_g[id as usize] {
                continue;
            }
            best_g[id as usize] = child_g;
            let node = to_u32(marking_of.len());
            marking_of.push(id);
            g_of.push(child_g);
            depth_of.push(depth + 1);
            open.push(Open {
                f: child_g,
                trusted: true,
                h: 0,
                depth: depth + 1,
                node,
            });
        }
    }
    found
}

/// The moves from the root to `node`.
fn path_to(nodes: &[Node], mut node: u32) -> Vec<u32> {
    let mut path = Vec::new();
    while nodes[node as usize].parent != NONE {
        path.push(nodes[node as usize].mv);
        node = nodes[node as usize].parent;
    }
    path.reverse();
    path
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("fewer than 2^32 search nodes")
}
