//! SM2 lifecycle projection, overlap concurrency and potential-OR matching.
use super::{
    Edge, Result,
    graph::{Graph, Kind},
    pair, rpst,
};
use ichnos_core::{ActivitySequences, EventKeys, EventLog, Position};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Complex {
    pub traces: Vec<Vec<usize>>,
    pub is_complex: bool,
    pub overlap: BTreeMap<Edge, u64>,
    pub observed: BTreeMap<usize, u64>,
    pub potential_ors: BTreeSet<Edge>,
}
pub(super) fn parse(
    log: &EventLog,
    keys: &EventKeys,
    sequences: &ActivitySequences,
    index: &BTreeMap<String, usize>,
    start: usize,
    end: usize,
) -> Result<Complex> {
    let mut result = Complex {
        traces: Vec::new(),
        is_complex: false,
        overlap: BTreeMap::new(),
        observed: BTreeMap::new(),
        potential_ors: BTreeSet::new(),
    };
    let (mut starts, mut completes, mut total) = (0usize, 0usize, 0usize);
    let mut executed_traces = Vec::new();
    let mut all = BTreeSet::new();
    for (t, trace) in log.traces.iter().enumerate() {
        let mut events: Vec<_> = trace.events.iter().enumerate().collect();
        if events.iter().all(|(_, e)| e.get(&keys.timestamp).is_some()) {
            let mut dated = Vec::new();
            for (i, e) in events {
                let value = e.get(&keys.timestamp).unwrap();
                let date = value
                    .as_date()
                    .ok_or_else(|| ichnos_core::Error::AttributeType {
                        key: keys.timestamp.clone(),
                        position: Position::Event { trace: t, event: i },
                        expected: "date",
                        found: value.type_name(),
                    })?;
                dated.push((date, i, e));
            }
            // Python datetime truncates fractional seconds to microseconds.
            dated.sort_by_key(|&(date, i, _)| (date.timestamp_micros(), i));
            events = dated.into_iter().map(|(_, i, e)| (i, e)).collect();
        }
        let mut executing = BTreeSet::new();
        let mut executed = BTreeSet::new();
        let mut ids = vec![start];
        for (i, event) in events {
            let a = index[sequences.activities.name(sequences.traces[t][i])];
            all.insert(a);
            let phase = event
                .get(&keys.transition)
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string())
                })
                .unwrap_or_else(|| "complete".into())
                .trim()
                .to_lowercase();
            total += 1;
            match phase.as_str() {
                "start" => {
                    starts += 1;
                    for &b in &executing {
                        if a != b {
                            *result.overlap.entry(pair(a, b)).or_default() += 1;
                        }
                    }
                    executing.insert(a);
                    executed.insert(a);
                }
                "complete" => {
                    completes += 1;
                    executing.remove(&a);
                    *result.observed.entry(a).or_default() += 1;
                    ids.push(a);
                    executed.insert(a);
                }
                _ => {}
            }
        }
        ids.push(end);
        result.traces.push(ids);
        executed_traces.push(executed);
    }
    result.is_complex = (starts.abs_diff(completes) as f64) < (total as f64 * 0.5);
    if result.is_complex {
        for (&p, &count) in &result.overlap {
            if count > 0
                && all.contains(&p.0)
                && all.contains(&p.1)
                && executed_traces
                    .iter()
                    .any(|t| t.contains(&p.0) != t.contains(&p.1))
            {
                result.potential_ors.insert(p);
            }
        }
    } else {
        result.overlap.clear();
    }
    Ok(result)
}
pub(super) fn concurrency(
    dfg: &mut BTreeMap<Edge, u64>,
    complex: &Complex,
    eps: f64,
    n: usize,
) -> BTreeSet<Edge> {
    let mut inc = vec![0; n];
    let mut out = vec![0; n];
    let mut candidates = Vec::new();
    for (&e, &f) in dfg.iter() {
        out[e.0] += 1;
        inc[e.1] += 1;
        let overlap = complex.overlap.get(&pair(e.0, e.1)).copied().unwrap_or(0);
        let observed = complex.observed.get(&e.0).copied().unwrap_or(0)
            + complex.observed.get(&e.1).copied().unwrap_or(0);
        if observed > 0 && overlap as f64 / observed as f64 > eps {
            candidates.push((f, e));
        }
    }
    candidates.sort();
    let mut concurrent = BTreeSet::new();
    let mut dropped = BTreeSet::new();
    for (_, e) in candidates {
        if dropped.contains(&e) {
            continue;
        }
        let target = if out[e.0] > 1 && inc[e.1] > 1 {
            concurrent.insert(pair(e.0, e.1));
            Some(e)
        } else {
            concurrent.remove(&pair(e.0, e.1));
            let r = (e.1, e.0);
            (dfg.contains_key(&r) && !dropped.contains(&r) && out[r.0] > 1 && inc[r.1] > 1)
                .then_some(r)
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
pub(super) fn or_splits(g: &mut Graph, potential: &BTreeSet<Edge>) {
    if potential.is_empty() {
        return;
    }
    let mut converted = false;
    for v in g.order.clone() {
        if g.nodes[v].is_none() || g.kind(v) != Kind::And || g.out[v].len() <= 1 {
            continue;
        }
        let activities: Vec<_> = g.out[v]
            .iter()
            .copied()
            .filter(|&a| g.kind(a) == Kind::Task)
            .collect();
        let mut count = 0;
        for &a in &activities {
            for &b in &activities {
                if a != b && potential.contains(&pair(a, b)) {
                    count += 1;
                }
            }
        }
        if count > g.out[v].len() {
            g.nodes[v].as_mut().unwrap().kind = Kind::Or;
            converted = true;
        }
    }
    if converted {
        let edges: Vec<_> = g
            .order
            .iter()
            .flat_map(|&a| g.out[a].iter().map(move |&b| (a, b)))
            .collect();
        if let Some(fragments) = rpst::fragments(&edges, g.nodes.len()) {
            for f in fragments.into_iter().rev() {
                if f.kind == 'B' && g.kind(f.entry) == Kind::Or && g.kind(f.exit).gate() {
                    g.nodes[f.exit].as_mut().unwrap().kind = Kind::Or;
                }
            }
        }
    }
}
