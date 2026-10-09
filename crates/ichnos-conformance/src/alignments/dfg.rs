//! Alignments against a directly-follows graph (pm4py's
//! `algo/conformance/alignments/dfg/variants/classic.py`).
//!
//! The model runs from an artificial start node through DFG activities to an
//! artificial end node: start activities follow the start node, end
//! activities lead to the end node, and every other step follows a DFG edge.
//! Costs are pm4py's standard ones: 10000 per log move and per model move,
//! 0 per synchronous move.
//!
//! The search is pm4py's, step for step, so costs match pm4py exactly. It is
//! not an exact shortest-path search, and the cost it returns can exceed the
//! optimum:
//!
//! - It orders states by an internal cost in which a log move on an activity
//!   that is not in the DFG costs 0. The reported cost counts it as 10000.
//! - It closes a DFG node at the first trace position it reaches it with,
//!   and skips later states at that node with an equal or smaller position.
//! - It allows no model move when the next event can move synchronously,
//!   and no log move directly after a model move.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::Dfg;
use rustc_hash::FxHashMap;

use super::costs::{STD_LOG_MOVE_COST, STD_MODEL_MOVE_COST, STD_SYNC_MOVE_COST};
use super::result::{LogAlignment, SequenceAlignment, SequenceMove};
use crate::error::{Error, Result};

/// A DFG prepared for alignments: activities numbered in name order, then
/// the start and end nodes.
#[derive(Debug, Clone)]
pub struct DfgAligner {
    names: Vec<String>,
    index: FxHashMap<String, u32>,
    /// Successors of each node, sorted.
    outgoing: Vec<Vec<u32>>,
    empty_cost: u64,
}

/// A search state; field order is pm4py's tuple order for comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    g: u64,
    /// Minus the number of events explained.
    log: i64,
    visited: u64,
    real_f: u64,
    f: u64,
    node: u32,
    is_log: bool,
    is_model: bool,
}

#[derive(Debug, Clone, Copy)]
struct State {
    key: Key,
    prev: Option<u32>,
}

