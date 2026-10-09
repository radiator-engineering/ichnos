//! Time intervals between events, ported from pm4py's
//! `convert_log_to_time_intervals`
//! (`algo/transformation/log_to_interval_tree/variants/open_paths.py`).

use ichnos_core::{AttributeValue, EventLog};

use crate::Error;

/// Options for [`convert_log_to_time_intervals`], with pm4py's defaults.
#[derive(Debug, Clone)]
pub struct TimeIntervalOptions {
    /// The activity attribute. Default `concept:name`.
    pub activity_key: String,
    /// The completion timestamp attribute. Default `time:timestamp`.
    pub timestamp_key: String,
    /// The start timestamp attribute. Default `time:timestamp`.
    pub start_timestamp_key: String,
    /// Keep only the intervals from the first activity to the second.
    pub filter_activity_couple: Option<(String, String)>,
}

impl Default for TimeIntervalOptions {
    fn default() -> Self {
        TimeIntervalOptions {
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            start_timestamp_key: "time:timestamp".to_owned(),
            filter_activity_couple: None,
        }
    }
}

/// The time between two events of a case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeInterval {
    /// The completion of the source event, in seconds since the Unix epoch.
    pub begin: f64,
    /// The start of the target event, in seconds since the Unix epoch.
    pub end: f64,
    /// The trace index.
    pub trace: usize,
    /// The index of the source event in the trace.
    pub source: usize,
    /// The index of the target event in the trace.
    pub target: usize,
}

/// The time intervals of `log`, as pm4py's `convert_log_to_time_intervals`
/// computes them.
///
/// Each event of a trace but the last links to the first later event that
/// starts no earlier than it completes, if any. The interval runs from the
/// completion of the source to the start of the target. With
/// `filter_activity_couple`, only the links from its first activity to its
/// second are kept. The intervals are sorted by begin, then end.
///
/// pm4py returns the source and target events and the trace attributes;
/// here an interval holds their indices. pm4py fails when two intervals have
/// the same begin and end, because it then compares events; here they keep
/// their log order. An event without the activity attribute matches no
/// activity, where pm4py fails.
///
/// # Errors
///
/// [`Error::Timestamp`] when a timestamp that the walk reads is missing or
/// not a date.
pub fn convert_log_to_time_intervals(
    log: &EventLog,
    options: &TimeIntervalOptions,
) -> Result<Vec<TimeInterval>, Error> {
    let seconds = |t: usize, e: usize, key: &str| -> Result<f64, Error> {
        let date = log.traces[t].events[e]
            .get(key)
            .and_then(AttributeValue::as_date)
            .ok_or_else(|| Error::Timestamp {
                trace: t,
                event: e,
                key: key.to_owned(),
            })?;
        Ok(date.timestamp() as f64 + f64::from(date.timestamp_subsec_nanos()) / 1e9)
    };
    let is = |t: usize, e: usize, activity: &str| {
        log.traces[t].events[e]
            .get(&options.activity_key)
            .and_then(AttributeValue::as_str)
            == Some(activity)
    };
    let mut out = Vec::new();
    for (t, trace) in log.traces.iter().enumerate() {
        for i in 0..trace.events.len().saturating_sub(1) {
            let begin = seconds(t, i, &options.timestamp_key)?;
            for j in i + 1..trace.events.len() {
                let end = seconds(t, j, &options.start_timestamp_key)?;
                if end >= begin {
                    let keep = match &options.filter_activity_couple {
                        None => true,
                        Some((a, b)) => is(t, i, a) && is(t, j, b),
                    };
                    if keep {
                        out.push(TimeInterval {
                            begin,
                            end,
                            trace: t,
                            source: i,
                            target: j,
                        });
                    }
                    break;
                }
            }
        }
    }
    out.sort_by(|x, y| x.begin.total_cmp(&y.begin).then(x.end.total_cmp(&y.end)));
    Ok(out)
}
