//! Artificial start and end events. Port of pm4py's
//! `insert_artificial_start_end` for event logs
//! (`objects/log/util/artificial.py`).

use std::sync::Arc;

use chrono::TimeDelta;

use crate::attribute::AttributeValue;
use crate::error::{Error, Position, Result};
use crate::keys::EventKeys;
use crate::log::{Event, EventLog};

/// pm4py's default artificial start activity.
pub const ARTIFICIAL_START: &str = "▶";
/// pm4py's default artificial end activity.
pub const ARTIFICIAL_END: &str = "■";

impl EventLog {
    /// Adds an event with activity `start` at the front of every trace and one
    /// with activity `end` at the back. pm4py's defaults are
    /// [`ARTIFICIAL_START`] and [`ARTIFICIAL_END`].
    ///
    /// If the first event has a timestamp under `keys.timestamp`, the start
    /// event gets that timestamp minus one second; the end event likewise
    /// gets the last event's timestamp plus one second. Empty traces get both
    /// events without timestamps. Fails without changing the log if a first
    /// or last event has a `keys.timestamp` attribute that is not a date.
    pub fn insert_artificial_start_end(
        &mut self,
        keys: &EventKeys,
        start: &str,
        end: &str,
    ) -> Result<()> {
        let one_second = TimeDelta::seconds(1);
        let mut shifted = Vec::with_capacity(self.traces.len());
        for (t, trace) in self.traces.iter().enumerate() {
            let last = trace.events.len().saturating_sub(1);
            let at = |e: usize, delta: TimeDelta| -> Result<Option<AttributeValue>> {
                let Some(value) = trace.events.get(e).and_then(|ev| ev.get(&keys.timestamp)) else {
                    return Ok(None);
                };
                let date = value.as_date().ok_or_else(|| Error::AttributeType {
                    key: keys.timestamp.clone(),
                    position: Position::Event { trace: t, event: e },
                    expected: "date",
                    found: value.type_name(),
                })?;
                Ok(Some((date + delta).into()))
            };
            shifted.push((at(0, -one_second)?, at(last, one_second)?));
        }

        let activity: Arc<str> = keys.activity.as_str().into();
        let timestamp: Arc<str> = keys.timestamp.as_str().into();
        let event = |name: &str, at: Option<AttributeValue>| {
            let mut event = Event::new();
            event.insert(activity.clone(), name);
            if let Some(at) = at {
                event.insert(timestamp.clone(), at);
            }
            event
        };
        for (trace, (first, last)) in self.traces.iter_mut().zip(shifted) {
            trace.events.insert(0, event(start, first));
            trace.events.push(event(end, last));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::Trace;

    #[test]
    fn wraps_every_trace_with_shifted_timestamps() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B"], ",", &keys);
        log.traces.push(Trace::with_case_id("empty"));
        log.insert_artificial_start_end(&keys, ARTIFICIAL_START, ARTIFICIAL_END)
            .unwrap();

        let trace = &log.traces[0];
        let names: Vec<String> = trace
            .iter()
            .map(|e| e.get("concept:name").unwrap().to_string())
            .collect();
        assert_eq!(names, ["▶", "A", "B", "■"]);
        let secs: Vec<i64> = trace
            .iter()
            .map(|e| {
                e.get("time:timestamp")
                    .unwrap()
                    .as_date()
                    .unwrap()
                    .timestamp()
            })
            .collect();
        assert_eq!(secs, [9_999_999, 10_000_000, 10_000_001, 10_000_002]);

        let empty = &log.traces[1];
        assert_eq!(empty.len(), 2);
        assert!(empty.iter().all(|e| e.get("time:timestamp").is_none()));
    }

    #[test]
    fn non_date_timestamp_fails_without_changes() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B"], ",", &keys);
        log.traces[0].events[1].insert("time:timestamp", "late");
        let before = log.clone();
        let err = log
            .insert_artificial_start_end(&keys, ARTIFICIAL_START, ARTIFICIAL_END)
            .unwrap_err();
        assert!(matches!(
            err,
            Error::AttributeType {
                position: Position::Event { trace: 0, event: 1 },
                ..
            }
        ));
        assert_eq!(log, before);
    }
}
