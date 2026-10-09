//! The two inputs the inductive miner recurses on: a variant log (pm4py's
//! UVCL) and a directly-follows graph over activity numbers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// An activity, numbered in sorted name order, so that sorting numbers sorts
/// names as pm4py's `sorted(alphabet)` does.
pub(crate) type Act = u32;

/// A set of activities.
pub(crate) type Group = BTreeSet<Act>;

/// pm4py's univariate variant-compressed log (UVCL): each distinct activity
/// sequence with its number of traces. Ordered, so iteration is
/// deterministic.
pub(crate) type Uvcl = BTreeMap<Vec<Act>, u64>;

/// Adds `count` traces with sequence `trace` (pm4py's `Counter.update`).
pub(crate) fn add_trace(log: &mut Uvcl, trace: Vec<Act>, count: u64) {
    *log.entry(trace).or_insert(0) += count;
}

/// The activities that occur in `log`, sorted (pm4py's `get_alphabet`).
pub(crate) fn alphabet(log: &Uvcl) -> Group {
    log.keys().flatten().copied().collect()
}

/// The number of traces in `log`.
pub(crate) fn trace_count(log: &Uvcl) -> u64 {
    log.values().sum()
}

/// A directly-follows graph with edge, start and end frequencies (pm4py's
/// `DFG` object).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Dfg {
    pub graph: BTreeMap<(Act, Act), u64>,
    pub start: BTreeMap<Act, u64>,
    pub end: BTreeMap<Act, u64>,
}

impl Dfg {
    /// The DFG of a variant log (pm4py's `discover_dfg_uvcl`).
    pub fn from_log(log: &Uvcl) -> Self {
        let mut dfg = Dfg::default();
        for (trace, &count) in log {
            for pair in trace.windows(2) {
                *dfg.graph.entry((pair[0], pair[1])).or_insert(0) += count;
            }
            if let (Some(&first), Some(&last)) = (trace.first(), trace.last()) {
                *dfg.start.entry(first).or_insert(0) += count;
                *dfg.end.entry(last).or_insert(0) += count;
            }
        }
        dfg
    }

    /// `true` if the DFG has no edges and no start or end activities.
    pub fn is_empty(&self) -> bool {
        self.graph.is_empty() && self.start.is_empty() && self.end.is_empty()
    }

    /// Edge ends plus start and end activities (pm4py's `get_vertices`).
    pub fn vertices(&self) -> Group {
        let mut v: Group = self.graph.keys().flat_map(|&(a, b)| [a, b]).collect();
        v.extend(self.start.keys());
        v.extend(self.end.keys());
        v
    }

    /// `true` if the edge `a -> b` exists.
    pub fn has_edge(&self, a: Act, b: Act) -> bool {
        self.graph.contains_key(&(a, b))
    }

    /// The part of the DFG inside `group`: edges with both ends in it and the
    /// start and end activities in it.
    pub fn restrict(&self, group: &Group) -> Dfg {
        Dfg {
            graph: self
                .graph
                .iter()
                .filter(|((a, b), _)| group.contains(a) && group.contains(b))
                .map(|(&e, &n)| (e, n))
                .collect(),
            start: restrict_map(&self.start, group),
            end: restrict_map(&self.end, group),
        }
    }

    /// For each vertex, the vertices that can reach it and the vertices it
    /// can reach, excluding itself (pm4py's `get_transitive_relations`, which
    /// uses networkx's `ancestors` and `descendants`).
    pub fn transitive_relations(&self) -> (BTreeMap<Act, Group>, BTreeMap<Act, Group>) {
        let vertices = self.vertices();
        let mut forward: BTreeMap<Act, Vec<Act>> = BTreeMap::new();
        let mut backward: BTreeMap<Act, Vec<Act>> = BTreeMap::new();
        for &(a, b) in self.graph.keys() {
            forward.entry(a).or_default().push(b);
            backward.entry(b).or_default().push(a);
        }
        let pre = vertices
            .iter()
            .map(|&a| (a, reachable(&backward, a)))
            .collect();
        let post = vertices
            .iter()
            .map(|&a| (a, reachable(&forward, a)))
            .collect();
        (pre, post)
    }
}

fn restrict_map(map: &BTreeMap<Act, u64>, group: &Group) -> BTreeMap<Act, u64> {
    map.iter()
        .filter(|(a, _)| group.contains(a))
        .map(|(&a, &n)| (a, n))
        .collect()
}

/// Vertices reachable from `from` along `adjacency`, without `from` itself.
fn reachable(adjacency: &BTreeMap<Act, Vec<Act>>, from: Act) -> Group {
    let mut seen = Group::from([from]);
    let mut queue = VecDeque::from([from]);
    while let Some(a) = queue.pop_front() {
        for &b in adjacency.get(&a).into_iter().flatten() {
            if seen.insert(b) {
                queue.push_back(b);
            }
        }
    }
    seen.remove(&from);
    seen
}

/// Connected components of the undirected graph on `vertices` with `edges`,
/// ordered by their smallest activity.
pub(crate) fn connected_components(
    vertices: &Group,
    edges: impl IntoIterator<Item = (Act, Act)>,
) -> Vec<Group> {
    let mut adjacency: BTreeMap<Act, Vec<Act>> = BTreeMap::new();
    for (a, b) in edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    let mut seen = Group::new();
    let mut components = Vec::new();
    for &v in vertices {
        if seen.contains(&v) {
            continue;
        }
        let mut component = reachable(&adjacency, v);
        component.insert(v);
        component.retain(|a| vertices.contains(a));
        seen.extend(&component);
        components.push(component);
    }
    components
}
