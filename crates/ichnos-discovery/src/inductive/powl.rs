//! POWL discovery, ported from pm4py's `algo/discovery/powl/`.
//!
//! The recursion is the inductive miner's, with three changes: cuts and
//! fall-throughs that IM turns into sequence and parallel nodes give partial
//! orders, three variants add a partial-order cut after the IM cuts, and an
//! optional filter drops infrequent variants when no cut holds.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;

use ichnos_model::powl::{BinaryRelation, StrictPartialOrder};
use ichnos_model::{Label, Operator, Powl};

use super::cuts::{Cut, find_cut, loop_cut, project_log, xor_cut};
use super::data::{Act, Dfg, Uvcl, add_trace, alphabet};
use super::fall_through::{Split, fall_through_except_flower};

/// Which POWL miner to run (pm4py's `POWLDiscoveryVariant`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowlVariant {
    /// The inductive miner's cuts only, so every partial order is a sequence
    /// or a concurrency (pm4py's `TREE`).
    Tree,
    /// The inductive miner's cuts, then the first partition of the
    /// activities, from most groups to fewest, whose eventually-follows order
    /// is valid (pm4py's `BRUTE_FORCE`). The search is exponential in the
    /// number of activities.
    BruteForce,
    /// The inductive miner's cuts, then the maximal partial order the
    /// eventually-follows relation gives, with activities that share their
    /// predecessors and successors grouped (pm4py's `MAXIMAL`, the default).
    Maximal,
    /// The choice and loop cuts, then a partial order over clusters of
    /// activities, merged until the eventually-follows frequencies order them
    /// (pm4py's `DYNAMIC_CLUSTERING`, which runs its frequency miner).
    DynamicClustering {
        /// A cluster comes before another when at least this fraction of
        /// the traces that hold both put it first. In `(0.5, 1]`; pm4py's
        /// `order_graph_filtering_threshold`, default 1.
        order_frequency_ratio: f64,
    },
}

/// Options for POWL discovery.
///
/// The default runs the maximal variant without filtering, as pm4py's
/// `discover_powl` does with its defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowlOptions {
    /// The variant to run.
    pub variant: PowlVariant,
    /// pm4py's `filtering_weight_factor`, in `[0, 1)`. When no cut holds and
    /// the factor is above 0, the miner keeps the most frequent variants,
    /// each more frequent than this factor times the one before it, and
    /// mines them instead, if that drops some variants but not all.
    pub filtering_weight_factor: f64,
}

impl Default for PowlOptions {
    fn default() -> Self {
        Self::new(PowlVariant::Maximal)
    }
}

impl PowlOptions {
    /// Options for `variant`, without filtering.
    pub fn new(variant: PowlVariant) -> Self {
        Self {
            variant,
            filtering_weight_factor: 0.0,
        }
    }

    /// Sets [`PowlOptions::filtering_weight_factor`].
    pub fn with_filtering_weight_factor(mut self, factor: f64) -> Self {
        self.filtering_weight_factor = factor;
        self
    }

    /// The first option outside its range, as an error message.
    pub(crate) fn invalid(&self) -> Option<&'static str> {
        if !(0.0..1.0).contains(&self.filtering_weight_factor) {
            return Some("filtering_weight_factor must be in [0, 1)");
        }
        match self.variant {
            PowlVariant::DynamicClustering {
                order_frequency_ratio: r,
            } if !(r > 0.5 && r <= 1.0) => Some("order_frequency_ratio must be in (0.5, 1]"),
            _ => None,
        }
    }
}

/// Activity groups in child order and the order between them, by group
/// position.
type Order = (Vec<Vec<Act>>, BinaryRelation);

/// What a cut or fall-through builds from the models of its sublogs.
enum Shape {
    Xor,
    Loop,
    Order(BinaryRelation),
}

impl Shape {
    /// The POWL counterpart of an inductive-miner operator over `n` children.
    fn of(operator: Operator, n: usize) -> Shape {
        match operator {
            Operator::Xor => Shape::Xor,
            Operator::Loop => Shape::Loop,
            Operator::Parallel => Shape::Order(BinaryRelation::new(n)),
            Operator::Sequence => {
                let mut order = BinaryRelation::new(n);
                for i in 0..n {
                    for j in i + 1..n {
                        order.add_edge(i, j);
                    }
                }
                Shape::Order(order)
            }
            Operator::Or | Operator::Interleaving => {
                unreachable!("the inductive miner builds no {operator} nodes")
            }
        }
    }
}

