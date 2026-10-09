//! DFG filters, ported from pm4py's `algo/filtering/dfg/dfg_filtering.py`.
//!
//! Each filter takes the DFG and its activity counts and returns filtered
//! copies of both.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use super::{ActivityCounts, Dfg, DfgError};
use crate::Label;

/// A node of the adjacency structure: an activity or one of the two
/// artificial nodes that connect to the start and end activities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Node {
    Start,
    End,
    Act(Label),
}

impl Node {
    /// The string pm4py uses for this node when it sorts edges.
    fn sort_key(&self) -> &str {
        match self {
            Node::Start => "_S_START_",
            Node::End => "_S_END_",
            Node::Act(l) => l.as_str(),
        }
    }
}

type Adj = HashMap<Node, BTreeSet<NodeKey>>;

/// Adjacency sets hold nodes in a fixed order for determinism.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum NodeKey {
    Start,
    End,
    Act(Label),
}

impl From<&Node> for NodeKey {
    fn from(n: &Node) -> Self {
        match n {
            Node::Start => NodeKey::Start,
            Node::End => NodeKey::End,
            Node::Act(l) => NodeKey::Act(l.clone()),
        }
    }
}

impl From<&NodeKey> for Node {
    fn from(n: &NodeKey) -> Self {
        match n {
            NodeKey::Start => Node::Start,
            NodeKey::End => Node::End,
            NodeKey::Act(l) => Node::Act(l.clone()),
        }
    }
}

struct Graph {
    adj: Adj,
    rev: Adj,
}

impl Graph {
    /// pm4py's `build_adjacency_structures`.
    fn new(dfg: &Dfg) -> Self {
        let mut adj: Adj = HashMap::new();
        let mut rev: Adj = HashMap::new();
        for v in dfg
            .graph
            .keys()
            .flat_map(|(a, b)| [a, b])
            .chain(dfg.start_activities.keys())
            .chain(dfg.end_activities.keys())
        {
            adj.entry(Node::Act(v.clone())).or_default();
            rev.entry(Node::Act(v.clone())).or_default();
        }
        for (a, b) in dfg.graph.keys() {
            adj.get_mut(&Node::Act(a.clone()))
                .expect("added above")
                .insert(NodeKey::Act(b.clone()));
            rev.get_mut(&Node::Act(b.clone()))
                .expect("added above")
                .insert(NodeKey::Act(a.clone()));
        }
        adj.insert(
            Node::Start,
            dfg.start_activities
                .keys()
                .cloned()
                .map(NodeKey::Act)
                .collect(),
        );
        rev.insert(Node::Start, BTreeSet::new());
        for s in dfg.start_activities.keys() {
            rev.get_mut(&Node::Act(s.clone()))
                .expect("added above")
                .insert(NodeKey::Start);
        }
        adj.insert(Node::End, BTreeSet::new());
        rev.insert(
            Node::End,
            dfg.end_activities
                .keys()
                .cloned()
                .map(NodeKey::Act)
                .collect(),
        );
        for e in dfg.end_activities.keys() {
            adj.get_mut(&Node::Act(e.clone()))
                .expect("added above")
                .insert(NodeKey::End);
        }
        Graph { adj, rev }
    }

    fn reachable(adj: &Adj, start: Node) -> BTreeSet<NodeKey> {
        let mut seen = BTreeSet::from([NodeKey::from(&start)]);
        let mut queue = VecDeque::from([start]);
        while let Some(u) = queue.pop_front() {
            for v in adj.get(&u).into_iter().flatten() {
                if seen.insert(v.clone()) {
                    queue.push_back(Node::from(v));
                }
            }
        }
        seen
    }

    /// Returns `true` if every activity in `keep` is reachable from the
    /// start node and reaches the end node.
    fn connects(&self, keep: &BTreeSet<Label>) -> bool {
        let from_start = Self::reachable(&self.adj, Node::Start);
        let to_end = Self::reachable(&self.rev, Node::End);
        keep.iter().all(|a| {
            let k = NodeKey::Act(a.clone());
            from_start.contains(&k) && to_end.contains(&k)
        })
    }

