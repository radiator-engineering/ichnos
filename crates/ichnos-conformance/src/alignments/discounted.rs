//! Discounted alignments (pm4py's `discounted_a_star` variant, by
//! Boltenhagen).
//!
//! A log or model move taken after `l` earlier moves costs `exponent^-l`; a
//! synchronous move costs 0. Silent model moves cost the same as visible
//! ones, and the model and log move costs of the [`Aligner`] play no part.
//! Deviations late in the trace therefore cost less than early ones.
//!
//! The search is pm4py's: Dijkstra's algorithm over the markings of the
//! synchronous product, which closes a marking the first time it comes off
//! the queue. The cost of a move depends on the path length, not only on
//! the marking, so closing markings is a heuristic and the cost found can
//! exceed the cheapest discounted alignment. The queue orders states by
//! cost, then deeper first, as pm4py's `DijkstraSearchTuple` does. pm4py
//! breaks the remaining ties by the iteration order of sets of transitions
//! hashed by object id, so its cost for one trace can change from run to
//! run. ichnos breaks them by queue order, oldest first.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::Instant;

use ichnos_core::{EventKeys, EventLog};

use super::Aligner;
use super::marking::{MarkingStore, Packed, apply_delta};
use super::result::{LogAlignment, Move};
use super::search::Expander;
use super::sync_product::{MoveSet, SyncProduct};
use crate::error::{Error, Result};

/// pm4py's default `exponent`.
pub const DEFAULT_DISCOUNT_EXPONENT: f64 = 2.0;

/// A discounted alignment of one trace.
///
/// pm4py also reports `fitness` and `bwc` for this variant, but both are
/// always 0: the variant does not hand its trace costs back to
/// `apply_trace`, and its best-worst cost is the discounted cost of the
/// empty trace divided by 10000 with integer division. ichnos leaves them
/// out.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscountedAlignment {
    /// The moves, in order.
    pub moves: Vec<Move>,
    /// The sum of `exponent^-l` over the log and model moves, where `l` is
    /// the number of moves before each one.
    pub cost: f64,
    /// Markings taken off the queue and expanded.
    pub visited_states: usize,
    /// States put on the queue.
    pub queued_states: usize,
    /// Moves tried from expanded markings.
    pub traversed_arcs: usize,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    marking: u32,
    parent: u32,
    mv: u32,
    g: f64,
    depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Open {
    g: f64,
    depth: u32,
    node: u32,
}

impl Eq for Open {}

impl Ord for Open {
    /// `BinaryHeap` pops the greatest entry, so the best state compares
    /// greatest: smallest cost, deepest, oldest.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .g
            .total_cmp(&self.g)
            .then(self.depth.cmp(&other.depth))
            .then(other.node.cmp(&self.node))
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const NONE: u32 = u32::MAX;

impl Aligner {
    /// Aligns a trace with pm4py's discounted costs and search. `exponent`
    /// is the base of the discount (pm4py's `exponent`, 2 by default).
    /// Returns `None` if the final marking is unreachable or the time limit
    /// passed.
    ///
    /// Fails with [`Error::DiscountExponent`] unless `exponent` is finite
    /// and positive.
    pub fn align_discounted<S: AsRef<str>>(
        &self,
        trace: &[S],
        exponent: f64,
    ) -> Result<Option<DiscountedAlignment>> {
        if !(exponent.is_finite() && exponent > 0.0) {
            return Err(Error::DiscountExponent(exponent));
        }
        let deadline = self.options().trace_time_limit.map(|d| Instant::now() + d);
        let zeros = vec![0; trace.len()];
        let sp = SyncProduct::new(self.model(), trace, &zeros, MoveSet::All);
        Ok(search(&sp, exponent, deadline))
    }