/// One run of the POWL miner: the activity names and the options.
#[derive(Debug)]
pub(crate) struct PowlMiner<'a> {
    /// Activity names, indexed by [`Act`].
    pub labels: &'a [Label],
    pub options: PowlOptions,
}

impl PowlMiner<'_> {
    /// pm4py's `IMBasePOWL.apply`.
    pub fn mine(&self, log: Uvcl) -> Powl {
        if log.contains_key(&[][..]) {
            let mut rest = log;
            rest.remove(&[][..]);
            return Powl::xor([Powl::Silent, self.mine(rest)]);
        }
        let mut variants = log.keys();
        match (variants.next(), variants.next()) {
            (None, _) => return Powl::Silent,
            (Some(t), None) if t.len() == 1 => {
                return Powl::Activity(self.labels[t[0] as usize].clone());
            }
            _ => {}
        }
        if let Some((shape, parts)) = self.find_cut(&log) {
            return self.build(shape, parts);
        }
        let factor = self.options.filtering_weight_factor;
        if factor > 0.0 {
            let filtered = most_frequent_variants(&log, factor);
            if !filtered.is_empty() && filtered.len() < log.len() {
                return self.mine(filtered);
            }
        }
        match fall_through_except_flower(&log, true, true) {
            Some(Split::Node(op, parts)) => self.build(Shape::of(op, parts.len()), parts),
            Some(Split::Tau) => Powl::Silent,
            // pm4py's `POWLFlowerModelUVCL`: each activity once as the do
            // part, nothing as the redo part.
            None => {
                let each = alphabet(&log).into_iter().map(|a| (vec![a], 1)).collect();
                self.build(Shape::Loop, vec![each, Uvcl::new()])
            }
        }
    }

    /// The cut of the variant, with the sublogs of its children.
    fn find_cut(&self, log: &Uvcl) -> Option<(Shape, Vec<Uvcl>)> {
        let dfg = Dfg::from_log(log);
        let im_cut = |cut: Cut| {
            let parts = project_log(log, &cut);
            (Shape::of(cut.operator, cut.groups.len()), parts)
        };
        let order_cut = |(groups, order): Order| {
            let parts = groups.iter().map(|g| project_groups(log, g)).collect();
            (Shape::Order(order), parts)
        };
        match self.options.variant {
            PowlVariant::Tree => find_cut(&dfg, true).map(im_cut),
            PowlVariant::BruteForce => find_cut(&dfg, true)
                .map(im_cut)
                .or_else(|| brute_force_order(&dfg, &eventually_follows(log)).map(order_cut)),
            PowlVariant::Maximal => {
                if dfg.vertices().len() < 2 {
                    return None;
                }
                find_cut(&dfg, true)
                    .map(im_cut)
                    .or_else(|| maximal_order(&dfg, &eventually_follows(log)).map(order_cut))
            }
            PowlVariant::DynamicClustering {
                order_frequency_ratio,
            } => {
                if dfg.vertices().len() < 2 {
                    return None;
                }
                let cut = |operator, groups| Cut { operator, groups };
                xor_cut(&dfg)
                    .map(|g| im_cut(cut(Operator::Xor, g)))
                    .or_else(|| loop_cut(&dfg).map(|g| im_cut(cut(Operator::Loop, g))))
                    .or_else(|| clustered_order(log, &dfg, order_frequency_ratio).map(order_cut))
            }
        }
    }

    /// Mines each sublog and joins the models (pm4py's `_recurse`).
    fn build(&self, shape: Shape, parts: Vec<Uvcl>) -> Powl {
        let children: Vec<Powl> = parts.into_iter().map(|l| self.mine(l)).collect();
        match shape {
            Shape::Xor => Powl::Xor(children),
            Shape::Loop => {
                let [d, r] = <[Powl; 2]>::try_from(children)
                    .unwrap_or_else(|c| panic!("a loop has two parts, not {}", c.len()));
                Powl::looped(d, r)
            }
            Shape::Order(order) => {
                let mut po = StrictPartialOrder::new(children);
                for (i, j) in order.edges() {
                    po.add_edge(i, j);
                }
                po.into()
            }
        }
    }
}