    /// pm4py's `remove_unreachable_nodes`.
    fn remove_unreachable(&self, dfg: &mut Dfg, counts: &mut ActivityCounts) {
        let from_start = Self::reachable(&self.adj, Node::Start);
        let to_end = Self::reachable(&self.rev, Node::End);
        let keep = |a: &Label| {
            let k = NodeKey::Act(a.clone());
            from_start.contains(&k) && to_end.contains(&k)
        };
        let removed: Vec<Label> = counts.keys().filter(|a| !keep(a)).cloned().collect();
        for a in &removed {
            counts.remove(a);
            dfg.start_activities.remove(a);
            dfg.end_activities.remove(a);
        }
        dfg.graph
            .retain(|(a, b), _| counts.contains_key(a) && counts.contains_key(b));
    }

    fn remove_edge(&mut self, u: &Node, v: &Node) -> bool {
        let vk = NodeKey::from(v);
        let removed = self.adj.get_mut(u).is_some_and(|succ| succ.remove(&vk));
        if removed && let Some(pred) = self.rev.get_mut(v) {
            pred.remove(&NodeKey::from(u));
        }
        removed
    }

    fn add_edge(&mut self, u: &Node, v: &Node) {
        self.adj
            .entry(u.clone())
            .or_default()
            .insert(NodeKey::from(v));
        self.rev
            .entry(v.clone())
            .or_default()
            .insert(NodeKey::from(u));
    }
}

/// An edge of the DFG, including the artificial start and end edges, with
/// the score used to rank it.
struct RankedEdge {
    from: Node,
    to: Node,
    score: f64,
}

/// Sorts edges by score, then by `(from, to)` names, both descending, as
/// pm4py's `sort(key=lambda x: (x[1], x[0]), reverse=True)`.
fn sort_desc(edges: &mut [RankedEdge]) {
    edges.sort_by(|x, y| {
        y.score
            .partial_cmp(&x.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                (y.from.sort_key(), y.to.sort_key()).cmp(&(x.from.sort_key(), x.to.sort_key()))
            })
    });
}

fn removable_edges_filter(
    dfg: &Dfg,
    counts: &ActivityCounts,
    mut edges: Vec<RankedEdge>,
    split: impl Fn(&[RankedEdge]) -> usize,
    keep_all_activities: bool,
) -> (Dfg, ActivityCounts) {
    let mut dfg = dfg.clone();
    let mut counts = counts.clone();
    let mut graph = Graph::new(&dfg);
    sort_desc(&mut edges);
    let cut = split(&edges);
    let (kept, discardable) = edges.split_at(cut);
    let keep: BTreeSet<Label> = if keep_all_activities {
        counts
            .keys()
            .chain(dfg.start_activities.keys())
            .chain(dfg.end_activities.keys())
            .cloned()
            .collect()
    } else {
        kept.iter()
            .flat_map(|e| [&e.from, &e.to])
            .filter_map(|n| match n {
                Node::Act(l) => Some(l.clone()),
                _ => None,
            })
            .collect()
    };
    // pm4py's `__filter_specified_paths_adjacency`, least important first.
    for e in discardable.iter().rev() {
        if !graph.remove_edge(&e.from, &e.to) {
            continue;
        }
        if graph.connects(&keep) {
            match (&e.from, &e.to) {
                (Node::Act(a), Node::Act(b)) => {
                    dfg.graph.remove(&(a.clone(), b.clone()));
                }
                (Node::Start, Node::Act(b)) => {
                    dfg.start_activities.remove(b);
                }
                (Node::Act(a), Node::End) => {
                    dfg.end_activities.remove(a);
                }
                _ => {}
            }
        } else {
            graph.add_edge(&e.from, &e.to);
        }
    }
    graph.remove_unreachable(&mut dfg, &mut counts);
    (dfg, counts)
}

