//! Temporal profile discovery, ported from pm4py's
//! `discover_temporal_profile`.
//!
//! A temporal profile holds, for each ordered pair of activities `(a, b)`,
//! the mean and the standard deviation of the time from the completion of
//! `a` to the start of a later `b` in the same case. It implements the
//! approach of Stertz, Mangler and Rinderle-Ma, "Temporal Conformance
//! Checking at Runtime based on Time-infused Process Models" (2020).

use std::collections::BTreeMap;

use ichnos_core::{ActivityId, EventKeys, EventLog};
use ichnos_stats::time::for_each_event_pair;
pub use ichnos_stats::time::{TemporalProfile, TemporalProfileOptions};

use crate::Result;

/// Discovers the temporal profile of `log`.
///
/// For each trace and each pair of events `i < j` where event `j` starts no
/// earlier than event `i` completes, records the time from the completion of
/// `i` to the start of `j` under the pair of their activities (see
/// [`ichnos_stats::time::for_each_event_pair`]). The profile maps each pair
/// to the mean and the sample standard deviation of its times.
///
/// Fails if an event has no activity, or lacks a timestamp that the options
/// read, or if the business schedule is invalid.
pub fn discover_temporal_profile(
    log: &EventLog,
    keys: &EventKeys,
    options: &TemporalProfileOptions,
) -> Result<TemporalProfile> {
    let sequences = log.activity_sequences(keys)?;
    let mut times: BTreeMap<(ActivityId, ActivityId), Vec<f64>> = BTreeMap::new();
    for_each_event_pair(
        log,
        keys,
        &sequences,
        options,
        |_, _| true,
        |pair| {
            times
                .entry((pair.from, pair.to))
                .or_default()
                .push(pair.seconds);
        },
    )?;
    let name = |a: ActivityId| sequences.activities.name(a).to_owned();
    Ok(times
        .into_iter()
        .map(|((a, b), values)| ((name(a), name(b)), mean_stdev(&values)))
        .collect())
}

/// The mean and the sample standard deviation of `values`, with a standard
/// deviation of 0 for a single value. `values` is never empty.
fn mean_stdev(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    if values.len() < 2 {
        return (mean, 0.0);
    }
    let squares: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
    (mean, (squares / (n - 1.0)).sqrt())
}

#[cfg(test)]
mod tests;
