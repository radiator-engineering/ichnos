//! BPMN discovery from pm4py's classic and SM2 Split Miner pipelines.
//!
//! Frequency filtering, concurrency, split hierarchies and RPST joins precede
//! optional inclusive-join replacement and the existing BPMN reductions.
mod gateways;
mod graph;
mod lifecycle;
mod rpst;
mod spqr;
use crate::{Error, Result};
use graph::{Graph, Kind};
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::bpmn::{Bpmn, GatewayDirection, GatewayKind, NodeKind};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type Edge = (usize, usize);

/// Split Miner pipeline variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitMinerVariant {
    /// Original frequency-based Split Miner.
    #[default]
    Classic,
    /// Lifecycle-aware Split Miner 2.0, with eta fixed to one.
    Sm2,
}

/// A discovered BPMN and SM2's informational self-loop markers.
#[derive(Debug)]
pub struct SplitMinerResult {
    /// Discovered BPMN diagram.
    pub bpmn: Bpmn,
    /// Tasks marked as self-looping by SM2; these do not add BPMN flows.
    pub looped_tasks: BTreeSet<ichnos_model::bpmn::NodeId>,
}

/// Options for Split Miner BPMN discovery.
#[derive(Clone, Debug)]
pub struct SplitMinerOptions {
    /// Classic by default; SM2 reads lifecycle and optional completion timestamps.
    pub variant: SplitMinerVariant,
    /// Concurrency threshold, default 0.1. Classic uses strict imbalance below
    /// this value; complex SM2 logs use strict overlap above it.
    pub epsilon: f64,
    /// Classic frequency percentile, default 0.4; must be finite in `[0, 1]`.
    /// SM2 ignores this value and fixes the percentile to one.
    pub eta: f64,
    /// Classic inclusive-join replacement, enabled by default.
    /// SM2 ignores this flag and always runs its OR handling.
    pub minimize_or_joins: bool,
}
impl Default for SplitMinerOptions {
    fn default() -> Self {
        Self {
            variant: SplitMinerVariant::Classic,
            epsilon: 0.1,
            eta: 0.4,
            minimize_or_joins: true,
        }
    }
}

/// Discover a Split Miner BPMN model. Use [`discover_split_miner`] for SM2 loop markers.
///
/// Classic ignores empty traces; SM2 retains empty traces as a start/end path.
/// Classic needs no timestamps. SM2 uses `keys.transition` and stably sorts a
/// trace by `keys.timestamp` when all its events have timestamps. Activity
/// errors retain core positions.
/// Synthetic boundaries and gateways have distinct arena identities even when
/// activity names resemble pm4py's synthetic labels. The precomputed-DFG discovery entry point
/// is not ported; paths can be loaded separately through reader APIs.
pub fn bpmn_split_miner(
    log: &EventLog,
    keys: &EventKeys,
    options: &SplitMinerOptions,
) -> Result<Bpmn> {
    Ok(discover_split_miner(log, keys, options)?.bpmn)
}

