//! The time measure that temporal profile discovery and conformance share
//! (pm4py's `discover_temporal_profile` and `conformance_temporal_profile`).

use std::collections::BTreeMap;

use ichnos_core::chrono::{DateTime, FixedOffset};
use ichnos_core::{ActivityId, ActivitySequences, EventKeys, EventLog, Position};

use super::{BusinessHours, date};
use crate::Result;

/// The mean and the sample standard deviation, in seconds, of the time
/// between each ordered pair of activities, keyed by `(from, to)`.
///
/// The standard deviation is 0 when a pair occurs once.
pub type TemporalProfile = BTreeMap<(String, String), (f64, f64)>;

/// How a temporal profile measures the time between two events.
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

/// A pair of events of one trace that a temporal profile measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventPair {
    /// The index of the trace in the log.
    pub trace: usize,
    /// The activity of the earlier event.
    pub from: ActivityId,
    /// The activity of the later event.
    pub to: ActivityId,
    /// The time, in seconds, from the completion of the earlier event to
    /// the start of the later one.
    pub seconds: f64,
}

/// Visits the event pairs of a temporal profile, trace by trace in log
/// order, then by the earlier event and the later event.
///
/// A pair is two events `i < j` of one trace where `j` starts no earlier
/// than `i` completes. `sequences` must be `log.activity_sequences(keys)`.
/// `wanted` sees each pair's activities first; the time is computed and
/// `visit` called only when it returns true. Elapsed time is exact, as in
/// pm4py's dataframe variant.
///
/// Only events after the first need a start timestamp, as in pm4py's log
/// variant, since no pair starts at the first event. Fails if an event lacks
/// a timestamp that the options read, or if the business schedule is
/// invalid.
pub fn for_each_event_pair(
    log: &EventLog,
    keys: &EventKeys,
    sequences: &ActivitySequences,
    options: &TemporalProfileOptions,
    mut wanted: impl FnMut(ActivityId, ActivityId) -> bool,
    mut visit: impl FnMut(EventPair),
) -> Result<()> {
    for (t, (trace, activities)) in log.traces.iter().zip(&sequences.traces).enumerate() {
        let dates = |key: &str, first: usize| -> Result<Vec<DateTime<FixedOffset>>> {
            trace
                .events
                .iter()
                .enumerate()
                .skip(first)
                .map(|(e, event)| date(event, key, Position::Event { trace: t, event: e }))
                .collect()
        };
        let completes = dates(&keys.timestamp, 0)?;
        let starts = if options.use_start_timestamp {
            // starts[0] is never read.
            let mut starts: Vec<_> = completes.first().copied().into_iter().collect();
            starts.extend(dates(&keys.start_timestamp, 1)?);
            starts
        } else {
            completes.clone()
        };
        for (i, &complete) in completes.iter().enumerate() {
            for (j, &start) in starts.iter().enumerate().skip(i + 1) {
                if start < complete || !wanted(activities[i], activities[j]) {
                    continue;
                }
                let seconds = match &options.business_hours {
                    Some(hours) => hours.seconds_between(complete, start)?,
                    None => {
                        let delta = start - complete;
                        delta.num_seconds() as f64 + f64::from(delta.subsec_nanos()) / 1e9
                    }
                };
                visit(EventPair {
                    trace: t,
                    from: activities[i],
                    to: activities[j],
                    seconds,
                });
            }
        }
    }
    Ok(())
}
