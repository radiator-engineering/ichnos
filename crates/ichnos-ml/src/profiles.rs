//! Binary string/succession features and last-observed numeric attributes.
use ichnos_core::{AttributeValue, EventLog};
use std::collections::{BTreeMap, BTreeSet};

/// Explicit feature selection, avoiding random attribute sampling.
#[derive(Debug, Clone, Default)]
pub struct ProfileOptions {
    /// Categorical trace attributes.
    pub string_trace: Vec<String>,
    /// Categorical event attributes.
    pub string_event: Vec<String>,
    /// Numeric trace attributes.
    pub numeric_trace: Vec<String>,
    /// Numeric event attributes (last observed value per trace).
    pub numeric_event: Vec<String>,
    /// Adjacent categorical event values.
    pub succession: Vec<String>,
}
impl ProfileOptions {
    /// Activity-presence and directly-follows presence features only.
    pub fn activities(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            string_event: vec![key.clone()],
            succession: vec![key],
            ..Self::default()
        }
    }
    /// Select attributes present in every trace, and categorical attributes
    /// with fewer than 12.5 distinct values. The complete log is scanned.
    pub fn infer(log: &EventLog, activity_key: impl Into<String>) -> Self {
        let mut out = Self::default();
        for trace_level in [true, false] {
            let mut candidates: BTreeMap<String, (BTreeSet<String>, bool, bool, BTreeSet<usize>)> =
                BTreeMap::new();
            for (i, t) in log.traces.iter().enumerate() {
                let attributes: Vec<_> = if trace_level {
                    vec![&t.attributes]
                } else {
                    t.events.iter().map(|e| &e.attributes).collect()
                };
                for attrs in attributes {
                    for (key, v) in attrs.iter() {
                        if (trace_level && key.as_ref() == "concept:name")
                            || (!trace_level && key.as_ref() == "lifecycle:transition")
                        {
                            continue;
                        }
                        let c = candidates
                            .entry(key.to_string())
                            .or_insert_with(|| (BTreeSet::new(), true, true, BTreeSet::new()));
                        c.1 &= matches!(v.plain(), AttributeValue::String(_));
                        c.2 &= v.as_f64().is_some();
                        c.3.insert(i);
                        if let Some(s) = v.as_str() {
                            c.0.insert(s.to_string());
                        }
                    }
                }
            }
            for (key, (values, string, numeric, present)) in candidates {
                if present.len() != log.traces.len() {
                    continue;
                }
                if string && values.len() < 13 {
                    if trace_level {
                        out.string_trace.push(key)
                    } else {
                        out.string_event.push(key)
                    }
                } else if numeric {
                    if trace_level {
                        out.numeric_trace.push(key)
                    } else {
                        out.numeric_event.push(key)
                    }
                }
            }
        }
        let key = activity_key.into();
        if !out.string_event.contains(&key) {
            out.string_event.push(key.clone());
        }
        out.succession.push(key);
        out
    }
}
/// Named dense features, one row per input trace.
#[derive(Debug, Clone, PartialEq)]
pub struct Profiles {
    /// Lexically ordered feature names.
    pub names: Vec<String>,
    /// Trace feature vectors.
    pub rows: Vec<Vec<f64>>,
}
/// Invalid data or clustering options.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A selected feature has no compatible value in this trace.
    #[error("trace {trace}: invalid or missing feature attribute {key}")]
    Attribute {
        /// Trace index.
        trace: usize,
        /// Attribute name.
        key: String,
    },
    /// Cluster count, shape, or iteration options are invalid.
    #[error("invalid clustering options: {0}")]
    InvalidOptions(&'static str),
    /// A custom clusterer returned invalid assignments.
    #[error("clusterer returned invalid assignments")]
    InvalidAssignments,
}
/// Sorted, distinct activity labels in an event log.
pub fn activity_labels(log: &EventLog, key: &str) -> Result<BTreeSet<String>, Error> {
    let mut labels = BTreeSet::new();
    for (trace, t) in log.traces.iter().enumerate() {
        for e in &t.events {
            let label =
                e.get(key)
                    .and_then(AttributeValue::as_str)
                    .ok_or_else(|| Error::Attribute {
                        trace,
                        key: key.into(),
                    })?;
            labels.insert(label.to_string());
        }
    }
    Ok(labels)
}
/// Encode string presence, adjacent string pairs, and last numeric values.
pub fn trace_profiles(log: &EventLog, o: &ProfileOptions) -> Result<Profiles, Error> {
    let mut traces = Vec::new();
    let mut names = BTreeSet::new();
    for (i, t) in log.traces.iter().enumerate() {
        let error = |key: &str| Error::Attribute {
            trace: i,
            key: key.into(),
        };
        let mut row = BTreeMap::new();
        for key in &o.string_trace {
            let v = t
                .attributes
                .get(key)
                .map(|v| v.as_str().map(str::to_string));
            let text = match v {
                Some(Some(s)) => s,
                None => "UNDEFINED".into(),
                _ => return Err(error(key)),
            };
            row.insert(format!("trace:{key}@{text}"), 1.0);
        }
        for key in &o.string_event {
            for event in &t.events {
                if let Some(value) = event.get(key) {
                    let s = value.as_str().ok_or_else(|| error(key))?;
                    row.insert(format!("event:{key}@{s}"), 1.0);
                }
            }
        }
        for key in &o.string_event {
            if !t.events.iter().any(|e| e.get(key).is_some()) {
                row.insert(format!("event:{key}@UNDEFINED"), 1.0);
            }
        }
        for key in &o.numeric_trace {
            let n = t
                .attributes
                .get(key)
                .and_then(AttributeValue::as_f64)
                .filter(|n| n.is_finite())
                .ok_or_else(|| error(key))?;
            row.insert(format!("trace:{key}"), n);
        }
        for key in &o.numeric_event {
            let n = t
                .events
                .iter()
                .rev()
                .find_map(|e| e.get(key))
                .and_then(AttributeValue::as_f64)
                .filter(|n| n.is_finite())
                .ok_or_else(|| error(key))?;
            row.insert(format!("event:{key}"), n);
        }
        for key in &o.succession {
            for pair in t.events.windows(2) {
                if let (Some(a), Some(b)) = (pair[0].get(key), pair[1].get(key)) {
                    let a = a.as_str().ok_or_else(|| error(key))?;
                    let b = b.as_str().ok_or_else(|| error(key))?;
                    row.insert(format!("succession:{key}@{a}#{b}"), 1.0);
                }
            }
        }
        for key in &o.succession {
            if !t
                .events
                .windows(2)
                .any(|pair| pair[0].get(key).is_some() && pair[1].get(key).is_some())
            {
                row.insert(format!("succession:{key}@UNDEFINED"), 1.0);
            }
        }
        names.extend(row.keys().cloned());
        traces.push(row);
    }
    let names: Vec<_> = names.into_iter().collect();
    let rows = traces
        .iter()
        .map(|r| {
            names
                .iter()
                .map(|n| r.get(n).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();
    Ok(Profiles { names, rows })
}
/// A user-supplied clustering backend, analogous to `fit_predict`.
pub trait Clusterer {
    /// Assign each input row to a zero-based cluster.
    fn fit_predict(&self, rows: &[Vec<f64>]) -> Result<Vec<usize>, Error>;
}
/// Lloyd K-means with explicit centers and deterministic tie-breaking.
#[derive(Debug, Clone)]
pub struct KMeans {
    /// Number of clusters.
    pub clusters: usize,
    /// Optional initial centers. Otherwise choose the
    /// first point followed by points farthest from the existing centers.
    pub initial_centers: Option<Vec<Vec<f64>>>,
    /// Maximum iterations.
    pub max_iterations: usize,
    /// Relative variance-scaled convergence threshold.
    pub tolerance: f64,
}
impl Default for KMeans {
    fn default() -> Self {
        Self {
            clusters: 2,
            initial_centers: None,
            max_iterations: 300,
            tolerance: 1e-4,
        }
    }
}
fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum()
}
impl Clusterer for KMeans {
    fn fit_predict(&self, rows: &[Vec<f64>]) -> Result<Vec<usize>, Error> {
        let dimensions = rows.first().map_or(0, Vec::len);
        if rows.is_empty()
            || self.clusters == 0
            || self.clusters > rows.len()
            || self.max_iterations == 0
            || !self.tolerance.is_finite()
            || self.tolerance < 0.0
            || dimensions == 0
            || rows
                .iter()
                .any(|r| r.len() != dimensions || r.iter().any(|x| !x.is_finite()))
        {
            return Err(Error::InvalidOptions(
                "nonempty finite matrix, valid cluster count and iterations required",
            ));
        }
        let mut centers = if let Some(c) = &self.initial_centers {
            if c.len() != self.clusters
                || c.iter()
                    .any(|r| r.len() != dimensions || r.iter().any(|x| !x.is_finite()))
            {
                return Err(Error::InvalidOptions("invalid initial centers"));
            }
            c.clone()
        } else {
            let mut c = vec![rows[0].clone()];
            while c.len() < self.clusters {
                let mut chosen = 0;
                let mut farthest = -1.0;
                for (i, r) in rows.iter().enumerate() {
                    let d = c
                        .iter()
                        .map(|x| distance(x, r))
                        .fold(f64::INFINITY, f64::min);
                    if d > farthest {
                        chosen = i;
                        farthest = d;
                    }
                }
                c.push(rows[chosen].clone());
            }
            c
        };
        let assign = |centers: &[Vec<f64>]| -> Vec<usize> {
            rows.iter()
                .map(|r| {
                    centers
                        .iter()
                        .enumerate()
                        .min_by(|(i, a), (j, b)| {
                            distance(r, a).total_cmp(&distance(r, b)).then(i.cmp(j))
                        })
                        .expect("validated cluster count is positive")
                        .0
                })
                .collect()
        };
        let mut variance = 0.0;
        for j in 0..dimensions {
            let mean = rows.iter().map(|r| r[j]).sum::<f64>() / rows.len() as f64;
            variance += rows.iter().map(|r| (r[j] - mean).powi(2)).sum::<f64>() / rows.len() as f64;
        }
        variance /= dimensions as f64;
        if !variance.is_finite() {
            return Err(Error::InvalidOptions(
                "numeric range overflows the variance calculation",
            ));
        }
        let mut previous = vec![usize::MAX; rows.len()];
        for _ in 0..self.max_iterations {
            let labels = assign(&centers);
            let mut sums = vec![vec![0.0; dimensions]; self.clusters];
            let mut counts = vec![0; self.clusters];
            for (r, &l) in rows.iter().zip(&labels) {
                counts[l] += 1;
                for (s, v) in sums[l].iter_mut().zip(r) {
                    *s += v;
                }
            }
            // Relocate the farthest assigned point to each empty cluster.
            let mut taken = BTreeSet::new();
            for empty in 0..self.clusters {
                if counts[empty] == 0 {
                    let candidate = rows
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| !taken.contains(i) && counts[labels[*i]] > 1)
                        .max_by(|(i, a), (j, b)| {
                            distance(a, &centers[labels[*i]])
                                .total_cmp(&distance(b, &centers[labels[*j]]))
                                .then(i.cmp(j))
                        });
                    if let Some((i, r)) = candidate {
                        taken.insert(i);
                        let old = labels[i];
                        counts[old] -= 1;
                        counts[empty] = 1;
                        for (sum, value) in sums[old].iter_mut().zip(r) {
                            *sum -= value;
                        }
                        sums[empty].clone_from(r);
                    }
                }
            }
            let largest = counts
                .iter()
                .enumerate()
                .max_by_key(|(_, n)| *n)
                .expect("validated cluster count is positive")
                .0;
            for c in 0..self.clusters {
                if counts[c] > 0 {
                    for x in &mut sums[c] {
                        *x /= counts[c] as f64;
                    }
                } else {
                    sums[c] = centers[largest].clone();
                }
            }
            if sums.iter().flatten().any(|n| !n.is_finite()) {
                return Err(Error::InvalidOptions(
                    "numeric range overflows the center calculation",
                ));
            }
            let shift = centers
                .iter()
                .zip(&sums)
                .map(|(a, b)| distance(a, b))
                .sum::<f64>();
            centers = sums;
            if labels == previous || shift <= self.tolerance * variance {
                break;
            }
            previous = labels;
        }
        Ok(assign(&centers))
    }
}
/// Partition traces in input order. Cluster logs retain source metadata.
///
/// With [`ProfileOptions::infer`] and [`KMeans::default`], the partition can
/// differ from pm4py's default `cluster_log`: attributes are selected from the
/// complete log, and centers use farthest-point seeding. pm4py samples up to
/// 50 traces and uses sklearn's seed-0 K-means++ initialization. Supply explicit
/// feature options and centers to compare the same Lloyd optimization.
pub fn cluster_log(
    log: &EventLog,
    o: &ProfileOptions,
    clusterer: &impl Clusterer,
) -> Result<Vec<EventLog>, Error> {
    let profiles = trace_profiles(log, o)?;
    let labels = clusterer.fit_predict(&profiles.rows)?;
    if labels.len() != log.traces.len() || labels.iter().any(|&x| x >= log.traces.len()) {
        return Err(Error::InvalidAssignments);
    }
    let count = labels.iter().max().map_or(0, |n| n + 1);
    let mut template = log.clone();
    template.traces.clear();
    let mut logs = vec![template; count];
    for (trace, l) in log.traces.iter().zip(labels) {
        logs[l].traces.push(trace.clone());
    }
    Ok(logs)
}
