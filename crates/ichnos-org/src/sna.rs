//! Social networks of resources, ported from pm4py's
//! `algo/organizational_mining/sna/variants/log`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_core::{EventKeys, EventLog};
use indexmap::IndexMap;

use crate::error::{Error, Result, event_str};

/// A social network: one value for each ordered pair of resources that the
/// metric links (pm4py's `SNA`).
#[derive(Debug, Clone, PartialEq)]
pub struct Sna {
    /// The value of each `(from, to)` pair.
    pub connections: BTreeMap<(String, String), f64>,
    /// Whether the metric is directed. An undirected metric holds both
    /// orders of each pair.
    pub directed: bool,
}

/// The resource sequence of each trace and how many traces share it, in the
/// order of first occurrence (pm4py's variants on the resource attribute).
fn resource_variants(log: &EventLog, resource: &str) -> Result<IndexMap<Vec<String>, u64>> {
    let mut variants: IndexMap<Vec<String>, u64> = IndexMap::new();
    for (t, trace) in log.traces.iter().enumerate() {
        let seq = (0..trace.events.len())
            .map(|e| event_str(log, t, e, resource))
            .collect::<Result<Vec<_>>>()?;
        *variants.entry(seq).or_default() += 1;
    }
    Ok(variants)
}

/// The handover-of-work network, as pm4py's
/// `discover_handover_of_work_network` computes it on an event log.
///
/// With `beta` 0, each resource hands over to the next resource of its
/// trace. Otherwise it hands over to every later resource, weighted by
/// `beta` to the power of the number of resources in between. Values are
/// divided by the total weight.
///
/// # Errors
///
/// An event without the resource attribute.
pub fn discover_handover_of_work_network(
    log: &EventLog,
    beta: f64,
    keys: &EventKeys,
) -> Result<Sna> {
    let mut sums: BTreeMap<(String, String), f64> = BTreeMap::new();
    let mut dividend = 0.0;
    for (rv, &occ) in &resource_variants(log, &keys.resource)? {
        let occ = occ as f64;
        for i in 0..rv.len().saturating_sub(1) {
            for j in i + 1..rv.len() {
                let weight = if beta == 0.0 {
                    occ
                } else {
                    occ * beta.powi(i32::try_from(j - i - 1).unwrap_or(i32::MAX))
                };
                *sums.entry((rv[i].clone(), rv[j].clone())).or_default() += weight;
                dividend += weight;
                if beta == 0.0 {
                    break;
                }
            }
        }
    }
    Ok(Sna {
        connections: sums.into_iter().map(|(k, v)| (k, v / dividend)).collect(),
        directed: true,
    })
}

/// The working-together network, as pm4py's
/// `discover_working_together_network` computes it on an event log.
///
/// Two resources are linked, both ways, by the share of traces in which both
/// work.
///
/// # Errors
///
/// An event without the resource attribute.
pub fn discover_working_together_network(log: &EventLog, keys: &EventKeys) -> Result<Sna> {
    let traces = log.traces.len() as f64;
    let mut connections: BTreeMap<(String, String), f64> = BTreeMap::new();
    for (rv, &occ) in &resource_variants(log, &keys.resource)? {
        let share = occ as f64 / traces;
        let set: Vec<&String> = rv.iter().collect::<BTreeSet<_>>().into_iter().collect();
        for (i, a) in set.iter().enumerate() {
            for b in &set[i + 1..] {
                *connections.entry(((*a).clone(), (*b).clone())).or_default() += share;
                *connections.entry(((*b).clone(), (*a).clone())).or_default() += share;
            }
        }
    }
    Ok(Sna {
        connections,
        directed: false,
    })
}