fn all_edges(
    dfg: &Dfg,
    score: impl Fn(&Label, &Label, u64) -> f64,
    boundary: impl Fn(u64) -> f64,
) -> Vec<RankedEdge> {
    let mut edges: Vec<RankedEdge> = dfg
        .graph
        .iter()
        .map(|((a, b), &n)| RankedEdge {
            from: Node::Act(a.clone()),
            to: Node::Act(b.clone()),
            score: score(a, b, n),
        })
        .collect();
    edges.extend(dfg.start_activities.iter().map(|(s, &n)| RankedEdge {
        from: Node::Start,
        to: Node::Act(s.clone()),
        score: boundary(n),
    }));
    edges.extend(dfg.end_activities.iter().map(|(e, &n)| RankedEdge {
        from: Node::Act(e.clone()),
        to: Node::End,
        score: boundary(n),
    }));
    edges
}

/// pm4py's `math.ceil((n - 1) * percentage) + 1`, clamped to `n`.
fn cut_index(n: usize, percentage: f64) -> usize {
    let cut = ((n.saturating_sub(1)) as f64 * percentage).ceil() as usize + 1;
    cut.min(n)
}

impl Dfg {
    /// Keeps the most frequent `percentage` (0 to 1) of the activities, plus
    /// any activity needed to keep every kept activity on a path from start
    /// to end (pm4py's `filter_dfg_on_activities_percentage`).
    pub fn filter_activities_percentage(
        &self,
        counts: &ActivityCounts,
        percentage: f64,
    ) -> (Dfg, ActivityCounts) {
        let mut dfg = self.clone();
        let mut counts = counts.clone();
        if counts.len() <= 1 || dfg.graph.len() <= 1 {
            return (dfg, counts);
        }
        let mut sorted: Vec<(&Label, u64)> = counts.iter().map(|(a, &n)| (a, n)).collect();
        sorted.sort_by(|x, y| (y.1, y.0).cmp(&(x.1, x.0)));
        let cut = cut_index(sorted.len(), percentage);
        let keep: BTreeSet<Label> = sorted[..cut].iter().map(|(a, _)| (*a).clone()).collect();
        let candidates: Vec<Label> = sorted[cut..]
            .iter()
            .rev()
            .map(|(a, _)| (*a).clone())
            .collect();
        let mut graph = Graph::new(&dfg);
        for act in candidates {
            let node = Node::Act(act.clone());
            let saved_succ = graph.adj.get(&node).cloned().unwrap_or_default();
            let saved_pred = graph.rev.get(&node).cloned().unwrap_or_default();
            if graph.adj.contains_key(&node) {
                for p in &saved_pred {
                    if let Some(s) = graph.adj.get_mut(&Node::from(p)) {
                        s.remove(&NodeKey::Act(act.clone()));
                    }
                }
                for s in &saved_succ {
                    if let Some(p) = graph.rev.get_mut(&Node::from(s)) {
                        p.remove(&NodeKey::Act(act.clone()));
                    }
                }
                graph.adj.remove(&node);
                graph.rev.remove(&node);
            }
            if graph.connects(&keep) {
                dfg.graph.retain(|(a, b), _| *a != act && *b != act);
                counts.remove(&act);
                dfg.start_activities.remove(&act);
                dfg.end_activities.remove(&act);
            } else {
                for p in &saved_pred {
                    graph
                        .adj
                        .entry(Node::from(p))
                        .or_default()
                        .insert(NodeKey::Act(act.clone()));
                }
                for s in &saved_succ {
                    graph
                        .rev
                        .entry(Node::from(s))
                        .or_default()
                        .insert(NodeKey::Act(act.clone()));
                }
                graph.adj.insert(node.clone(), saved_succ);
                graph.rev.insert(node, saved_pred);
            }
        }
        graph.remove_unreachable(&mut dfg, &mut counts);
        (dfg, counts)
    }