impl DfgAligner {
    /// Prepares `dfg`. Fails with [`Error::DfgEndUnreachable`] when no path
    /// leads from a start activity to an end activity: then no trace can be
    /// aligned.
    pub fn new(dfg: &Dfg) -> Result<Self> {
        // Activities in name order, so node order is pm4py's string order.
        let names: Vec<String> = dfg
            .vertices()
            .into_iter()
            .map(|l| l.as_str().to_owned())
            .collect();
        let index: FxHashMap<String, u32> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), to_u32(i)))
            .collect();
        let n = names.len();
        let (start, end) = (to_u32(n), to_u32(n + 1));
        let mut outgoing = vec![Vec::new(); n + 2];
        for (a, b) in dfg.graph.keys() {
            outgoing[index[a.as_str()] as usize].push(index[b.as_str()]);
        }
        for a in dfg.start_activities.keys() {
            outgoing[start as usize].push(index[a.as_str()]);
        }
        for a in dfg.end_activities.keys() {
            outgoing[index[a.as_str()] as usize].push(end);
        }
        for o in &mut outgoing {
            o.sort_unstable();
            o.dedup();
        }
        let mut aligner = Self {
            names,
            index,
            outgoing,
            empty_cost: 0,
        };
        let empty = aligner
            .search::<&str>(&[])
            .ok_or(Error::DfgEndUnreachable)?;
        aligner.empty_cost = empty.cost;
        Ok(aligner)
    }

    fn start(&self) -> u32 {
        to_u32(self.names.len())
    }

    fn end(&self) -> u32 {
        to_u32(self.names.len() + 1)
    }

    /// The cost of aligning the empty trace.
    pub fn empty_trace_cost(&self) -> u64 {
        self.empty_cost
    }

    /// Aligns one trace, given as activity names. `None` when the search
    /// finds no alignment, which pm4py also returns.
    pub fn align<S: AsRef<str>>(&self, trace: &[S]) -> Option<SequenceAlignment> {
        let mut a = self.search(trace)?;
        let bwc = self.empty_cost + STD_LOG_MOVE_COST * trace.len() as u64;
        a.best_worst_cost = bwc;
        a.fitness = 1.0 - a.cost as f64 / bwc as f64;
        Some(a)
    }

    /// Aligns every trace of `log`, once per variant (pm4py's `apply_log`).
    pub fn align_log(
        &self,
        log: &EventLog,
        keys: &EventKeys,
    ) -> Result<LogAlignment<SequenceAlignment>> {
        let variants = log.variants(keys)?;
        let alignments = variants
            .iter()
            .map(|v| self.align(&variants.names(v).collect::<Vec<_>>()))
            .collect();
        Ok(LogAlignment::new(variants, alignments))
    }

    fn search<S: AsRef<str>>(&self, trace: &[S]) -> Option<SequenceAlignment> {
        let len = trace.len() as i64;
        // The DFG node of each event, or None for activities not in the DFG.
        let nodes: Vec<Option<u32>> = trace
            .iter()
            .map(|a| self.index.get(a.as_ref()).copied())
            .collect();
        let (start, end) = (self.start(), self.end());
        let mut closed = vec![-1i64; self.names.len() + 2];
        let mut states: Vec<State> = Vec::new();
        let mut open: BinaryHeap<Reverse<(Key, u32)>> = BinaryHeap::new();
        let push = |states: &mut Vec<State>,
                    open: &mut BinaryHeap<Reverse<(Key, u32)>>,
                    key: Key,
                    prev: u32| {
            let id = to_u32(states.len());
            states.push(State {
                key,
                prev: Some(prev),
            });
            open.push(Reverse((key, id)));
        };
        states.push(State {
            key: Key {
                g: 0,
                log: 0,
                visited: 0,
                real_f: 0,
                f: 0,
                node: start,
                is_log: false,
                is_model: false,
            },
            prev: None,
        });
        open.push(Reverse((states[0].key, 0)));
        let mut visited = 0u64;
        let mut closed_count = 0usize;
        while let Some(Reverse((cur, id))) = open.pop() {
            visited += 1;
            let pos = -cur.log;
            if pos <= closed[cur.node as usize] {
                continue;
            }
            closed[cur.node as usize] = pos;
            let not_end = pos < len;
            if cur.node == end && !not_end {
                return Some(self.result(&states, id, trace.len(), closed_count));
            }
            closed_count += 1;
            let event = not_end.then(|| nodes[pos as usize]);
            if cur.node != end {
                let succ = &self.outgoing[cur.node as usize];
                let in_model = event.is_none_or(|e| e.is_some());
                if in_model {
                    let sync = event.flatten().filter(|e| succ.binary_search(e).is_ok());
                    if let Some(e) = sync {
                        let key = Key {
                            g: cur.f + STD_SYNC_MOVE_COST,
                            log: cur.log - 1,
                            visited,
                            real_f: cur.real_f + STD_SYNC_MOVE_COST,
                            f: cur.f + STD_SYNC_MOVE_COST,
                            node: e,
                            is_log: false,
                            is_model: false,
                        };
                        if pos + 1 > closed[e as usize] {
                            push(&mut states, &mut open, key, id);
                        }
                    }
                    for &act in succ {
                        if act == end {
                            let key = Key {
                                visited,
                                node: end,
                                is_log: false,
                                is_model: false,
                                ..cur
                            };
                            if pos > closed[end as usize] {
                                push(&mut states, &mut open, key, id);
                            }
                        } else if sync.is_none() {
                            let key = Key {
                                g: cur.f + STD_MODEL_MOVE_COST,
                                log: cur.log,
                                visited,
                                real_f: cur.real_f + STD_MODEL_MOVE_COST,
                                f: cur.f + STD_MODEL_MOVE_COST,
                                node: act,
                                is_log: false,
                                is_model: true,
                            };
                            if pos > closed[act as usize] {
                                push(&mut states, &mut open, key, id);
                            }
                        }
                    }
                }
            }
            if let Some(e) = event
                && !cur.is_model
            {
                // pm4py's internal cost: free for activities not in the DFG.
                let internal = if e.is_some() { STD_LOG_MOVE_COST } else { 0 };
                let key = Key {
                    g: cur.f + internal,
                    log: cur.log - 1,
                    visited,
                    real_f: cur.real_f + STD_LOG_MOVE_COST,
                    f: cur.f + internal,
                    node: cur.node,
                    is_log: true,
                    is_model: false,
                };
                if pos + 1 > closed[cur.node as usize] {
                    push(&mut states, &mut open, key, id);
                }
            }
        }
        None
    }

    fn result(
        &self,
        states: &[State],
        last: u32,
        events: usize,
        closed: usize,
    ) -> SequenceAlignment {
        let end = self.end();
        let mut moves = Vec::new();
        let mut cur = states[last as usize];
        while let Some(prev) = cur.prev {
            let k = cur.key;
            // The event this move explains, counted from the front.
            let explained = (-k.log) as usize;
            if k.is_log {
                moves.push(SequenceMove::Log {
                    event: explained - 1,
                });
            } else if k.is_model {
                moves.push(SequenceMove::Model {
                    activity: Some(self.names[k.node as usize].as_str().into()),
                });
            } else if k.node != end {
                moves.push(SequenceMove::Sync {
                    event: explained - 1,
                });
            }
            cur = states[prev as usize];
        }
        moves.reverse();
        debug_assert_eq!(moves.iter().filter_map(SequenceMove::event).count(), events);
        let key = states[last as usize].key;
        SequenceAlignment {
            moves,
            cost: key.real_f,
            fitness: 0.0,
            best_worst_cost: 0,
            visited_states: key.visited as usize,
            closed_states: closed,
        }
    }
}

