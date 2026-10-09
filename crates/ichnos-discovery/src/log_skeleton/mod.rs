//! Classic log-skeleton discovery, including the pinned occurrence-count
//! denominators and event-count frequency coverage used by pm4py.

mod conformance;

pub use conformance::{
    SkeletonConformanceOptions, SkeletonConstraint, SkeletonDeviation, SkeletonTraceConformance,
    conformance_log_skeleton,
};

use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::Label;
use std::collections::{BTreeMap, BTreeSet};

/// Options for classic log-skeleton discovery.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LogSkeletonOptions {
    /// Allowed noise fraction, in `[0, 1]` (default zero).
    pub noise_threshold: f64,
}

/// A directed activity relation in a log skeleton.
pub type SkeletonRelation = BTreeSet<(Label, Label)>;

/// The six components of a classic log skeleton.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogSkeleton {
    /// Activities whose counts agree, weighted by occurrences of the source.
    pub equivalence: SkeletonRelation,
    /// Observed later-activity pairs meeting the source occurrence threshold.
    pub always_after: SkeletonRelation,
    /// Observed earlier-activity pairs meeting the source occurrence threshold.
    pub always_before: SkeletonRelation,
    /// Source occurrence counts minus co-occurring trace counts meeting the threshold.
    pub never_together: SkeletonRelation,
    /// Adjacent-pair occurrence counts meeting the source occurrence threshold.
    pub directly_follows: SkeletonRelation,
    /// Retained per-trace occurrence counts of each activity, including zero.
    pub activity_frequencies: BTreeMap<Label, BTreeSet<u64>>,
}

/// Discovers all six skeleton components from input activity order.
///
/// This follows the pinned source's occurrence-based denominators, including
/// the event-count target for activity-frequency coverage. Frequency ties
/// follow the first occurrence of a trace variant. Empty logs return an empty
/// model; timestamps are not required. Input errors retain core positions.
pub fn log_skeleton(
    log: &EventLog,
    keys: &EventKeys,
    options: &LogSkeletonOptions,
) -> Result<LogSkeleton> {
    if !options.noise_threshold.is_finite() || !(0.0..=1.0).contains(&options.noise_threshold) {
        return Err(Error::NoiseThreshold(options.noise_threshold));
    }
    let sequences = log.activity_sequences(keys)?;
    let traces: Vec<Vec<Label>> = sequences
        .traces
        .iter()
        .map(|t| {
            t.iter()
                .map(|&a| Label::from(sequences.activities.name(a)))
                .collect()
        })
        .collect();
    // Keep variant order for stable frequency ties, and weight each variant
    // without expanding it again.
    let mut variants = Vec::<(Vec<Label>, u64)>::new();
    let mut index = BTreeMap::new();
    let mut totals: BTreeMap<Label, u64> = BTreeMap::new();
    let mut events = 0u64;
    for trace in traces {
        events += trace.len() as u64;
        for a in &trace {
            *totals.entry(a.clone()).or_default() += 1;
        }
        let next = variants.len();
        let i = *index.entry(trace.clone()).or_insert(next);
        if i == next {
            variants.push((trace, 1));
        } else {
            variants[i].1 += 1;
        }
    }
    let mut equivalent = BTreeMap::<(Label, Label), u64>::new();
    let mut after = BTreeMap::<(Label, Label), u64>::new();
    let mut before = BTreeMap::<(Label, Label), u64>::new();
    let mut adjacent = BTreeMap::<(Label, Label), u64>::new();
    let mut together = BTreeMap::<(Label, Label), u64>::new();
    let mut histograms: BTreeMap<Label, Vec<(u64, u64)>> =
        totals.keys().map(|a| (a.clone(), Vec::new())).collect();
    for (trace, weight) in variants {
        let mut counts: BTreeMap<Label, u64> = BTreeMap::new();
        for a in &trace {
            *counts.entry(a.clone()).or_default() += 1;
        }
        for (a, histogram) in &mut histograms {
            let n = counts.get(a).copied().unwrap_or(0);
            if let Some((_, count)) = histogram.iter_mut().find(|(value, _)| *value == n) {
                *count += weight;
            } else {
                histogram.push((n, weight));
            }
        }
        for (a, &n) in &counts {
            for (b, &m) in &counts {
                if a != b {
                    *together.entry((a.clone(), b.clone())).or_default() += weight;
                    if n == m {
                        *equivalent.entry((a.clone(), b.clone())).or_default() += n * weight;
                    }
                }
            }
        }
        let pairs: BTreeSet<_> = trace
            .iter()
            .enumerate()
            .flat_map(|(i, a)| trace[i + 1..].iter().map(move |b| (a.clone(), b.clone())))
            .collect();
        for (a, b) in pairs {
            *after.entry((a.clone(), b.clone())).or_default() += weight;
            *before.entry((b, a)).or_default() += weight;
        }
        for pair in trace.windows(2) {
            *adjacent
                .entry((pair[0].clone(), pair[1].clone()))
                .or_default() += weight;
        }
    }
    let retained = |counts: BTreeMap<(Label, Label), u64>| -> SkeletonRelation {
        counts
            .into_iter()
            .filter(|((a, _), n)| *n as f64 >= totals[a] as f64 * (1.0 - options.noise_threshold))
            .map(|(pair, _)| pair)
            .collect()
    };
    let mut never_together = BTreeSet::new();
    for (a, &n) in &totals {
        for b in totals.keys() {
            if a != b {
                let absent = n - together.get(&(a.clone(), b.clone())).copied().unwrap_or(0);
                // Counter subtraction removes zero entries even at noise one.
                if absent > 0 && absent as f64 >= n as f64 * (1.0 - options.noise_threshold) {
                    never_together.insert((a.clone(), b.clone()));
                }
            }
        }
    }
    let activity_frequencies = histograms
        .into_iter()
        .map(|(a, mut histogram)| {
            histogram.sort_by_key(|value| std::cmp::Reverse(value.1));
            let mut covered = 0u64;
            let mut allowed = BTreeSet::new();
            for (n, count) in histogram {
                covered += count;
                allowed.insert(n);
                if covered as f64 >= events as f64 * (1.0 - options.noise_threshold) {
                    break;
                }
            }
            (a, allowed)
        })
        .collect();
    Ok(LogSkeleton {
        equivalence: retained(equivalent),
        always_after: retained(after),
        always_before: retained(before),
        never_together,
        directly_follows: retained(adjacent),
        activity_frequencies,
    })
}