/// Each trace keeps only the events of `group` (pm4py's
/// `project_on_groups_with_unique_activities`).
///
/// pm4py's brute-force cut assigns each projected variant the count of the
/// last variant that projects to it, where this adds the counts. Counts only
/// matter to the variant filter.
fn project_groups(log: &Uvcl, group: &[Act]) -> Uvcl {
    let mut sublog = Uvcl::new();
    for (trace, &count) in log {
        let projected = trace
            .iter()
            .copied()
            .filter(|a| group.contains(a))
            .collect();
        add_trace(&mut sublog, projected, count);
    }
    sublog
}

/// The most frequent variants, each more frequent than `factor` times the
/// one kept before it (pm4py's
/// `filter_most_frequent_variants_with_decreasing_factor`).
fn most_frequent_variants(log: &Uvcl, factor: f64) -> Uvcl {
    let mut variants: Vec<(&Vec<Act>, u64)> = log.iter().map(|(t, &n)| (t, n)).collect();
    variants.sort_by_key(|&(_, n)| Reverse(n));
    let mut kept = Uvcl::new();
    let mut previous = 0;
    for (trace, n) in variants {
        if !kept.is_empty() && n as f64 <= factor * previous as f64 {
            break;
        }
        kept.insert(trace.clone(), n);
        previous = n;
    }
    kept
}

/// The pairs `(a, b)` where `a` occurs before `b` in some trace (pm4py's
/// eventually-follows graph of a UVCL, without its counts).
fn eventually_follows(log: &Uvcl) -> BTreeSet<(Act, Act)> {
    let mut efg = BTreeSet::new();
    for trace in log.keys() {
        for (i, &a) in trace.iter().enumerate() {
            efg.extend(trace[i + 1..].iter().map(|&b| (a, b)));
        }
    }
    efg
}

/// Whether every activity of `g` eventually precedes every activity of `h`.
fn all_follow(efg: &BTreeSet<(Act, Act)>, g: &[Act], h: &[Act]) -> bool {
    g.iter().all(|&a| h.iter().all(|&b| efg.contains(&(a, b))))
}

/// pm4py's `is_valid_order` of the maximal and brute-force cuts. The order
/// must be a strict partial order; two groups must be ordered unless each
/// eventually follows the other completely, and then they must not be; every
/// group without a predecessor must hold a start activity, and every group
/// without a successor an end activity.
fn is_valid_order(
    groups: &[Vec<Act>],
    order: &BinaryRelation,
    efg: &BTreeSet<(Act, Act)>,
    dfg: &Dfg,
) -> bool {
    if !order.is_strict_partial_order() {
        return false;
    }
    let n = groups.len();
    for i in 0..n {
        for j in i + 1..n {
            let ordered = order.is_edge(i, j) || order.is_edge(j, i);
            let interleaved =
                all_follow(efg, &groups[i], &groups[j]) && all_follow(efg, &groups[j], &groups[i]);
            if ordered == interleaved {
                return false;
            }
        }
    }
    let starts: BTreeSet<usize> = order.start_nodes().into_iter().collect();
    let ends: BTreeSet<usize> = order.end_nodes().into_iter().collect();
    (0..n).all(|i| {
        let holds = |m: &BTreeMap<Act, u64>| groups[i].iter().any(|a| m.contains_key(a));
        (!starts.contains(&i) || holds(&dfg.start)) && (!ends.contains(&i) || holds(&dfg.end))
    })
}

/// pm4py's `MaximalPartialOrderCut.holds`. Orders two activities when one
/// eventually follows the other but not the reverse, groups activities with
/// the same predecessors and successors, and keeps the result if it is a
/// valid order of at least two groups.
fn maximal_order(dfg: &Dfg, efg: &BTreeSet<(Act, Act)>) -> Option<Order> {
    let nodes: Vec<Act> = dfg.vertices().into_iter().collect();
    let n = nodes.len();
    let mut initial = BinaryRelation::new(n);
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (nodes[i], nodes[j]);
            match (efg.contains(&(a, b)), efg.contains(&(b, a))) {
                (true, false) => initial.add_edge(i, j),
                (false, true) => initial.add_edge(j, i),
                _ => {}
            }
        }
    }
    let pre = |j: usize| {
        (0..n)
            .filter(|&i| initial.is_edge(i, j))
            .collect::<Vec<_>>()
    };
    let post = |i: usize| {
        (0..n)
            .filter(|&j| initial.is_edge(i, j))
            .collect::<Vec<_>>()
    };
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for v in 0..n {
        match clusters
            .iter_mut()
            .find(|c| pre(c[0]) == pre(v) && post(c[0]) == post(v))
        {
            Some(c) => c.push(v),
            None => clusters.push(vec![v]),
        }
    }
    let mut order = BinaryRelation::new(clusters.len());
    for (x, cx) in clusters.iter().enumerate() {
        for (y, cy) in clusters.iter().enumerate() {
            if initial.is_edge(cx[0], cy[0]) {
                order.add_edge(x, y);
            }
        }
    }
    let groups: Vec<Vec<Act>> = clusters
        .iter()
        .map(|c| c.iter().map(|&i| nodes[i]).collect())
        .collect();
    (groups.len() >= 2 && is_valid_order(&groups, &order, efg, dfg)).then_some((groups, order))
}

