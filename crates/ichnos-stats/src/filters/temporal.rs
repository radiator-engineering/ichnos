use super::{Retention, select};
use crate::{
    Error, Result,
    time::{date, seconds},
};
use ichnos_core::{
    EventKeys, EventLog, Position,
    chrono::{DateTime, FixedOffset},
};

/// Inclusive duration limits in seconds.
#[derive(Clone, Copy, Debug)]
pub struct DurationRange {
    /// Minimum accepted duration.
    pub minimum: f64,
    /// Maximum accepted duration.
    pub maximum: f64,
}
impl DurationRange {
    fn validate(self) -> Result<()> {
        if !self.minimum.is_finite() || !self.maximum.is_finite() || self.minimum > self.maximum {
            return Err(Error::InvalidOption(
                "duration bounds must be finite and ordered",
            ));
        }
        Ok(())
    }
    fn contains(self, value: f64) -> bool {
        (self.minimum..=self.maximum).contains(&value)
    }
}
/// Retain cases with an inclusive number of events; empty cases can be retained.
pub fn filter_case_size(log: &EventLog, minimum: usize, maximum: usize) -> Result<EventLog> {
    if minimum > maximum {
        return Err(Error::InvalidOption("case size bounds must be ordered"));
    }
    Ok(log.filter_traces(|t| (minimum..=maximum).contains(&t.len())))
}
/// Retain nonempty cases whose last-minus-first completion time falls in the inclusive range.
pub fn filter_case_performance(
    log: &EventLog,
    keys: &EventKeys,
    range: DurationRange,
) -> Result<EventLog> {
    range.validate()?;
    select(log, |i, t| {
        if t.is_empty() {
            return Ok(false);
        }
        let a = date(
            &t.events[0],
            &keys.timestamp,
            Position::Event { trace: i, event: 0 },
        )?;
        let b = date(
            &t.events[t.len() - 1],
            &keys.timestamp,
            Position::Event {
                trace: i,
                event: t.len() - 1,
            },
        )?;
        let delta = b.signed_duration_since(a);
        let whole = delta.num_seconds();
        let remainder = (delta - ichnos_core::chrono::Duration::seconds(whole))
            .num_nanoseconds()
            .unwrap();
        Ok(range.contains(whole as f64 + remainder as f64 / 1e9))
    })
}
/// Filter cases containing an adjacent path with completion-to-completion time in the inclusive range.
pub fn filter_paths_performance(
    log: &EventLog,
    keys: &EventKeys,
    path: (&str, &str),
    range: DurationRange,
    retention: Retention,
) -> Result<EventLog> {
    range.validate()?;
    let seq = log.activity_sequences(keys)?;
    select(log, |i, t| {
        let mut found = false;
        for (j, w) in seq.traces[i].windows(2).enumerate() {
            if seq.activities.name(w[0]) == path.0 && seq.activities.name(w[1]) == path.1 {
                let a = date(
                    &t.events[j],
                    &keys.timestamp,
                    Position::Event { trace: i, event: j },
                )?;
                let b = date(
                    &t.events[j + 1],
                    &keys.timestamp,
                    Position::Event {
                        trace: i,
                        event: j + 1,
                    },
                )?;
                if range.contains(seconds(b) - seconds(a)) {
                    found = true;
                    break;
                }
            }
        }
        Ok(retention.accepts(found))
    })
}
/// Scope and endpoint rule for time-range filtering.
#[derive(Clone, Copy, Debug, Default)]
pub enum TimeRangeMode {
    /// Filter individual events, dropping empty cases.
    #[default]
    Events,
    /// First timestamp is at least the lower bound and last is at most the upper bound.
    Contained,
    /// Case endpoint interval intersects the time range.
    Intersecting,
    /// First completion timestamp lies within the time range.
    Starting,
    /// Last completion timestamp lies within the time range.
    Completing,
}
/// Inclusive timestamp range and retention options.
#[derive(Clone, Copy, Debug)]
pub struct TimeRange {
    /// Inclusive lower bound.
    pub start: DateTime<FixedOffset>,
    /// Inclusive upper bound.
    pub end: DateTime<FixedOffset>,
    /// Filter rule.
    pub mode: TimeRangeMode,
    /// Retain or exclude matches; pm4py exposes exclusion for start/completion modes.
    pub retention: Retention,
}
/// Filter by completion timestamps in input order; endpoint modes do not sort the trace.
pub fn filter_time_range(log: &EventLog, keys: &EventKeys, range: TimeRange) -> Result<EventLog> {
    if range.start > range.end {
        return Err(Error::InvalidOption("time bounds must be ordered"));
    }
    let contains = |d| range.start <= d && d <= range.end;
    if matches!(range.mode, TimeRangeMode::Events) {
        let mut result = log.filter_traces(|_| false);
        for (i, t) in log.traces.iter().enumerate() {
            let mut events = Vec::new();
            for (j, e) in t.events.iter().enumerate() {
                if range.retention.accepts(contains(date(
                    e,
                    &keys.timestamp,
                    Position::Event { trace: i, event: j },
                )?)) {
                    events.push(e.clone());
                }
            }
            if !events.is_empty() {
                result.traces.push(ichnos_core::Trace {
                    attributes: t.attributes.clone(),
                    events,
                });
            }
        }
        return Ok(result);
    }
    select(log, |i, t| {
        let matched = if t.is_empty() {
            false
        } else {
            let first = || {
                date(
                    &t.events[0],
                    &keys.timestamp,
                    Position::Event { trace: i, event: 0 },
                )
            };
            let last = || {
                date(
                    &t.events[t.len() - 1],
                    &keys.timestamp,
                    Position::Event {
                        trace: i,
                        event: t.len() - 1,
                    },
                )
            };
            match range.mode {
                TimeRangeMode::Starting => contains(first()?),
                TimeRangeMode::Completing => contains(last()?),
                TimeRangeMode::Contained => first()? >= range.start && last()? <= range.end,
                TimeRangeMode::Intersecting => {
                    let a = first()?;
                    let b = last()?;
                    contains(a)
                        || contains(b)
                        || (a <= range.start && range.start <= b)
                        || (a <= range.end && range.end <= b)
                }
                TimeRangeMode::Events => unreachable!(),
            }
        };
        Ok(range.retention.accepts(matched))
    })
}
