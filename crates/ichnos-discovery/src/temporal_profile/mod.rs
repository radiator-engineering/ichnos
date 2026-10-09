//! Temporal profile discovery, ported from pm4py's
//! `discover_temporal_profile`.
//!
//! A temporal profile holds, for each ordered pair of activities `(a, b)`,
//! the mean and the standard deviation of the time from the completion of
//! `a` to the start of a later `b` in the same case. It implements the
//! approach of Stertz, Mangler and Rinderle-Ma, "Temporal Conformance
//! Checking at Runtime based on Time-infused Process Models" (2020).

use std::collections::BTreeMap;

use ichnos_core::chrono::{DateTime, FixedOffset};
use ichnos_core::{EventKeys, EventLog, Position, Trace};
use ichnos_stats::time::BusinessHours;

use crate::Result;

/// The mean and the sample standard deviation, in seconds, of the time
/// between each ordered pair of activities, keyed by `(from, to)`.
///
/// The standard deviation is 0 when a pair occurs once.
pub type TemporalProfile = BTreeMap<(String, String), (f64, f64)>;

/// Options for [`discover_temporal_profile`].
#[derive(Clone, Debug, Default)]
pub struct TemporalProfileOptions {
    /// Read the start of the later event from `EventKeys::start_timestamp`.
    /// When false, the completion timestamp is also the start, as in pm4py's
    /// log variant.
    pub use_start_timestamp: bool,
    /// Measure only the time inside this weekly schedule. `None` measures
    /// elapsed time.
    pub business_hours: Option<BusinessHours>,
}

/// Discovers the temporal profile of `log`.
///
/// For each trace and each pair of events `i < j` where event `j` starts no
/// earlier than event `i` completes, records the time from the completion of
/// `i` to the start of `j` under the pair of their activities. The profile
/// maps each pair to the mean and the sample standard deviation of its times.
///
/// Fails if an event has no activity, or lacks a timestamp that the options
/// read, or if the business schedule is invalid.
pub fn discover_temporal_profile(
    log: &EventLog,
    keys: &EventKeys,
    options: &TemporalProfileOptions,
) -> Result<TemporalProfile> {
    let sequences = log.activity_sequences(keys)?;
    let start_key = if options.use_start_timestamp {
        &keys.start_timestamp
    } else {
        &keys.timestamp
    };
    let mut times: BTreeMap<(&str, &str), Vec<f64>> = BTreeMap::new();
    for (t, (trace, activities)) in log.traces.iter().zip(&sequences.traces).enumerate() {
        let completes = dates(trace, &keys.timestamp, t)?;
        let starts = if start_key == &keys.timestamp {
            completes.clone()
        } else {
            dates(trace, start_key, t)?
        };
        for (i, &complete) in completes.iter().enumerate() {
            for (j, &start) in starts.iter().enumerate().skip(i + 1) {
                if start < complete {
                    continue;
                }
                let seconds = match &options.business_hours {
                    Some(hours) => hours.seconds_between(complete, start)?,
                    None => {
                        let delta = start - complete;
                        delta.num_seconds() as f64 + f64::from(delta.subsec_nanos()) / 1e9
                    }
                };
                let pair = (
                    sequences.activities.name(activities[i]),
                    sequences.activities.name(activities[j]),
                );
                times.entry(pair).or_default().push(seconds);
            }
        }
    }
    Ok(times
        .into_iter()
        .map(|((a, b), values)| ((a.to_owned(), b.to_owned()), mean_stdev(&values)))
        .collect())
}

/// The value of the date attribute `key` of each event of `trace`.
fn dates(trace: &Trace, key: &str, t: usize) -> Result<Vec<DateTime<FixedOffset>>> {
    trace
        .events
        .iter()
        .enumerate()
        .map(|(e, event)| {
            let position = Position::Event { trace: t, event: e };
            let value = event
                .get(key)
                .ok_or_else(|| ichnos_core::Error::MissingAttribute {
                    key: key.to_owned(),
                    position,
                })?;
            Ok(value
                .as_date()
                .ok_or_else(|| ichnos_core::Error::AttributeType {
                    key: key.to_owned(),
                    position,
                    expected: "date",
                    found: value.type_name(),
                })?)
        })
        .collect()
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
