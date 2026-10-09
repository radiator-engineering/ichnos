//! Elapsed and business time, interval overlap, concurrency and temporal relations.
mod business;
mod matching;
mod relations;
mod temporal_profile;
pub use business::*;
pub use matching::*;
pub use relations::*;
pub use temporal_profile::*;

use crate::{Error, Result};
use ichnos_core::{
    Event, EventKeys, EventLog, Position,
    chrono::{DateTime, FixedOffset},
};
use std::collections::BTreeMap;

pub(crate) fn date(event: &Event, key: &str, position: Position) -> Result<DateTime<FixedOffset>> {
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
}
pub(crate) fn seconds(d: DateTime<FixedOffset>) -> f64 {
    d.timestamp() as f64 + d.timestamp_subsec_nanos() as f64 / 1e9
}

/// Aggregation of observed durations.
#[derive(Clone, Copy, Debug, Default)]
pub enum Aggregation {
    /// Arithmetic mean.
    #[default]
    Mean,
    /// Median, averaging the middle pair for even samples.
    Median,
    /// Minimum.
    Min,
    /// Maximum.
    Max,
    /// Sum.
    Sum,
    /// Sample standard deviation, zero for one observation.
    StandardDeviation,
}
pub(crate) fn aggregate(values: &[f64], how: Aggregation) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len();
    let sum = values.iter().sum::<f64>();
    match how {
        Aggregation::Mean => sum / n as f64,
        Aggregation::Sum => sum,
        Aggregation::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        Aggregation::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        Aggregation::Median => {
            let mut v = values.to_vec();
            v.sort_by(f64::total_cmp);
            (v[(n - 1) / 2] + v[n / 2]) / 2.0
        }
        Aggregation::StandardDeviation => {
            if n == 1 {
                0.0
            } else {
                let mean = sum / n as f64;
                (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
            }
        }
    }
}
/// Timestamp and duration options. Completion timestamps are also starts by default.
#[derive(Clone, Debug, Default)]
pub struct TimeOptions {
    /// Use EventKeys.start_timestamp instead of completion timestamps for starts.
    pub use_start_timestamp: bool,
    /// Optional weekly schedule; None measures elapsed time.
    pub business_hours: Option<BusinessHours>,
    /// Duration aggregation.
    pub aggregation: Aggregation,
}
impl TimeOptions {
    pub(crate) fn start_key<'a>(&self, keys: &'a EventKeys) -> &'a str {
        if self.use_start_timestamp {
            &keys.start_timestamp
        } else {
            &keys.timestamp
        }
    }
    pub(crate) fn duration(
        &self,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
    ) -> Result<f64> {
        match &self.business_hours {
            Some(schedule) => schedule.seconds_between(start, end),
            None => Ok(seconds(end) - seconds(start)),
        }
    }
}
/// Service duration per activity, using configured start and completion keys.
pub fn get_service_time(
    log: &EventLog,
    keys: &EventKeys,
    options: &TimeOptions,
) -> Result<BTreeMap<String, f64>> {
    let seq = log.activity_sequences(keys)?;
    let mut durations = BTreeMap::<String, Vec<f64>>::new();
    for (ti, t) in log.traces.iter().enumerate() {
        for (ei, e) in t.events.iter().enumerate() {
            let position = Position::Event {
                trace: ti,
                event: ei,
            };
            let duration = options.duration(
                date(e, options.start_key(keys), position)?,
                date(e, &keys.timestamp, position)?,
            )?;
            durations
                .entry(seq.activities.name(seq.traces[ti][ei]).to_owned())
                .or_default()
                .push(duration);
        }
    }
    Ok(durations
        .into_iter()
        .map(|(a, v)| (a, aggregate(&v, options.aggregation)))
        .collect())
}
/// pm4py-compatible cycle computation. The last merged interval is deliberately not added.
///
/// This reproduces the reference implementation's final-interval omission.
pub fn cycle_time(events: &[(f64, f64)], instances: usize) -> Result<f64> {
    if instances == 0 || events.is_empty() {
        return Ok(0.0);
    }
    if events.iter().any(|(s, e)| !s.is_finite() || !e.is_finite()) {
        return Err(Error::InvalidOption("finite cycle timestamps required"));
    }
    let mut events = events.to_vec();
    events.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    let (mut start, mut end) = events[0];
    let mut production = 0.0;
    for (s, e) in events.into_iter().skip(1) {
        if s > end {
            production += end - start;
            start = s;
        }
        end = end.max(e);
    }
    Ok(production / instances as f64)
}
/// Cycle time over all event intervals, divided by the number of cases.
pub fn get_cycle_time(log: &EventLog, keys: &EventKeys, options: &TimeOptions) -> Result<f64> {
    let mut events = Vec::new();
    for (ti, t) in log.traces.iter().enumerate() {
        for (ei, e) in t.events.iter().enumerate() {
            let position = Position::Event {
                trace: ti,
                event: ei,
            };
            events.push((
                seconds(date(e, options.start_key(keys), position)?),
                seconds(date(e, &keys.timestamp, position)?),
            ));
        }
    }
    cycle_time(&events, log.len())
}
/// Duration and frequency of a neighboring activity.
#[derive(Clone, Debug)]
pub struct PassedTimeNeighbor {
    /// Neighbor's activity.
    pub activity: String,
    /// Aggregated nonnegative directly-follows time.
    pub duration: f64,
    /// Number of observed edges.
    pub count: usize,
}
/// Predecessor and successor duration summaries.
#[derive(Clone, Debug, Default)]
pub struct PassedTime {
    /// Preceding activities.
    pub pre: Vec<PassedTimeNeighbor>,
    /// Following activities.
    pub post: Vec<PassedTimeNeighbor>,
    /// Frequency-weighted predecessor duration.
    pub pre_avg_perf: f64,
    /// Frequency-weighted successor duration.
    pub post_avg_perf: f64,
}
/// Aggregate preceding and succeeding directly-follows times for an activity.
pub fn get_passed_time(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
    options: &TimeOptions,
) -> Result<PassedTime> {
    let seq = log.activity_sequences(keys)?;
    let mut pairs = BTreeMap::<(String, String), Vec<f64>>::new();
    for (ti, t) in log.traces.iter().enumerate() {
        for ei in 1..t.events.len() {
            let source = seq.activities.name(seq.traces[ti][ei - 1]);
            let target = seq.activities.name(seq.traces[ti][ei]);
            let start = date(
                &t.events[ei - 1],
                &keys.timestamp,
                Position::Event {
                    trace: ti,
                    event: ei - 1,
                },
            )?;
            let end = date(
                &t.events[ei],
                options.start_key(keys),
                Position::Event {
                    trace: ti,
                    event: ei,
                },
            )?;
            pairs
                .entry((source.to_owned(), target.to_owned()))
                .or_default()
                .push(options.duration(start, end)?.max(0.0));
        }
    }
    let mut result = PassedTime::default();
    for ((a, b), v) in pairs {
        let neighbor = |activity| PassedTimeNeighbor {
            activity,
            duration: aggregate(&v, options.aggregation),
            count: v.len(),
        };
        if b == activity {
            result.pre.push(neighbor(a.clone()));
        }
        if a == activity {
            result.post.push(neighbor(b));
        }
    }
    let weighted = |neighbors: &[PassedTimeNeighbor]| {
        let n = neighbors.iter().map(|v| v.count).sum::<usize>();
        if n == 0 {
            0.0
        } else {
            neighbors
                .iter()
                .map(|v| v.duration * v.count as f64)
                .sum::<f64>()
                / n as f64
        }
    };
    result.pre_avg_perf = weighted(&result.pre);
    result.post_avg_perf = weighted(&result.post);
    Ok(result)
}
