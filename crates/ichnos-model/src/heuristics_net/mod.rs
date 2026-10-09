//! Heuristics nets, ported from pm4py's `objects/heuristics_net/`.
//!
//! A [`HeuristicsNet`] is what the Heuristics Miner produces: activities
//! (nodes) joined by dependency edges, with AND measures that say which
//! successors (or predecessors) of an activity run in parallel. Discovery
//! fills it in; this module holds the type, the AND and length-two loop
//! measures that pm4py computes on its nodes, and merging. The conversion to
//! a Petri net is [`HeuristicsNet::to_petri_net`].

use std::collections::{BTreeMap, BTreeSet};

use crate::{Dfg, Label};

/// A matrix indexed by two activities, as pm4py's nested dicts.
pub type Matrix<T> = BTreeMap<Label, BTreeMap<Label, T>>;

/// pm4py's `defaults.DEFAULT_DEPENDENCY_THRESH`.
pub const DEFAULT_DEPENDENCY_THRESH: f64 = 0.5;
/// pm4py's `defaults.DEFAULT_AND_MEASURE_THRESH`.
pub const DEFAULT_AND_MEASURE_THRESH: f64 = 0.65;
/// pm4py's `defaults.DEFAULT_LOOP_LENGTH_TWO_THRESH`.
pub const DEFAULT_LOOP_LENGTH_TWO_THRESH: f64 = 0.5;

/// One connection between two nodes (pm4py's `heuristics_net.edge.Edge`).
///
/// pm4py's colours, pen widths and performance values are drawing concerns
/// and are not stored here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeuristicsEdge {
    /// The dependency measure of the connection.
    pub dependency: f64,
    /// The directly-follows frequency of the connection.
    pub frequency: u64,
}

/// A node of a heuristics net: one activity (pm4py's
/// `heuristics_net.node.Node`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeuristicsNode {
    /// The activity.
    pub name: Label,
    /// How often the activity occurs.
    pub occurrences: u64,
    /// Outgoing connections per target node. A target can have more than
    /// one edge, for example after [`HeuristicsNet::merge`].
    pub outputs: BTreeMap<Label, Vec<HeuristicsEdge>>,
    /// Incoming connections per source node.
    pub inputs: BTreeMap<Label, Vec<HeuristicsEdge>>,
    /// AND measures of pairs of predecessors, stored once per unordered pair
    /// with the smaller name first.
    pub and_measures_in: Matrix<f64>,
    /// AND measures of pairs of successors, stored like
    /// [`and_measures_in`](Self::and_measures_in).
    pub and_measures_out: Matrix<f64>,
    /// Activities that form a loop of length two with this one, with the
    /// directly-follows frequency from this activity to them.
    pub loop_length_two: BTreeMap<Label, u64>,
}

/// A heuristics net (pm4py's `HeuristicsNet`).
///
/// Discovery sets the matrices and adds nodes and connections. Fields that
/// pm4py keeps only for drawing (colours, performance values) are left out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeuristicsNet {
    /// All activities.
    pub activities: BTreeSet<Label>,
    /// How often each activity occurs.
    pub activity_occurrences: BTreeMap<Label, u64>,
    /// Start activities with their frequencies. There is one map per net:
    /// a merged net ([`HeuristicsNet::merge`]) has several, and the Petri
    /// net conversion gives each its own source place.
    pub start_activities: Vec<BTreeMap<Label, u64>>,
    /// End activities with their frequencies, one map per net.
    pub end_activities: Vec<BTreeMap<Label, u64>>,
    /// The directly-follows frequencies.
    pub dfg: BTreeMap<(Label, Label), u64>,
    /// The directly-follows frequencies as a matrix.
    pub dfg_matrix: Matrix<u64>,
    /// The dependency measure of each directly-follows pair.
    pub dependency_matrix: Matrix<f64>,
    /// Frequencies of `a` followed by `b` with one event in between.
    pub dfg_window_2_matrix: Matrix<u64>,
    /// Frequencies of `a b a` patterns, by `a` then `b`.
    pub freq_triples_matrix: Matrix<u64>,
    /// The nodes, by activity.
    pub nodes: BTreeMap<Label, HeuristicsNode>,
}