/// Discover a BPMN together with SM2's informational self-loop task markers.
///
/// Shares the options and input semantics of [`bpmn_split_miner`]. SM2 always
/// runs OR handling, fixes eta to one and ignores the supplied eta/minimization
/// settings, as in pm4py. Markers replace Python's private `_sm_looped` attribute;
/// the BPMN itself has no loop-characteristic field.
pub fn discover_split_miner(
    log: &EventLog,
    keys: &EventKeys,
    options: &SplitMinerOptions,
) -> Result<SplitMinerResult> {
    if [
        options.epsilon,
        if options.variant == SplitMinerVariant::Classic {
            options.eta
        } else {
            1.0
        },
    ]
    .iter()
    .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
    {
        return Err(Error::InvalidOption(
            "split miner thresholds must be finite fractions in [0, 1]",
        ));
    }
    let seq = log.activity_sequences(keys)?;
    let mut names: Vec<_> = seq
        .activities
        .iter()
        .map(|(_, name)| name.to_owned())
        .collect();
    names.sort();
    if (names.is_empty() && options.variant == SplitMinerVariant::Classic) || log.traces.is_empty()
    {
        return Err(Error::InvalidOption(
            "split miner needs a nonempty activity trace",
        ));
    }
    // Typed boundaries are placed by their reference lexical keys, without label collisions.
    let mut labels: Vec<_> = names.into_iter().map(|s| (s, Kind::Task)).collect();
    labels.push(("__start__".into(), Kind::Start));
    labels.push(("__end__".into(), Kind::End));
    labels.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut graph = Graph::new(labels);
    let mut index = BTreeMap::new();
    for (i, n) in graph.nodes.iter().enumerate() {
        let n = n.as_ref().unwrap();
        if n.kind == Kind::Task {
            index.insert(n.key.clone(), i);
        }
    }
    let start = graph.start;
    let end = graph.end;
    let mut dfg = BTreeMap::<Edge, u64>::new();
    let mut appearance = Vec::new();
    let mut self_loops = BTreeSet::new();
    let mut short = BTreeSet::new();
    let mut traces = Vec::new();
    let complex = if options.variant == SplitMinerVariant::Sm2 {
        Some(lifecycle::parse(log, keys, &seq, &index, start, end)?)
    } else {
        None
    };
    let projected = if let Some(c) = &complex {
        c.traces.clone()
    } else {
        seq.traces
            .iter()
            .filter(|t| !t.is_empty())
            .map(|trace| {
                let mut ids = vec![start];
                ids.extend(trace.iter().map(|&a| index[seq.activities.name(a)]));
                ids.push(end);
                ids
            })
            .collect()
    };
    for ids in projected {
        for w in ids.windows(2) {
            let edge = (w[0], w[1]);
            if !dfg.contains_key(&edge) {
                appearance.push(edge);
            }
            *dfg.entry(edge).or_default() += 1;
            if edge.0 == edge.1 {
                self_loops.insert(edge.0);
            }
        }
        traces.push(ids);
    }
    let active: BTreeSet<_> = dfg.keys().flat_map(|&(a, b)| [a, b]).collect();
    for i in 0..graph.nodes.len() {
        if !active.contains(&i) {
            graph.remove(i);
        }
    }
    for trace in &traces {
        for w in trace.windows(3) {
            if w[0] == w[2]
                && w[0] != w[1]
                && !self_loops.contains(&w[0])
                && !self_loops.contains(&w[1])
            {
                short.insert(pair(w[0], w[1]));
            }
        }
    }
    dfg.retain(|&(a, b), _| a != b);
    appearance.retain(|e| e.0 != e.1);
    let concurrent = if let Some(c) = complex.as_ref().filter(|c| c.is_complex) {
        lifecycle::concurrency(&mut dfg, c, options.epsilon, graph.nodes.len())
    } else {
        concurrency(&mut dfg, &short, options.epsilon, graph.nodes.len())
    };
    appearance.retain(|e| dfg.contains_key(e));
    let kept = filter(
        &dfg,
        &appearance,
        start,
        end,
        if options.variant == SplitMinerVariant::Sm2 {
            1.0
        } else {
            options.eta
        },
        graph.nodes.len(),
    );
    for &(a, b) in kept.iter().rev() {
        graph.edge(a, b);
    }
    graph.concurrent = concurrent;
    graph.splits();
    graph.joins();
    if options.variant == SplitMinerVariant::Sm2 {
        gateways::replace(&mut graph, false);
        lifecycle::or_splits(&mut graph, &complex.as_ref().unwrap().potential_ors);
    } else if options.minimize_or_joins {
        gateways::replace(&mut graph, true);
    }
    for &task in self_loops
        .iter()
        .rev()
        .filter(|_| options.variant == SplitMinerVariant::Classic)
    {
        let join = graph.add(Kind::Xor);
        let split = graph.add(Kind::Xor);
        for from in graph.ins[task].clone() {
            graph.unedge(from, task);
            graph.edge(from, join);
        }
        for to in graph.out[task].clone() {
            graph.unedge(task, to);
            graph.edge(split, to);
        }
        graph.edge(join, task);
        graph.edge(task, split);
        graph.edge(split, join);
    }
    let mut bpmn = Bpmn::new("split_miner");
    let mut ids = BTreeMap::new();
    for &i in &graph.order {
        if let Some(n) = &graph.nodes[i] {
            let kind = match n.kind {
                Kind::Task => NodeKind::task(),
                Kind::Start => NodeKind::start_event(),
                Kind::End => NodeKind::end_event(),
                k => NodeKind::gateway(
                    match k {
                        Kind::Xor => GatewayKind::Exclusive,
                        Kind::And => GatewayKind::Parallel,
                        _ => GatewayKind::Inclusive,
                    },
                    GatewayDirection::Unspecified,
                ),
            };
            let name = if n.kind == Kind::Task {
                n.key.as_str()
            } else {
                ""
            };
            ids.insert(
                i,
                bpmn.add_node_with_id(format!("split_node_{i}"), kind, name),
            );
        }
    }
    for &i in &graph.order {
        if graph.nodes[i].is_some() {
            for &to in &graph.out[i] {
                bpmn.add_flow(ids[&i], ids[&to])
                    .expect("live working graph endpoints");
            }
        }
    }
    bpmn.reduce(options.variant == SplitMinerVariant::Classic);
    let looped_tasks = if options.variant == SplitMinerVariant::Sm2 {
        self_loops.iter().map(|t| ids[t]).collect()
    } else {
        BTreeSet::new()
    };
    Ok(SplitMinerResult { bpmn, looped_tasks })
}
fn pair(a: usize, b: usize) -> Edge {
    if a < b { (a, b) } else { (b, a) }
}
fn concurrency(
    dfg: &mut BTreeMap<Edge, u64>,
    short: &BTreeSet<Edge>,
    eps: f64,
    n: usize,
) -> BTreeSet<Edge> {
    let mut inc = vec![0; n];
    let mut out = vec![0; n];
    for &(a, b) in dfg.keys() {
        out[a] += 1;
        inc[b] += 1;
    }
    let mut candidates = Vec::new();
    let mut concurrent = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for (&(a, b), &f) in dfg.iter() {
        let p = pair(a, b);
        if !seen.insert(p) || short.contains(&p) {
            continue;
        }
        let g = dfg.get(&(b, a)).copied().unwrap_or(0);
        if g == 0 {
            continue;
        }
        if f.abs_diff(g) as f64 / ((f + g) as f64) < eps {
            concurrent.insert(p);
            candidates.push((f, (a, b), true));
            candidates.push((g, (b, a), true));
        } else {
            candidates.push((f.min(g), if f < g { (a, b) } else { (b, a) }, false));
        }
    }
    candidates.sort();
    let mut dropped = BTreeSet::new();
    for (_, e, par) in candidates {
        if dropped.contains(&e) {
            continue;
        }
        let target = if out[e.0] > 1 && inc[e.1] > 1 {
            Some(e)
        } else if par {
            concurrent.remove(&pair(e.0, e.1));
            let r = (e.1, e.0);
            (!dropped.contains(&r) && out[r.0] > 1 && inc[r.1] > 1).then_some(r)
        } else {
            None
        };
        if let Some((a, b)) = target {
            dropped.insert((a, b));
            out[a] -= 1;
            inc[b] -= 1;
        }
    }
    dfg.retain(|e, _| !dropped.contains(e));
    concurrent
}
fn filter(
    dfg: &BTreeMap<Edge, u64>,
    appearance: &[Edge],
    start: usize,
    end: usize,
    eta: f64,
    n: usize,
) -> BTreeSet<Edge> {
    let mut ins = vec![Vec::new(); n];
    let mut outs = vec![Vec::new(); n];
    for &(a, b) in appearance {
        outs[a].push((a, b));
        ins[b].push((a, b));
    }
    let rank = |e: &Edge| (dfg[e], e.0, e.1);
    let mut freq = BTreeSet::new();
    for i in 0..n {
        if i != end
            && let Some(e) = outs[i].iter().max_by_key(|e| rank(e))
        {
            freq.insert(*e);
        }
        if i != start
            && let Some(e) = ins[i].iter().max_by_key(|e| rank(e))
        {
            freq.insert(*e);
        }
    }
    let mut ordered: Vec<_> = freq.into_iter().collect();
    ordered.sort_by_key(rank);
    let threshold = if ordered.is_empty() {
        0
    } else {
        dfg[&ordered
            [((ordered.len() as f64 * eta).round_ties_even() as usize).min(ordered.len() - 1)]]
    };
    let mut backbone = BTreeSet::new();
    for (root, reverse) in [(start, false), (end, true)] {
        let mut cap = vec![0; n];
        cap[root] = u64::MAX;
        let mut best = vec![None; n];
        let mut queue = VecDeque::from([root]);
        let mut queued = BTreeSet::from([root]);
        while let Some(v) = queue.pop_front() {
            queued.remove(&v);
            for &e in if reverse { &ins[v] } else { &outs[v] } {
                let next = if reverse { e.0 } else { e.1 };
                let c = cap[v].min(dfg[&e]);
                if c > cap[next] {
                    cap[next] = c;
                    best[next] = Some(e);
                    if queued.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }
        backbone.extend(best.into_iter().flatten());
    }
    let mut indeg: Vec<_> = ins.iter().map(Vec::len).collect();
    let mut outdeg: Vec<_> = outs.iter().map(Vec::len).collect();
    let mut kept = BTreeSet::new();
    let mut removable = Vec::new();
    for (&e, &f) in dfg {
        if f >= threshold || backbone.contains(&e) {
            kept.insert(e);
        } else {
            removable.push(e);
        }
    }
    removable.sort_by_key(rank);
    for e in removable {
        if indeg[e.1] == 1 || outdeg[e.0] == 1 {
            kept.insert(e);
        } else {
            indeg[e.1] -= 1;
            outdeg[e.0] -= 1;
        }
    }
    kept
}