/// pm4py's `BruteForcePartialOrderCut.holds`: the first partition of the
/// sorted activities, from most groups to two, whose order is valid. Two
/// groups are ordered when every pair between them eventually follows one
/// way and none the other way.
fn brute_force_order(dfg: &Dfg, efg: &BTreeSet<(Act, Act)>) -> Option<Order> {
    let activities: Vec<Act> = dfg.vertices().into_iter().collect();
    let mut found = None;
    for k in (2..=activities.len()).rev() {
        let mut test = |groups: Vec<Vec<Act>>| {
            let n = groups.len();
            let mut order = BinaryRelation::new(n);
            for i in 0..n {
                for j in i + 1..n {
                    let none = |g: &[Act], h: &[Act]| {
                        g.iter().all(|&a| h.iter().all(|&b| !efg.contains(&(a, b))))
                    };
                    let (g, h) = (&groups[i], &groups[j]);
                    if all_follow(efg, g, h) && none(h, g) {
                        order.add_edge(i, j);
                    } else if none(g, h) && all_follow(efg, h, g) {
                        order.add_edge(j, i);
                    }
                }
            }
            if is_valid_order(&groups, &order, efg, dfg) {
                found = Some((groups, order));
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        if partitions(&activities, k, &mut test).is_break() {
            break;
        }
    }
    found
}

/// Calls `f` on each partition of `l` into `k` groups, in the order of
/// pm4py's `get_partitions_of_size_k`, until `f` breaks. Needs
/// `1 <= k <= l.len()`.
fn partitions(
    l: &[Act],
    k: usize,
    f: &mut dyn FnMut(Vec<Vec<Act>>) -> ControlFlow<()>,
) -> ControlFlow<()> {
    if k == 1 {
        return f(vec![l.to_vec()]);
    }
    if l.len() == k {
        return f(l.iter().map(|&a| vec![a]).collect());
    }
    let (e, rest) = (l[0], &l[1..]);
    partitions(rest, k - 1, &mut |p| {
        let mut q = vec![vec![e]];
        q.extend(p);
        f(q)
    })?;
    partitions(rest, k, &mut |p| {
        for i in 0..p.len() {
            let mut q = p.clone();
            q[i].insert(0, e);
            f(q)?;
        }
        ControlFlow::Continue(())
    })
}

/// pm4py's `DynamicClusteringFrequencyPartialOrderCut.holds` and
/// `generate_order`, with its recursion as a loop.
///
/// Starts with one cluster per activity. Orders two clusters when at least
/// `ratio` of the traces that hold both put the first one first. Then
/// merges clusters, starting over after each round of merges, until:
/// closing the order under transitivity relates no two clusters that occur
/// in a common trace without an order, no two clusters are ordered both
/// ways, no two unordered clusters both stay unordered and never occur
/// together, and no two clusters share their predecessors and successors.
/// One cluster left means no cut.
///
/// When closing the order relates a cluster to itself, pm4py merges the
/// cluster with itself and recurses on the same clusters without end; this
/// gives no cut.
fn clustered_order(log: &Uvcl, dfg: &Dfg, ratio: f64) -> Option<Order> {
    let mut clusters: Vec<Vec<Act>> = dfg.vertices().into_iter().map(|a| vec![a]).collect();
    'restart: loop {
        if clusters.len() < 2 {
            return None;
        }
        let mut nodes = clusters.clone();
        nodes.sort();
        let n = nodes.len();
        let freq = cluster_frequencies(log, &nodes);
        let f = |a: usize, b: usize| freq[a][b];
        let mut order = BinaryRelation::new(n);
        for i in 0..n {
            for j in i + 1..n {
                let sum = (f(i, j) + f(j, i)) as f64;
                if sum > 0.0 {
                    if f(i, j) as f64 / sum >= ratio {
                        order.add_edge(i, j);
                    }
                    if f(j, i) as f64 / sum >= ratio {
                        order.add_edge(j, i);
                    }
                }
            }
        }
        if !order.is_transitive() {
            let mut changed = true;
            while changed {
                changed = false;
                for i in 0..n {
                    for j in 0..n {
                        for k in 0..n {
                            if i == j
                                || j == k
                                || !order.is_edge(i, j)
                                || !order.is_edge(j, k)
                                || order.is_edge(i, k)
                            {
                                continue;
                            }
                            if f(k, i) + f(i, k) == 0 {
                                order.add_edge(i, k);
                                changed = true;
                            } else if i == k {
                                return None;
                            } else {
                                clusters = merge(nodes[i][0], nodes[k][0], clusters);
                                continue 'restart;
                            }
                        }
                    }
                }
            }
        }
        let mut merged = false;
        if !order.is_irreflexive() {
            for i in 0..n {
                for j in i + 1..n {
                    if order.is_edge(i, j) && order.is_edge(j, i) {
                        clusters = merge(nodes[i][0], nodes[j][0], clusters);
                        merged = true;
                    }
                }
            }
        }
        if merged {
            continue;
        }
        for i in 0..n {
            for j in i + 1..n {
                if !order.is_edge(i, j) && !order.is_edge(j, i) && f(i, j) == 0 && f(j, i) == 0 {
                    clusters = merge(nodes[i][0], nodes[j][0], clusters);
                    merged = true;
                }
            }
        }
        if merged {
            continue;
        }
        let pre = |j: usize| (0..n).filter(|&i| order.is_edge(i, j)).collect::<Vec<_>>();
        let post = |i: usize| (0..n).filter(|&j| order.is_edge(i, j)).collect::<Vec<_>>();
        for i in 0..n {
            for j in i + 1..n {
                if pre(i) == pre(j) && post(i) == post(j) {
                    clusters = merge(nodes[i][0], nodes[j][0], clusters);
                    merged = true;
                }
            }
        }
        if merged && clusters.len() > 1 {
            continue;
        }
        // pm4py's cut keeps one direction per pair. By now the order has
        // no pair in both directions, so this drops nothing.
        let mut cut = BinaryRelation::new(n);
        for i in 0..n {
            for j in i + 1..n {
                if order.is_edge(i, j) {
                    cut.add_edge(i, j);
                } else if order.is_edge(j, i) {
                    cut.add_edge(j, i);
                }
            }
        }
        return Some((nodes, cut));
    }
}

/// For each pair of clusters, the number of traces where an activity of
/// the first occurs before an activity of the second (pm4py's
/// `compute_efg_frequencies`). The diagonal counts traces with two
/// activities of one cluster.
fn cluster_frequencies(log: &Uvcl, nodes: &[Vec<Act>]) -> Vec<Vec<u64>> {
    let cluster: BTreeMap<Act, usize> = nodes
        .iter()
        .enumerate()
        .flat_map(|(i, c)| c.iter().map(move |&a| (a, i)))
        .collect();
    let mut freq = vec![vec![0; nodes.len()]; nodes.len()];
    for (trace, &count) in log {
        let mut seen = BTreeSet::new();
        for (i, a) in trace.iter().enumerate() {
            for b in &trace[i + 1..] {
                let pair = (cluster[a], cluster[b]);
                if seen.insert(pair) {
                    freq[pair.0][pair.1] += count;
                }
            }
        }
    }
    freq
}

/// Replaces the clusters of `a` and `b` with their concatenation, at the
/// end (pm4py's `merge_lists_based_on_activities`).
fn merge(a: Act, b: Act, clusters: Vec<Vec<Act>>) -> Vec<Vec<Act>> {
    let find = |x: Act| {
        clusters
            .iter()
            .position(|c| c.contains(&x))
            .expect("every activity is in a cluster")
    };
    let (i, j) = (find(a), find(b));
    if i == j {
        return clusters;
    }
    let joined = [clusters[i].clone(), clusters[j].clone()].concat();
    let mut rest: Vec<Vec<Act>> = clusters
        .into_iter()
        .enumerate()
        .filter(|&(k, _)| k != i && k != j)
        .map(|(_, c)| c)
        .collect();
    rest.push(joined);
    rest
}
