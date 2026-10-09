//! Directly-follows graphs, ported from pm4py's `objects/dfg/` and the
//! DFG helpers in `objects/dfg/utils/dfg_utils.py`.

mod filtering;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::Label;

/// Occurrence counts per activity, as pm4py's `activities_count` dicts.
pub type ActivityCounts = BTreeMap<Label, u64>;

/// Errors raised by DFG operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DfgError {
    /// The activity is not in the activity counts.
    #[error("activity {0:?} is not in the DFG")]
    UnknownActivity(Label),
}

/// A directly-follows graph: edge frequencies plus start and end activity
/// frequencies (pm4py's `DirectlyFollowsGraph`).
///
/// Maps are ordered, so every iteration and every derived structure is
/// deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dfg {
    /// How often the second activity directly follows the first.
    pub graph: BTreeMap<(Label, Label), u64>,
    /// How often each activity starts a trace.
    pub start_activities: BTreeMap<Label, u64>,
    /// How often each activity ends a trace.
    pub end_activities: BTreeMap<Label, u64>,
}

impl Dfg {
    /// Creates an empty DFG.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `count` to the frequency of the edge `a -> b`.
    pub fn add_edge(&mut self, a: impl Into<Label>, b: impl Into<Label>, count: u64) {
        *self.graph.entry((a.into(), b.into())).or_insert(0) += count;
    }

    /// Adds `count` to the start frequency of `a`.
    pub fn add_start(&mut self, a: impl Into<Label>, count: u64) {
        *self.start_activities.entry(a.into()).or_insert(0) += count;
    }

    /// Adds `count` to the end frequency of `a`.
    pub fn add_end(&mut self, a: impl Into<Label>, count: u64) {
        *self.end_activities.entry(a.into()).or_insert(0) += count;
    }

    /// All activities: edge ends plus start and end activities (pm4py's
    /// `dfg.util.get_vertices`).
    pub fn vertices(&self) -> BTreeSet<Label> {
        let mut v: BTreeSet<Label> = self
            .graph
            .keys()
            .flat_map(|(a, b)| [a.clone(), b.clone()])
            .collect();
        v.extend(self.start_activities.keys().cloned());
        v.extend(self.end_activities.keys().cloned());
        v
    }

    /// Activities that occur on some edge, sorted (pm4py's
    /// `dfg_utils.get_activities_from_dfg`).
    pub fn edge_activities(&self) -> BTreeSet<Label> {
        self.graph
            .keys()
            .flat_map(|(a, b)| [a.clone(), b.clone()])
            .collect()
    }

    /// Successors of `a` with edge frequencies.
    pub fn outgoing(&self, a: &str) -> BTreeMap<&Label, u64> {
        self.graph
            .iter()
            .filter(|((x, _), _)| x.as_str() == a)
            .map(|((_, y), &n)| (y, n))
            .collect()
    }

    /// Predecessors of `b` with edge frequencies.
    pub fn incoming(&self, b: &str) -> BTreeMap<&Label, u64> {
        self.graph
            .iter()
            .filter(|((_, y), _)| y.as_str() == b)
            .map(|((x, _), &n)| (x, n))
            .collect()
    }

    /// Vertices without incoming edges (pm4py's `get_source_vertices`).
    pub fn source_vertices(&self) -> BTreeSet<Label> {
        let targets: BTreeSet<&Label> = self.graph.keys().map(|(_, b)| b).collect();
        self.vertices()
            .into_iter()
            .filter(|v| !targets.contains(v))
            .collect()
    }

    /// Vertices without outgoing edges (pm4py's `get_sink_vertices`).
    pub fn sink_vertices(&self) -> BTreeSet<Label> {
        let sources: BTreeSet<&Label> = self.graph.keys().map(|(a, _)| a).collect();
        self.vertices()
            .into_iter()
            .filter(|v| !sources.contains(v))
            .collect()
    }

    /// Activities with an outgoing edge but no incoming edge (pm4py's
    /// `dfg_utils.infer_start_activities`).
    pub fn infer_start_activities(&self) -> BTreeSet<Label> {
        let targets: BTreeSet<&Label> = self.graph.keys().map(|(_, b)| b).collect();
        self.graph
            .keys()
            .map(|(a, _)| a)
            .filter(|a| !targets.contains(a))
            .cloned()
            .collect()
    }

    /// Activities with an incoming edge but no outgoing edge (pm4py's
    /// `dfg_utils.infer_end_activities`).
    pub fn infer_end_activities(&self) -> BTreeSet<Label> {
        let sources: BTreeSet<&Label> = self.graph.keys().map(|(a, _)| a).collect();
        self.graph
            .keys()
            .map(|(_, b)| b)
            .filter(|b| !sources.contains(b))
            .cloned()
            .collect()
    }

    /// How often each vertex occurs: its outgoing edge frequencies plus its
    /// start frequency (pm4py's `get_vertex_frequencies`).
    pub fn vertex_frequencies(&self) -> ActivityCounts {
        let mut c: ActivityCounts = self.vertices().into_iter().map(|v| (v, 0)).collect();
        for ((a, _), &n) in &self.graph {
            *c.get_mut(a).expect("edge ends are vertices") += n;
        }
        for (a, &n) in &self.start_activities {
            *c.get_mut(a).expect("start activities are vertices") += n;
        }
        c
    }

    /// The largest frequency of an edge into or out of `act`, or `None` if
    /// it has no edge (pm4py's `get_max_activity_count` returns -1).
    pub fn max_activity_count(&self, act: &str) -> Option<u64> {
        self.graph
            .iter()
            .filter(|((a, b), _)| a.as_str() == act || b.as_str() == act)
            .map(|(_, &n)| n)
            .max()
    }

    /// For each vertex, the activities reachable from it by one or more
    /// edges (pm4py's `dfg_utils.get_successors` and the `post` part of
    /// `get_transitive_relations`).
    pub fn successors(&self) -> BTreeMap<Label, BTreeSet<Label>> {
        self.closure(false)
    }

    /// For each vertex, the activities that reach it by one or more edges
    /// (pm4py's `dfg_utils.get_predecessors`).
    pub fn predecessors(&self) -> BTreeMap<Label, BTreeSet<Label>> {
        self.closure(true)
    }

    fn closure(&self, reverse: bool) -> BTreeMap<Label, BTreeSet<Label>> {
        let mut adj: BTreeMap<&Label, Vec<&Label>> = BTreeMap::new();
        for (a, b) in self.graph.keys() {
            let (from, to) = if reverse { (b, a) } else { (a, b) };
            adj.entry(from).or_default().push(to);
        }
        self.vertices()
            .into_iter()
            .map(|v| {
                let mut seen: BTreeSet<Label> = BTreeSet::new();
                let mut queue: VecDeque<&Label> =
                    adj.get(&v).into_iter().flatten().copied().collect();
                while let Some(x) = queue.pop_front() {
                    if seen.insert(x.clone()) {
                        queue.extend(adj.get(x).into_iter().flatten().copied());
                    }
                }
                (v, seen)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