/// pm4py's `dfg_utils.sum_activities_count` for one activity: the sum of the
/// frequencies of its incoming and outgoing edges, halved (rounding down)
/// when it has both.
pub fn activity_count_from_dfg(dfg: &BTreeMap<(Label, Label), u64>, act: &str) -> u64 {
    let out: Option<u64> = dfg
        .iter()
        .filter(|((a, _), _)| a == act)
        .map(|(_, &n)| n)
        .reduce(|x, y| x + y);
    let inc: Option<u64> = dfg
        .iter()
        .filter(|((_, b), _)| b == act)
        .map(|(_, &n)| n)
        .reduce(|x, y| x + y);
    let sum = out.unwrap_or(0) + inc.unwrap_or(0);
    if out.is_some() && inc.is_some() {
        sum / 2
    } else {
        sum
    }
}

fn get<T: Copy + Default>(m: &Matrix<T>, a: &Label, b: &Label) -> T {
    m.get(a).and_then(|r| r.get(b)).copied().unwrap_or_default()
}

impl HeuristicsNet {
    /// Creates a heuristics net without nodes from a DFG, as pm4py's
    /// constructor does when only the DFG is given.
    ///
    /// Activities are those on the DFG's edges. Each occurrence count is
    /// [`activity_count_from_dfg`]. Start and end activities come from the
    /// DFG; when it has none, they are inferred from the edges and get
    /// frequency 0 (pm4py stores a plain set then).
    pub fn from_dfg(dfg: &Dfg) -> Self {
        let activities = dfg.edge_activities();
        let activity_occurrences = activities
            .iter()
            .map(|a| (a.clone(), activity_count_from_dfg(&dfg.graph, a)))
            .collect();
        let or_inferred = |given: &BTreeMap<Label, u64>, inferred: BTreeSet<Label>| {
            if given.is_empty() {
                inferred.into_iter().map(|a| (a, 0)).collect()
            } else {
                given.clone()
            }
        };
        Self {
            activities,
            activity_occurrences,
            start_activities: vec![or_inferred(
                &dfg.start_activities,
                dfg.infer_start_activities(),
            )],
            end_activities: vec![or_inferred(&dfg.end_activities, dfg.infer_end_activities())],
            dfg: dfg.graph.clone(),
            ..Self::default()
        }
    }

    /// Returns the node for `act`, adding it with the activity's occurrence
    /// count (0 if unknown) if it does not exist.
    pub fn node_mut(&mut self, act: &Label) -> &mut HeuristicsNode {
        let occurrences = self.activity_occurrences.get(act).copied().unwrap_or(0);
        self.nodes
            .entry(act.clone())
            .or_insert_with(|| HeuristicsNode {
                name: act.clone(),
                occurrences,
                ..HeuristicsNode::default()
            })
    }

    /// Adds an output connection `from -> to` on `from` (pm4py's
    /// `Node.add_output_connection`). Adds missing nodes.
    pub fn add_output_connection(&mut self, from: &Label, to: &Label, edge: HeuristicsEdge) {
        self.node_mut(to);
        self.node_mut(from)
            .outputs
            .entry(to.clone())
            .or_default()
            .push(edge);
    }

    /// Adds an input connection `from -> to` on `to` (pm4py's
    /// `Node.add_input_connection`). Adds missing nodes.
    pub fn add_input_connection(&mut self, to: &Label, from: &Label, edge: HeuristicsEdge) {
        self.node_mut(from);
        self.node_mut(to)
            .inputs
            .entry(from.clone())
            .or_default()
            .push(edge);
    }

    /// AND measures of the pairs of `act`'s successors that reach
    /// `threshold` (pm4py's `Node.calculate_and_measure_out`).
    ///
    /// For successors `a < b` the measure is
    /// `(|a>b| + |b>a|) / (|act>a| + |act>b| + 1)`.
    pub fn and_measures_out(&self, act: &Label, threshold: f64) -> Matrix<f64> {
        let Some(node) = self.nodes.get(act) else {
            return Matrix::new();
        };
        self.and_measures(node.outputs.keys(), threshold, |n| {
            get(&self.dfg_matrix, act, n)
        })
    }