    /// Keeps the most frequent `percentage` (0 to 1) of the edges, counting
    /// start and end as edges, plus any edge needed to keep the activities
    /// connected from start to end (pm4py's `filter_dfg_on_paths_percentage`).
    ///
    /// With `keep_all_activities`, every activity stays connected; otherwise
    /// only the activities on the kept edges.
    pub fn filter_paths_percentage(
        &self,
        counts: &ActivityCounts,
        percentage: f64,
        keep_all_activities: bool,
    ) -> (Dfg, ActivityCounts) {
        if counts.len() <= 1 || self.graph.len() <= 1 {
            return (self.clone(), counts.clone());
        }
        let edges = all_edges(self, |_, _, n| n as f64, |n| n as f64);
        removable_edges_filter(
            self,
            counts,
            edges,
            |e| cut_index(e.len(), percentage),
            keep_all_activities,
        )
    }

    /// Removes edges whose Heuristics Miner dependency is below `threshold`,
    /// unless the removal disconnects an activity that must stay (pm4py's
    /// `filter_dfg_keep_connected`).
    pub fn filter_keep_connected(
        &self,
        counts: &ActivityCounts,
        threshold: f64,
        keep_all_activities: bool,
    ) -> (Dfg, ActivityCounts) {
        if counts.len() <= 1 || self.graph.len() <= 1 {
            return (self.clone(), counts.clone());
        }
        let edges = all_edges(
            self,
            |a, b, ab| {
                let ab = ab as f64;
                match self.graph.get(&(b.clone(), a.clone())) {
                    Some(&ba) => (ab - ba as f64) / (ab + ba as f64 + 1.0),
                    None => ab / (ab + 1.0),
                }
            },
            |_| 1.0,
        );
        removable_edges_filter(
            self,
            counts,
            edges,
            |e| e.iter().take_while(|x| x.score >= threshold).count(),
            keep_all_activities,
        )
    }

    /// Makes `target` the only end activity and keeps only activities on a
    /// path from a start activity to it (pm4py's `filter_dfg_to_activity`).
    pub fn filter_to_activity(
        &self,
        counts: &ActivityCounts,
        target: &str,
    ) -> Result<(Dfg, ActivityCounts), DfgError> {
        let (target, &n) = counts
            .get_key_value(target)
            .ok_or_else(|| DfgError::UnknownActivity(target.into()))?;
        let mut dfg = self.clone();
        let mut counts = counts.clone();
        dfg.graph.retain(|(a, _), _| a != target);
        dfg.end_activities = BTreeMap::from([(target.clone(), n)]);
        loop {
            let succ = successors_within(&dfg, &counts);
            let pred = predecessors_within(&dfg, &counts);
            let mut from_start = BTreeSet::new();
            for s in dfg.start_activities.keys() {
                from_start.extend(succ.get(s).into_iter().flatten().cloned());
                from_start.insert(s.clone());
            }
            let mut reachable: BTreeSet<Label> = from_start
                .intersection(pred.get(target).unwrap_or(&BTreeSet::new()))
                .cloned()
                .collect();
            reachable.insert(target.clone());
            if !restrict(&mut dfg, &mut counts, &reachable, true, false) {
                return Ok((dfg, counts));
            }
        }
    }

    /// Makes `source` the only start activity and keeps only activities on a
    /// path from it to an end activity (pm4py's `filter_dfg_from_activity`).
    pub fn filter_from_activity(
        &self,
        counts: &ActivityCounts,
        source: &str,
    ) -> Result<(Dfg, ActivityCounts), DfgError> {
        let (source, &n) = counts
            .get_key_value(source)
            .ok_or_else(|| DfgError::UnknownActivity(source.into()))?;
        let mut dfg = self.clone();
        let mut counts = counts.clone();
        dfg.graph.retain(|(_, b), _| b != source);
        dfg.start_activities = BTreeMap::from([(source.clone(), n)]);
        loop {
            let succ = successors_within(&dfg, &counts);
            let pred = predecessors_within(&dfg, &counts);
            let mut to_end = BTreeSet::new();
            for e in dfg.end_activities.keys() {
                to_end.extend(pred.get(e).into_iter().flatten().cloned());
                to_end.insert(e.clone());
            }
            let mut reachable: BTreeSet<Label> = to_end
                .intersection(succ.get(source).unwrap_or(&BTreeSet::new()))
                .cloned()
                .collect();
            reachable.insert(source.clone());
            if !restrict(&mut dfg, &mut counts, &reachable, false, true) {
                return Ok((dfg, counts));
            }
        }
    }

