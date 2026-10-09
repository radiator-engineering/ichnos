//! Temporal profile conformance, ported from pm4py's
//! `conformance_temporal_profile`.
//!
//! A temporal profile holds, for each ordered pair of activities, the mean
//! and the standard deviation of the time between them. A pair of events in
//! a trace deviates when its time lies more than `zeta` standard deviations
//! from the mean. This implements the approach of Stertz, Mangler and
//! Rinderle-Ma, "Temporal Conformance Checking at Runtime based on
//! Time-infused Process Models" (2020).

use std::collections::BTreeMap;

use ichnos_core::chrono::{DateTime, FixedOffset};
use ichnos_core::{ActivityId, EventKeys, EventLog, Position, Trace};
use ichnos_stats::time::BusinessHours;
use rustc_hash::FxHashMap;

use crate::Result;

/// The mean and the standard deviation, in seconds, of the time between each
/// ordered pair of activities, keyed by `(from, to)`.
///
/// This is the type that `ichnos_discovery::discover_temporal_profile`
/// returns.
pub type TemporalProfile = BTreeMap<(String, String), (f64, f64)>;

/// Options for [`conformance_temporal_profile`].
#[derive(Clone, Debug)]
pub struct TemporalProfileOptions {
    /// How many standard deviations a time may lie from the mean before it
    /// deviates. Default 1, as in `pm4py.conformance_temporal_profile`.
    pub zeta: f64,
    /// Read the start of the later event from `EventKeys::start_timestamp`.
    /// When false, the completion timestamp is also the start, as in pm4py's
    /// log variant.
    pub use_start_timestamp: bool,
    /// Measure only the time inside this weekly schedule. `None` measures
    /// elapsed time. Use the schedule the profile was discovered with.
    pub business_hours: Option<BusinessHours>,
}

impl Default for TemporalProfileOptions {
    fn default() -> Self {
        Self {
            zeta: 1.0,
            use_start_timestamp: false,
            business_hours: None,
        }
    }
}

/// A pair of events whose time lies outside the profile's bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct TemporalDeviation {
    /// The activity of the earlier event.
    pub from: String,
    /// The activity of the later event.
    pub to: String,
    /// The time, in seconds, from the completion of the earlier event to the
    /// start of the later one.
    pub seconds: f64,
    /// How many standard deviations the time lies from the mean. Infinite
    /// when the standard deviation is 0.
    pub zeta: f64,
}

/// Checks each trace of `log` against `profile`.
///
/// For each trace and each pair of events `i < j` where event `j` starts no
/// earlier than event `i` completes, and the profile has their activity
/// pair, the pair deviates when its time is below `mean - zeta * stdev` or
/// above `mean + zeta * stdev`. Returns the deviations of each trace, in log
/// order, ordered by `i` and then `j`.
///
/// Fails if an event has no activity, or lacks a timestamp that the options
/// read, or if the business schedule is invalid. The first event of a trace
/// needs no start timestamp, since no pair starts there.
pub fn conformance_temporal_profile(
    log: &EventLog,
    keys: &EventKeys,
    profile: &TemporalProfile,
    options: &TemporalProfileOptions,
) -> Result<Vec<Vec<TemporalDeviation>>> {
    let sequences = log.activity_sequences(keys)?;
    let start_key = if options.use_start_timestamp {
        &keys.start_timestamp
    } else {
        &keys.timestamp
    };
    let zeta = options.zeta;
    // The profile by activity id, so that the pair loop does not allocate.
    let profile: FxHashMap<(ActivityId, ActivityId), (f64, f64)> = profile
        .iter()
        .filter_map(|((from, to), &bounds)| {
            let id = |name: &str| sequences.activities.get(name);
            Some(((id(from)?, id(to)?), bounds))
        })
        .collect();
    log.traces
        .iter()
        .zip(&sequences.traces)
        .enumerate()
        .map(|(t, (trace, activities))| {
            let completes = dates(trace, &keys.timestamp, t, 0)?;
            let starts = if start_key == &keys.timestamp {
                completes.clone()
            } else {
                // Only later events' starts count, so the first event need
                // not have one, as in pm4py.
                let mut starts = completes.first().copied().into_iter().collect::<Vec<_>>();
                starts.extend(dates(trace, start_key, t, 1)?);
                starts
            };
            let mut deviations = Vec::new();
            for (i, &complete) in completes.iter().enumerate() {
                for (j, &start) in starts.iter().enumerate().skip(i + 1) {
                    if start < complete {
                        continue;
                    }
                    let Some(&(mean, stdev)) = profile.get(&(activities[i], activities[j])) else {
                        continue;
                    };
                    let seconds = match &options.business_hours {
                        Some(hours) => hours.seconds_between(complete, start)?,
                        None => {
                            let delta = start - complete;
                            delta.num_seconds() as f64 + f64::from(delta.subsec_nanos()) / 1e9
                        }
                    };
                    if seconds < mean - zeta * stdev || seconds > mean + zeta * stdev {
                        deviations.push(TemporalDeviation {
                            from: sequences.activities.name(activities[i]).to_owned(),
                            to: sequences.activities.name(activities[j]).to_owned(),
                            seconds,
                            zeta: if stdev > 0.0 {
                                (seconds - mean).abs() / stdev
                            } else {
                                f64::INFINITY
                            },
                        });
                    }
                }
            }
            Ok(deviations)
        })
        .collect()
}

/// The value of the date attribute `key` of each event of `trace` from
/// index `first` on.
fn dates(trace: &Trace, key: &str, t: usize, first: usize) -> Result<Vec<DateTime<FixedOffset>>> {
    trace
        .events
        .iter()
        .enumerate()
        .skip(first)
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

#[cfg(test)]
mod tests {
    use ichnos_core::chrono::{TimeZone, Utc};
    use ichnos_core::{Event, EventKeys, EventLog, Trace};

    use super::*;

    fn event(keys: &EventKeys, activity: &str, start: Option<i64>, complete: i64) -> Event {
        let mut e = Event::new();
        e.insert(keys.activity.as_str(), activity);
        e.insert(
            keys.timestamp.as_str(),
            Utc.timestamp_opt(complete, 0).unwrap(),
        );
        if let Some(s) = start {
            e.insert(
                keys.start_timestamp.as_str(),
                Utc.timestamp_opt(s, 0).unwrap(),
            );
        }
        e
    }

    #[test]
    fn first_event_needs_no_start_timestamp() {
        let keys = EventKeys::default();
        let mut trace = Trace::new();
        trace.events = vec![
            event(&keys, "a", None, 100),
            event(&keys, "b", Some(400), 500),
        ];
        let mut log = EventLog::new();
        log.traces.push(trace);
        let profile: TemporalProfile = [(("a".into(), "b".into()), (100.0, 10.0))].into();
        let options = TemporalProfileOptions {
            use_start_timestamp: true,
            ..TemporalProfileOptions::default()
        };
        let result = conformance_temporal_profile(&log, &keys, &profile, &options).unwrap();
        assert_eq!(
            result,
            vec![vec![TemporalDeviation {
                from: "a".into(),
                to: "b".into(),
                seconds: 300.0,
                zeta: 20.0,
            }]]
        );

        // A later event without a start timestamp still fails.
        log.traces[0].events.push(event(&keys, "c", None, 600));
        assert!(conformance_temporal_profile(&log, &keys, &profile, &options).is_err());
    }
}
