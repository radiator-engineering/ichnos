//! Temporal profile conformance, ported from pm4py's
//! `conformance_temporal_profile`.
//!
//! A temporal profile holds, for each ordered pair of activities, the mean
//! and the standard deviation of the time between them. A pair of events in
//! a trace deviates when its time lies more than `zeta` standard deviations
//! from the mean. This implements the approach of Stertz, Mangler and
//! Rinderle-Ma, "Temporal Conformance Checking at Runtime based on
//! Time-infused Process Models" (2020).

use ichnos_core::{ActivityId, EventKeys, EventLog};
use ichnos_stats::time::for_each_event_pair;
pub use ichnos_stats::time::{TemporalProfile, TemporalProfileOptions};
use rustc_hash::FxHashMap;

use crate::Result;

/// Options for [`conformance_temporal_profile`].
#[derive(Clone, Debug)]
pub struct TemporalConformanceOptions {
    /// How many standard deviations a time may lie from the mean before it
    /// deviates. Default 1, as in `pm4py.conformance_temporal_profile`.
    pub zeta: f64,
    /// How to measure the time between two events. Use the options the
    /// profile was discovered with.
    pub time: TemporalProfileOptions,
}

impl Default for TemporalConformanceOptions {
    fn default() -> Self {
        Self {
            zeta: 1.0,
            time: TemporalProfileOptions::default(),
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
/// above `mean + zeta * stdev` (see
/// [`ichnos_stats::time::for_each_event_pair`]). Returns the deviations of
/// each trace, in log order, ordered by `i` and then `j`.
///
/// Fails if an event has no activity, or lacks a timestamp that the options
/// read, or if the business schedule is invalid. The first event of a trace
/// needs no start timestamp, since no pair starts there.
pub fn conformance_temporal_profile(
    log: &EventLog,
    keys: &EventKeys,
    profile: &TemporalProfile,
    options: &TemporalConformanceOptions,
) -> Result<Vec<Vec<TemporalDeviation>>> {
    let sequences = log.activity_sequences(keys)?;
    // The profile by activity id, so that the pair loop does not allocate.
    let bounds: FxHashMap<(ActivityId, ActivityId), (f64, f64)> = profile
        .iter()
        .filter_map(|((from, to), &bounds)| {
            let id = |name: &str| sequences.activities.get(name);
            Some(((id(from)?, id(to)?), bounds))
        })
        .collect();
    let zeta = options.zeta;
    let mut out = vec![Vec::new(); log.traces.len()];
    for_each_event_pair(
        log,
        keys,
        &sequences,
        &options.time,
        |from, to| bounds.contains_key(&(from, to)),
        |pair| {
            let (mean, stdev) = bounds[&(pair.from, pair.to)];
            let seconds = pair.seconds;
            if seconds < mean - zeta * stdev || seconds > mean + zeta * stdev {
                out[pair.trace].push(TemporalDeviation {
                    from: sequences.activities.name(pair.from).to_owned(),
                    to: sequences.activities.name(pair.to).to_owned(),
                    seconds,
                    zeta: if stdev > 0.0 {
                        (seconds - mean).abs() / stdev
                    } else {
                        f64::INFINITY
                    },
                });
            }
        },
    )?;
    Ok(out)
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
        let options = TemporalConformanceOptions {
            time: TemporalProfileOptions {
                use_start_timestamp: true,
                business_hours: None,
            },
            ..TemporalConformanceOptions::default()
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