    /// Keeps only activities that reach or are reached from `activity`
    /// (pm4py's `filter_dfg_contain_activity`).
    pub fn filter_contain_activity(
        &self,
        counts: &ActivityCounts,
        activity: &str,
    ) -> Result<(Dfg, ActivityCounts), DfgError> {
        let activity = counts
            .get_key_value(activity)
            .ok_or_else(|| DfgError::UnknownActivity(activity.into()))?
            .0
            .clone();
        let mut dfg = self.clone();
        let mut counts = counts.clone();
        loop {
            let succ = successors_within(&dfg, &counts);
            let pred = predecessors_within(&dfg, &counts);
            let mut before = pred.get(&activity).cloned().unwrap_or_default();
            before.insert(activity.clone());
            let mut after = succ.get(&activity).cloned().unwrap_or_default();
            after.insert(activity.clone());
            let starts = dfg.start_activities.len();
            let ends = dfg.end_activities.len();
            dfg.start_activities.retain(|a, _| before.contains(a));
            dfg.end_activities.retain(|a, _| after.contains(a));
            let mut changed =
                starts != dfg.start_activities.len() || ends != dfg.end_activities.len();
            let reachable: BTreeSet<Label> = before.union(&after).cloned().collect();
            changed |= restrict(&mut dfg, &mut counts, &reachable, false, false);
            if !changed {
                return Ok((dfg, counts));
            }
        }
    }

    /// Drops edges whose frequency is below `noise_threshold` times the
    /// largest edge frequency of either end, except edges listed in `keep`
    /// (pm4py's `clean_dfg_based_on_noise_thresh`).
    pub fn clean_noise(&self, noise_threshold: f64, keep: &BTreeSet<(Label, Label)>) -> Dfg {
        let mut max: BTreeMap<&Label, u64> = BTreeMap::new();
        for ((a, b), &n) in &self.graph {
            for x in [a, b] {
                let m = max.entry(x).or_insert(0);
                *m = (*m).max(n);
            }
        }
        let mut out = self.clone();
        out.graph.retain(|(a, b), n| {
            let limit = (max[a] as f64 * noise_threshold).min(max[b] as f64 * noise_threshold);
            keep.contains(&(a.clone(), b.clone())) || (*n as f64) >= limit
        });
        out
    }
}

/// Successors computed over the activities in `counts` (pm4py passes
/// `activities_count` as the activity universe).
fn successors_within(dfg: &Dfg, counts: &ActivityCounts) -> BTreeMap<Label, BTreeSet<Label>> {
    let mut g = Dfg::new();
    g.graph = dfg.graph.clone();
    let mut s = g.successors();
    s.retain(|k, _| counts.contains_key(k));
    s
}

fn predecessors_within(dfg: &Dfg, counts: &ActivityCounts) -> BTreeMap<Label, BTreeSet<Label>> {
    let mut g = Dfg::new();
    g.graph = dfg.graph.clone();
    let mut p = g.predecessors();
    p.retain(|k, _| counts.contains_key(k));
    p
}

/// Restricts counts, edges and (optionally) start or end activities to
/// `keep`. Returns `true` if the activity set changed.
///
/// pm4py compares `keep` with the activity set directly and loops for ever
/// when `keep` holds an activity missing from the counts; this compares
/// only the activities that are counted.
fn restrict(
    dfg: &mut Dfg,
    counts: &mut ActivityCounts,
    keep: &BTreeSet<Label>,
    starts: bool,
    ends: bool,
) -> bool {
    if counts.keys().all(|a| keep.contains(a)) {
        return false;
    }
    counts.retain(|a, _| keep.contains(a));
    if starts {
        dfg.start_activities.retain(|a, _| keep.contains(a));
    }
    if ends {
        dfg.end_activities.retain(|a, _| keep.contains(a));
    }
    dfg.graph
        .retain(|(a, b), _| keep.contains(a) && keep.contains(b));
    true
}