    /// Aligns every variant of `log` once with discounted costs.
    pub fn align_log_discounted(
        &self,
        log: &EventLog,
        keys: &EventKeys,
        exponent: f64,
    ) -> Result<LogAlignment<DiscountedAlignment>> {
        let variants = log.variants(keys)?;
        let alignments = variants
            .iter()
            .map(|v| self.align_discounted(&variants.names(v).collect::<Vec<_>>(), exponent))
            .collect::<Result<Vec<_>>>()?;
        Ok(LogAlignment::new(variants, alignments))
    }
}

fn search(
    sp: &SyncProduct,
    exponent: f64,
    deadline: Option<Instant>,
) -> Option<DiscountedAlignment> {
    let mut store = MarkingStore::new();
    let (start, _) = store.intern(&sp.initial);
    let (goal, _) = store.intern(&sp.final_marking);
    let mut closed = vec![false; store.len()];
    // The cheapest cost queued for each marking; dearer entries can never
    // come off the queue first, so they are not queued.
    let mut best_g = vec![f64::INFINITY; store.len()];
    let mut nodes = vec![Node {
        marking: start,
        parent: NONE,
        mv: NONE,
        g: 0.0,
        depth: 0,
    }];
    best_g[start as usize] = 0.0;
    let mut open = BinaryHeap::from([Open {
        g: 0.0,
        depth: 0,
        node: 0,
    }]);
    let (mut visited, mut queued, mut traversed) = (0, 0, 0);
    let mut expander = Expander::new(sp);
    let mut current: Vec<Packed> = Vec::new();
    let mut next: Vec<Packed> = Vec::new();
    let mut pops: u32 = 0;
    while let Some(cur) = open.pop() {
        pops = pops.wrapping_add(1);
        if pops.is_multiple_of(256) && deadline.is_some_and(|d| Instant::now() > d) {
            return None;
        }
        let node = nodes[cur.node as usize];
        if closed[node.marking as usize] {
            continue;
        }
        if node.marking == goal {
            let mut moves = Vec::new();
            let mut n = cur.node;
            while nodes[n as usize].parent != NONE {
                moves.push(sp.moves[nodes[n as usize].mv as usize]);
                n = nodes[n as usize].parent;
            }
            moves.reverse();
            return Some(DiscountedAlignment {
                moves,
                cost: node.g,
                visited_states: visited,
                queued_states: queued,
                traversed_arcs: traversed,
            });
        }
        closed[node.marking as usize] = true;
        visited += 1;
        // pm4py's `expo**(-l)`.
        let step = exponent.powf(-f64::from(node.depth));
        current.clear();
        current.extend_from_slice(store.get(node.marking));
        expander.enabled_moves(sp, &current);
        for &t in &expander.enabled {
            traversed += 1;
            apply_delta(&current, sp.delta(t as usize), &mut next);
            let (id, fresh) = store.intern(&next);
            if fresh {
                closed.push(false);
                best_g.push(f64::INFINITY);
            }
            if closed[id as usize] {
                continue;
            }
            let cost = match sp.moves[t as usize] {
                Move::Sync { .. } => 0.0,
                Move::Log { .. } | Move::Model { .. } => step,
            };
            let g = node.g + cost;
            if g > best_g[id as usize] {
                continue;
            }
            best_g[id as usize] = g;
            queued += 1;
            let child = u32::try_from(nodes.len()).expect("fewer than 2^32 search nodes");
            nodes.push(Node {
                marking: id,
                parent: cur.node,
                mv: t,
                g,
                depth: node.depth + 1,
            });
            open.push(Open {
                g,
                depth: node.depth + 1,
                node: child,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use ichnos_model::{Marking, PetriNet};

    use super::*;
    use crate::alignments::AlignmentOptions;

    /// source -> a -> p -> tau -> q -> b -> sink
    fn net() -> (PetriNet, Marking, Marking) {
        let mut net = PetriNet::new("n");
        let source = net.add_place("source");
        let p = net.add_place("p");
        let q = net.add_place("q");
        let sink = net.add_place("sink");
        let a = net.add_transition("a", Some("a"));
        let tau = net.add_transition("tau", None::<&str>);
        let b = net.add_transition("b", Some("b"));
        net.add_input_arc(source, a).unwrap();
        net.add_output_arc(a, p).unwrap();
        net.add_input_arc(p, tau).unwrap();
        net.add_output_arc(tau, q).unwrap();
        net.add_input_arc(q, b).unwrap();
        net.add_output_arc(b, sink).unwrap();
        (
            net,
            Marking::from([(source, 1)]),
            Marking::from([(sink, 1)]),
        )
    }

    #[test]
    fn deviations_cost_less_later() {
        let (net, im, fm) = net();
        let al = Aligner::new(&net, &im, &fm, AlignmentOptions::default()).unwrap();
        // Fits: only the silent move, second, costs 2^-1.
        let a = al.align_discounted(&["a", "b"], 2.0).unwrap().unwrap();
        assert_eq!(a.cost, 0.5);
        assert_eq!(a.moves.len(), 3);
        // Extra c at the end: silent move 2^-1, log move on c 2^-3 or
        // earlier. The cheapest puts the log move last.
        let a = al.align_discounted(&["a", "b", "c"], 2.0).unwrap().unwrap();
        assert_eq!(a.cost, 0.5 + 0.125);
        assert_eq!(a.moves.last(), Some(&Move::Log { event: 2 }));
        // Base 3.
        let a = al.align_discounted(&["a", "b"], 3.0).unwrap().unwrap();
        assert!((a.cost - 1.0 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn rejects_bad_exponents() {
        let (net, im, fm) = net();
        let al = Aligner::new(&net, &im, &fm, AlignmentOptions::default()).unwrap();
        for e in [0.0, -2.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                al.align_discounted(&["a"], e),
                Err(Error::DiscountExponent(_))
            ));
        }
    }
}
