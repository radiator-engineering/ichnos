//! Stable sorting by timestamp, as pm4py's `sorting` module does.

use chrono::{DateTime, FixedOffset};

use crate::error::{Error, Position, Result};
use crate::log::{Event, EventLog, EventStream, Trace};

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Earliest first.
    #[default]
    Ascending,
    /// Latest first. Equal timestamps keep their input order, as with
    /// Python's `sorted(..., reverse=True)`.
    Descending,
}

fn timestamp(event: &Event, key: &str, position: Position) -> Result<DateTime<FixedOffset>> {
    let value = event.get(key).ok_or_else(|| Error::MissingAttribute {
        key: key.to_owned(),
        position,
    })?;
    value.as_date().ok_or_else(|| Error::AttributeType {
        key: key.to_owned(),
        position,
        expected: "date",
        found: value.type_name(),
    })
}

/// Sorts `events` stably by the precomputed `stamps`.
fn reorder(events: &mut Vec<Event>, stamps: Vec<DateTime<FixedOffset>>, order: SortOrder) {
    let mut keyed: Vec<_> = stamps.into_iter().zip(events.drain(..)).collect();
    match order {
        SortOrder::Ascending => keyed.sort_by_key(|k| k.0),
        SortOrder::Descending => keyed.sort_by_key(|k| std::cmp::Reverse(k.0)),
    }
    events.extend(keyed.into_iter().map(|(_, e)| e));
}

fn trace_stamps(
    trace: &Trace,
    key: &str,
    position: impl Fn(usize) -> Position,
) -> Result<Vec<DateTime<FixedOffset>>> {
    trace
        .events
        .iter()
        .enumerate()
        .map(|(e, event)| timestamp(event, key, position(e)))
        .collect()
}

impl Trace {
    /// Sorts the events stably by the date attribute `key`. Fails without
    /// changing the trace if an event lacks a date under `key`.
    pub fn sort_by_timestamp(&mut self, key: &str, order: SortOrder) -> Result<()> {
        let stamps = trace_stamps(self, key, Position::TraceEvent)?;
        reorder(&mut self.events, stamps, order);
        Ok(())
    }
}

impl EventLog {
    /// Sorts the events of every trace stably by the date attribute `key`,
    /// then sorts the traces stably by their first event. Port of pm4py's
    /// `sort_timestamp_log`.
    ///
    /// Unlike pm4py, empty traces are kept; they go after all other traces.
    /// Fails without changing the log if an event lacks a date under `key`.
    pub fn sort_by_timestamp(&mut self, key: &str, order: SortOrder) -> Result<()> {
        let stamps: Vec<_> = self
            .traces
            .iter()
            .enumerate()
            .map(|(t, trace)| trace_stamps(trace, key, |e| Position::Event { trace: t, event: e }))
            .collect::<Result<_>>()?;
        for (trace, stamps) in self.traces.iter_mut().zip(stamps) {
            reorder(&mut trace.events, stamps, order);
        }
        let first = |t: &Trace| {
            t.events
                .first()
                .and_then(|e| e.get(key))
                .and_then(|v| v.as_date())
        };
        self.traces.sort_by(|a, b| match (first(a), first(b)) {
            (Some(x), Some(y)) => match order {
                SortOrder::Ascending => x.cmp(&y),
                SortOrder::Descending => y.cmp(&x),
            },
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        Ok(())
    }
}

impl EventStream {
    /// Sorts the events stably by the date attribute `key`. Port of pm4py's
    /// `sort_timestamp_stream`. Fails without changing the stream if an event
    /// lacks a date under `key`.
    pub fn sort_by_timestamp(&mut self, key: &str, order: SortOrder) -> Result<()> {
        let stamps = self
            .events
            .iter()
            .enumerate()
            .map(|(i, event)| timestamp(event, key, Position::StreamEvent(i)))
            .collect::<Result<_>>()?;
        reorder(&mut self.events, stamps, order);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::AttributeValue;
    use chrono::{TimeZone, Utc};

    fn event(name: &str, second: i64) -> Event {
        Event::from_iter([
            ("concept:name", AttributeValue::from(name)),
            (
                "time:timestamp",
                Utc.timestamp_opt(second, 0).unwrap().into(),
            ),
        ])
    }

    fn names(trace: &Trace) -> Vec<&str> {
        trace
            .iter()
            .map(|e| e.get("concept:name").unwrap().as_str().unwrap())
            .collect()
    }

    #[test]
    fn sort_is_stable_both_ways() {
        let mut trace = Trace {
            events: vec![event("a", 2), event("b", 1), event("c", 2), event("d", 1)],
            ..Trace::default()
        };
        trace
            .sort_by_timestamp("time:timestamp", SortOrder::Ascending)
            .unwrap();
        assert_eq!(names(&trace), ["b", "d", "a", "c"]);
        trace
            .sort_by_timestamp("time:timestamp", SortOrder::Descending)
            .unwrap();
        assert_eq!(names(&trace), ["a", "c", "b", "d"]);
    }

    #[test]
    fn log_sort_orders_traces_by_first_event_and_keeps_empty_ones() {
        let mut log = EventLog::from_traces(vec![
            Trace::with_case_id("empty"),
            Trace {
                events: vec![event("x", 9), event("y", 5)],
                ..Trace::with_case_id("late")
            },
            Trace {
                events: vec![event("z", 3)],
                ..Trace::with_case_id("early")
            },
        ]);
        log.sort_by_timestamp("time:timestamp", SortOrder::Ascending)
            .unwrap();
        let ids: Vec<_> = log
            .iter()
            .map(|t| t.case_id().unwrap().to_string())
            .collect();
        assert_eq!(ids, ["early", "late", "empty"]);
        assert_eq!(names(&log.traces[1]), ["y", "x"]);
    }

    #[test]
    fn sort_fails_without_changes_on_bad_timestamp() {
        let mut stream = EventStream {
            events: vec![
                event("a", 2),
                event("b", 1),
                Event::from_iter([("time:timestamp", "x")]),
            ],
            ..EventStream::default()
        };
        let before = stream.clone();
        let err = stream
            .sort_by_timestamp("time:timestamp", SortOrder::Ascending)
            .unwrap_err();
        assert!(matches!(
            err,
            Error::AttributeType {
                expected: "date",
                found: "string",
                ..
            }
        ));
        assert_eq!(stream, before);
    }

    #[test]
    fn trace_sort_error_names_the_event_only() {
        let mut trace = Trace {
            events: vec![event("a", 1), Event::from_iter([("concept:name", "b")])],
            ..Trace::default()
        };
        let err = trace
            .sort_by_timestamp("time:timestamp", SortOrder::Ascending)
            .unwrap_err();
        assert!(matches!(
            err,
            Error::MissingAttribute {
                position: Position::TraceEvent(1),
                ..
            }
        ));
        assert_eq!(
            err.to_string(),
            "event 1 of the trace has no attribute `time:timestamp`"
        );
    }
}