/// The similarity of resources by the activities they do, as pm4py's
/// `discover_activity_based_resource_similarity` computes it on an event
/// log.
///
/// Each resource has a profile: how often it does each activity. Two
/// resources are linked, both ways, by the Pearson correlation of their
/// profiles. The correlation is NaN when a profile is constant, or when the
/// log has fewer than two activities; pm4py fails on the latter.
///
/// # Errors
///
/// An event without the activity or resource attribute.
pub fn discover_activity_based_resource_similarity(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<Sna> {
    let mut counts: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut activities: BTreeSet<String> = BTreeSet::new();
    for (t, trace) in log.traces.iter().enumerate() {
        for e in 0..trace.events.len() {
            let act = event_str(log, t, e, &keys.activity)?;
            let res = event_str(log, t, e, &keys.resource)?;
            *counts
                .entry(res)
                .or_default()
                .entry(act.clone())
                .or_default() += 1.0;
            activities.insert(act);
        }
    }
    let profiles: Vec<(&String, Vec<f64>)> = counts
        .iter()
        .map(|(res, acts)| {
            let row = activities
                .iter()
                .map(|a| acts.get(a).copied().unwrap_or(0.0))
                .collect();
            (res, row)
        })
        .collect();
    let mut connections = BTreeMap::new();
    for (a, x) in &profiles {
        for (b, y) in &profiles {
            if a != b {
                connections.insert(((*a).clone(), (*b).clone()), pearson(x, y));
            }
        }
    }
    Ok(Sna {
        connections,
        directed: false,
    })
}

/// SciPy's `pearsonr` statistic.
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let constant = |v: &[f64]| v.iter().all(|&a| a == v[0]);
    if x.len() < 2 || constant(x) || constant(y) {
        return f64::NAN;
    }
    let n = x.len() as f64;
    let centred = |v: &[f64]| {
        let mean = v.iter().sum::<f64>() / n;
        let c: Vec<f64> = v.iter().map(|a| a - mean).collect();
        let max = c.iter().fold(0.0_f64, |m, a| m.max(a.abs()));
        let norm = max * c.iter().map(|a| (a / max).powi(2)).sum::<f64>().sqrt();
        c.into_iter().map(|a| a / norm).collect::<Vec<_>>()
    };
    let (xs, ys) = (centred(x), centred(y));
    let r: f64 = xs.iter().zip(&ys).map(|(a, b)| a * b).sum();
    r.clamp(-1.0, 1.0)
}

/// The subcontracting network, as pm4py's `discover_subcontracting_network`
/// computes it on an event log.
///
/// A resource subcontracts to the resources between two of its events that
/// are `n` events apart. Values count traces and are divided by the number
/// of traces.
///
/// pm4py counts only the first subcontracting window of each resource: the
/// first trace variant, in log order, where the resource opens a window. Later
/// windows of the same resource are skipped. This is kept.
///
/// # Errors
///
/// [`Error::InvalidOption`] when `n` is 0; pm4py raises `ValueError` there,
/// because the network is empty. An event without the resource attribute.
pub fn discover_subcontracting_network(log: &EventLog, n: usize, keys: &EventKeys) -> Result<Sna> {
    if n == 0 {
        return Err(Error::InvalidOption(
            "n must be at least 1 for the subcontracting network",
        ));
    }
    let mut sums: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut dividend = 0_u64;
    for (rv, &occ) in &resource_variants(log, &keys.resource)? {
        dividend += occ;
        for i in 0..rv.len().saturating_sub(n) {
            if rv[i] == rv[i + n] && !sums.contains_key(&rv[i]) {
                let inner = sums.entry(rv[i].clone()).or_default();
                for res in &rv[i + 1..i + n] {
                    *inner.entry(res.clone()).or_default() += occ;
                }
            }
        }
    }
    let connections = sums
        .into_iter()
        .flat_map(|(a, inner)| {
            inner
                .into_iter()
                .map(move |(b, c)| ((a.clone(), b), c as f64 / dividend as f64))
        })
        .collect();
    Ok(Sna {
        connections,
        directed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::pearson;

    #[test]
    fn pearson_matches_scipy() {
        // scipy.stats.pearsonr([1, 2, 0], [2, 3, 1]).statistic == 1.0
        let r = pearson(&[1.0, 2.0, 0.0], &[2.0, 3.0, 1.0]);
        assert!((r - 1.0).abs() < 1e-12, "{r}");
        assert!(pearson(&[1.0, 1.0], &[1.0, 2.0]).is_nan());
        // -3 / sqrt(84), by hand.
        let r = pearson(&[1.0, 0.0, 3.0], &[0.0, 2.0, 1.0]);
        assert!((r + 3.0 / 84f64.sqrt()).abs() < 1e-12, "{r}");
    }
}