    /// AND measures of the pairs of `act`'s predecessors that reach
    /// `threshold` (pm4py's `Node.calculate_and_measure_in`).
    ///
    /// For predecessors `a < b` the measure is
    /// `(|a>b| + |b>a|) / (|a>act| + |b>act| + 1)`.
    pub fn and_measures_in(&self, act: &Label, threshold: f64) -> Matrix<f64> {
        let Some(node) = self.nodes.get(act) else {
            return Matrix::new();
        };
        self.and_measures(node.inputs.keys(), threshold, |n| {
            get(&self.dfg_matrix, n, act)
        })
    }

    fn and_measures<'a>(
        &self,
        neighbours: impl Iterator<Item = &'a Label>,
        threshold: f64,
        to_act: impl Fn(&Label) -> u64,
    ) -> Matrix<f64> {
        let ns: Vec<&Label> = neighbours.collect();
        let mut out = Matrix::new();
        for (i, &n1) in ns.iter().enumerate() {
            for &n2 in &ns[i + 1..] {
                let c1 = get(&self.dfg_matrix, n1, n2);
                let c2 = get(&self.dfg_matrix, n2, n1);
                let value = (c1 + c2) as f64 / (to_act(n1) + to_act(n2) + 1) as f64;
                if value >= threshold {
                    out.entry(n1.clone())
                        .or_insert_with(BTreeMap::new)
                        .insert(n2.clone(), value);
                }
            }
        }
        out
    }

    /// Activities that form a loop of length two with `act`, with the
    /// frequency `|act>b|` (pm4py's `Node.calculate_loops_length_two`).
    ///
    /// `b` qualifies when `(t + u) / (t + u + 1) >= threshold`, where `t`
    /// counts `act b act` and `u` counts `b act b` in
    /// [`freq_triples_matrix`](Self::freq_triples_matrix).
    pub fn loops_length_two(&self, act: &Label, threshold: f64) -> BTreeMap<Label, u64> {
        let Some(row) = self.freq_triples_matrix.get(act) else {
            return BTreeMap::new();
        };
        row.keys()
            .filter(|n2| {
                let v1 = get(&self.freq_triples_matrix, act, n2);
                let v2 = get(&self.freq_triples_matrix, n2, act);
                (v1 + v2) as f64 / (v1 + v2 + 1) as f64 >= threshold
            })
            .map(|n2| (n2.clone(), get(&self.dfg_matrix, act, n2)))
            .collect()
    }

    /// Computes the AND measures and length-two loops of every node, as
    /// pm4py's Heuristics Miner does once the connections are in place.
    pub fn calculate_node_measures(&mut self, and_threshold: f64, loop_threshold: f64) {
        let acts: Vec<Label> = self.nodes.keys().cloned().collect();
        for act in acts {
            let out = self.and_measures_out(&act, and_threshold);
            let inc = self.and_measures_in(&act, and_threshold);
            let loops = self.loops_length_two(&act, loop_threshold);
            let node = self.nodes.get_mut(&act).expect("node exists");
            node.and_measures_out = out;
            node.and_measures_in = inc;
            node.loop_length_two.extend(loops);
        }
    }

    /// Combines two heuristics nets (pm4py's `HeuristicsNet.__add__`).
    ///
    /// Nodes are united. Where both nets connect the same two nodes, the
    /// edge lists are concatenated. The start and end activity maps of
    /// `other` are appended, so the Petri net of the result has one source
    /// and one sink place per original net. Matrices and AND measures come
    /// from `self`, as in pm4py.
    pub fn merge(&self, other: &HeuristicsNet) -> HeuristicsNet {
        let mut out = self.clone();
        for (name, theirs) in &other.nodes {
            match out.nodes.get_mut(name) {
                Some(ours) => {
                    for (target, edges) in &theirs.outputs {
                        ours.outputs
                            .entry(target.clone())
                            .or_default()
                            .extend(edges.iter().copied());
                    }
                }
                None => {
                    out.nodes.insert(name.clone(), theirs.clone());
                }
            }
        }
        out.start_activities
            .extend(other.start_activities.iter().cloned());
        out.end_activities
            .extend(other.end_activities.iter().cloned());
        out
    }
}

#[cfg(test)]
mod tests;
