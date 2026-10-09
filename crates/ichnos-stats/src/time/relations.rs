use super::{TimeOptions, date, seconds};
use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog, Position};
use std::collections::BTreeMap;

/// One relation between original event indices within a trace.
#[derive(Clone, Debug, PartialEq)]
pub struct EventRelation {
    /// Trace index.
    pub trace: usize,
    /// Source event index in the input trace.
    pub source: usize,
    /// Target event index in the input trace.
    pub target: usize,
    /// Overlap length (concurrency) or flow time (partial order), in seconds.
    pub duration: f64,
}
/// Interval relation options.
#[derive(Clone, Debug, Default)]
pub struct RelationOptions {
    /// Start/completion keys and business schedule.
    pub time: TimeOptions,
    /// Require positive overlap rather than allowing shared boundaries.
    pub strict: bool,
    /// Retain only the first temporally eligible follower of each event.
    pub keep_first_following: bool,
}
#[derive(Clone, Copy)]
struct Interval {
    index: usize,
    start: ichnos_core::chrono::DateTime<ichnos_core::chrono::FixedOffset>,
    end: ichnos_core::chrono::DateTime<ichnos_core::chrono::FixedOffset>,
}
fn intervals(
    log: &EventLog,
    keys: &EventKeys,
    options: &TimeOptions,
) -> Result<Vec<Vec<Interval>>> {
    log.traces
        .iter()
        .enumerate()
        .map(|(ti, t)| {
            let mut events = t
                .events
                .iter()
                .enumerate()
                .map(|(ei, e)| {
                    let position = Position::Event {
                        trace: ti,
                        event: ei,
                    };
                    Ok(Interval {
                        index: ei,
                        start: date(e, options.start_key(keys), position)?,
                        end: date(e, &keys.timestamp, position)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            events.sort_by_key(|e| e.start);
            Ok(events)
        })
        .collect()
}
/// Original event pairs that overlap, sorted stably by start timestamp.
/// Overlap uses elapsed seconds, ignoring business schedules, as the source does.
/// Durations retain subsecond precision; Polars truncates its duration column to whole seconds.
pub fn get_concurrent_events(
    log: &EventLog,
    keys: &EventKeys,
    options: &RelationOptions,
) -> Result<Vec<EventRelation>> {
    let mut result = Vec::new();
    for (ti, t) in intervals(log, keys, &options.time)?.into_iter().enumerate() {
        for (i, a) in t.iter().enumerate() {
            for b in &t[i + 1..] {
                let start = a.start.max(b.start);
                let end = a.end.min(b.end);
                if start > end {
                    break;
                }
                if !options.strict || start < end {
                    result.push(EventRelation {
                        trace: ti,
                        source: a.index,
                        target: b.index,
                        duration: seconds(end) - seconds(start),
                    });
                }
            }
        }
    }
    Ok(result)
}
/// Counts of unordered activity pairs with overlapping event intervals.
pub fn get_concurrent_activities(
    log: &EventLog,
    keys: &EventKeys,
    options: &RelationOptions,
) -> Result<BTreeMap<(String, String), usize>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = BTreeMap::new();
    for pair in get_concurrent_events(log, keys, options)? {
        let a = seq.activities.name(seq.traces[pair.trace][pair.source]);
        let b = seq.activities.name(seq.traces[pair.trace][pair.target]);
        let pair = if a <= b {
            (a.to_owned(), b.to_owned())
        } else {
            (b.to_owned(), a.to_owned())
        };
        *result.entry(pair).or_default() += 1;
    }
    Ok(result)
}
/// Temporally ordered event pairs, with optional first-following reduction.
/// Flow times honour the business schedule when supplied, unlike elapsed concurrency overlap.
/// Elapsed flow times retain subsecond precision; Polars truncates them to whole seconds.
pub fn get_partial_order(
    log: &EventLog,
    keys: &EventKeys,
    options: &RelationOptions,
) -> Result<Vec<EventRelation>> {
    let mut result = Vec::new();
    for (ti, t) in intervals(log, keys, &options.time)?.into_iter().enumerate() {
        for (i, a) in t.iter().enumerate() {
            for b in &t[i + 1..] {
                if a.end <= b.start {
                    result.push(EventRelation {
                        trace: ti,
                        source: a.index,
                        target: b.index,
                        duration: options.time.duration(a.end, b.start)?,
                    });
                    if options.keep_first_following {
                        break;
                    }
                }
            }
        }
    }
    Ok(result)
}
/// Frequency of ordered activity pairs where the first event completes before the second starts.
pub fn get_eventually_follows(
    log: &EventLog,
    keys: &EventKeys,
    options: &RelationOptions,
) -> Result<BTreeMap<(String, String), usize>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = BTreeMap::new();
    for pair in get_partial_order(log, keys, options)? {
        let a = seq.activities.name(seq.traces[pair.trace][pair.source]);
        let b = seq.activities.name(seq.traces[pair.trace][pair.target]);
        *result.entry((a.to_owned(), b.to_owned())).or_default() += 1;
    }
    Ok(result)
}
/// Event-order eventually-follows counts, corresponding to pm4py's timestamp-free UVCL backend.
pub fn get_eventually_follows_sequences(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<BTreeMap<(String, String), usize>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = BTreeMap::new();
    for t in &seq.traces {
        for (i, &a) in t.iter().enumerate() {
            for &b in &t[i + 1..] {
                *result
                    .entry((
                        seq.activities.name(a).to_owned(),
                        seq.activities.name(b).to_owned(),
                    ))
                    .or_default() += 1;
            }
        }
    }
    Ok(result)
}
/// Expansion applied to both endpoints before overlap queries.
#[derive(Clone, Copy, Debug)]
pub struct OverlapOptions {
    /// Reference default is 1e-5 seconds. Identical intervals are counted separately.
    pub epsilon: f64,
}
impl Default for OverlapOptions {
    fn default() -> Self {
        Self { epsilon: 1e-5 }
    }
}
/// Number of expanded intervals intersecting each input interval, including itself and duplicates.
pub fn get_overlap(points: &[(f64, f64)], options: OverlapOptions) -> Result<Vec<usize>> {
    if !options.epsilon.is_finite() || options.epsilon < 0.0 {
        return Err(Error::InvalidOption(
            "overlap epsilon must be finite and nonnegative",
        ));
    }
    let mut points = points
        .iter()
        .map(|(s, e)| (s - options.epsilon, e + options.epsilon))
        .collect::<Vec<_>>();
    if points
        .iter()
        .any(|(s, e)| !s.is_finite() || !e.is_finite() || s >= e)
    {
        return Err(Error::InvalidOption(
            "expanded intervals must be finite and nonempty",
        ));
    }
    let mut starts = points.iter().map(|(s, _)| *s).collect::<Vec<_>>();
    let mut ends = points.iter().map(|(_, e)| *e).collect::<Vec<_>>();
    starts.sort_by(f64::total_cmp);
    ends.sort_by(f64::total_cmp);
    Ok(points
        .drain(..)
        .map(|(s, e)| starts.partition_point(|v| *v < e) - ends.partition_point(|v| *v <= s))
        .collect())
}
/// Case envelopes use the earliest event start and latest completion; empty traces return zero.
pub fn get_case_overlap(
    log: &EventLog,
    keys: &EventKeys,
    time: &TimeOptions,
    options: OverlapOptions,
) -> Result<Vec<usize>> {
    let mut points = Vec::new();
    let mut indices = Vec::new();
    for (i, t) in intervals(log, keys, time)?.into_iter().enumerate() {
        if !t.is_empty() {
            points.push((
                seconds(t.iter().map(|e| e.start).min().unwrap()),
                seconds(t.iter().map(|e| e.end).max().unwrap()),
            ));
            indices.push(i);
        }
    }
    let mut result = vec![0; log.len()];
    for (i, n) in indices.into_iter().zip(get_overlap(&points, options)?) {
        result[i] = n;
    }
    Ok(result)
}
/// Event overlap across the complete log, in original trace/event order.
pub fn get_interval_event_overlap(
    log: &EventLog,
    keys: &EventKeys,
    time: &TimeOptions,
    options: OverlapOptions,
) -> Result<Vec<usize>> {
    let mut points = Vec::new();
    for (ti, t) in log.traces.iter().enumerate() {
        for (ei, e) in t.events.iter().enumerate() {
            let p = Position::Event {
                trace: ti,
                event: ei,
            };
            points.push((
                seconds(date(e, time.start_key(keys), p)?),
                seconds(date(e, &keys.timestamp, p)?),
            ));
        }
    }
    get_overlap(&points, options)
}