/// Aligns every trace of `log` against `dfg` (pm4py's
/// `conformance_diagnostics_alignments` with a DFG).
pub fn align_log_dfg(
    log: &EventLog,
    dfg: &Dfg,
    keys: &EventKeys,
) -> Result<LogAlignment<SequenceAlignment>> {
    DfgAligner::new(dfg)?.align_log(log, keys)
}

fn to_u32(i: usize) -> u32 {
    u32::try_from(i).expect("DFG node count fits u32")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dfg() -> Dfg {
        // a -> b -> c, with b optional.
        let mut d = Dfg::new();
        d.add_edge("a", "b", 1);
        d.add_edge("b", "c", 1);
        d.add_edge("a", "c", 1);
        d.add_start("a", 1);
        d.add_end("c", 1);
        d
    }

    #[test]
    fn fitting_and_deviating_traces() {
        let al = DfgAligner::new(&dfg()).unwrap();
        // Empty trace: model moves a, c.
        assert_eq!(al.empty_trace_cost(), 20_000);
        let a = al.align(&["a", "b", "c"]).unwrap();
        assert_eq!(a.cost, 0);
        assert_eq!(a.fitness, 1.0);
        assert_eq!(
            a.moves,
            [
                SequenceMove::Sync { event: 0 },
                SequenceMove::Sync { event: 1 },
                SequenceMove::Sync { event: 2 },
            ]
        );
        // x is not in the DFG: one log move.
        let a = al.align(&["a", "x", "c"]).unwrap();
        assert_eq!(a.cost, 10_000);
        assert_eq!(a.best_worst_cost, 50_000);
        assert_eq!(a.fitness, 1.0 - 10_000.0 / 50_000.0);
        // Missing c: one model move.
        let a = al.align(&["a"]).unwrap();
        assert_eq!(a.cost, 10_000);
        assert_eq!(
            a.moves.last(),
            Some(&SequenceMove::Model {
                activity: Some("c".into())
            })
        );
    }

    #[test]
    fn unreachable_end_is_an_error() {
        let mut d = Dfg::new();
        d.add_edge("a", "b", 1);
        d.add_start("a", 1);
        assert!(matches!(DfgAligner::new(&d), Err(Error::DfgEndUnreachable)));
    }
}
