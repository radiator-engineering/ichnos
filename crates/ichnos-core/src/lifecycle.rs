//! Conversion between lifecycle logs (one event per transition) and interval
//! logs (one event with a start and an end timestamp). Port of pm4py's
//! `objects/log/util/interval_lifecycle.py`.

use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use rustc_hash::FxHashMap;

use crate::attribute::AttributeValue;
use crate::error::{Error, Position, Result};
use crate::keys::{self, EventKeys};
use crate::log::{CaseKey, Event, EventLog, Trace};
use crate::sort::SortOrder;

/// Log attribute that marks a log as `interval` or `lifecycle`, as in pm4py.
pub const LOG_TYPE_KEY: &str = "PM4PY_TYPE";
/// Prefix for the start event's attributes in an interval event.
pub const START_EVENT_PREFIX: &str = "@@startevent_";
/// Interval event attribute: duration in seconds.
pub const DURATION_KEY: &str = "@@duration";
/// Lifecycle event attribute: 0 for the start event, 1 for the complete event.
pub const LIFECYCLE_ID_KEY: &str = "@@custom_lif_id";
/// Lifecycle event attribute: index of the interval event it came from.
pub const ORIGIN_EVENT_KEY: &str = "@@origin_ev_idx";

/// A value as a hash key, as Python compares dict keys.
fn hashable(key: &str, value: &AttributeValue) -> Result<CaseKey> {
    CaseKey::new(value).ok_or_else(|| Error::NestedAttribute {
        key: key.to_owned(),
        kind: value.type_name(),
    })
}

fn date(event: &Event, key: &str, position: Position) -> Result<DateTime<FixedOffset>> {
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

/// Seconds between two dates, at microsecond precision, as Python's
/// `timedelta.total_seconds()` gives.
fn seconds_between(start: DateTime<FixedOffset>, end: DateTime<FixedOffset>) -> f64 {
    let delta = end - start;
    match delta.num_microseconds() {
        Some(us) => us as f64 / 1e6,
        None => delta.num_seconds() as f64,
    }
}

fn header_like(log: &EventLog, log_type: &str) -> EventLog {
    let mut out = EventLog {
        attributes: log.attributes.clone(),
        extensions: log.extensions.clone(),
        globals: log.globals.clone(),
        classifiers: log.classifiers.clone(),
        traces: Vec::with_capacity(log.traces.len()),
    };
    out.attributes.insert(LOG_TYPE_KEY, log_type);
    out
}

/// Sort key of a lifecycle event: (timestamp, origin index, lifecycle ID).
type LifecycleKey = (DateTime<FixedOffset>, usize, u8);

fn first_event(log: &EventLog) -> Option<&Event> {
    log.traces.first().and_then(|t| t.events.first())
}

fn is_type(log: &EventLog, log_type: &str) -> bool {
    log.attributes
        .get(LOG_TYPE_KEY)
        .and_then(AttributeValue::as_str)
        == Some(log_type)
}

impl EventLog {
    /// Converts a lifecycle log to an interval log. Port of pm4py's
    /// `to_interval`.
    ///
    /// Each `complete` event (or event without `keys.transition`) becomes one
    /// interval event. It is paired with the oldest unmatched `start` event of
    /// the same activity and `concept:instance`, compared as values with
    /// Python dict semantics (the int `1` and the string `"1"` differ). The
    /// interval event keeps the
    /// complete event's attributes, gets the start event's attributes under
    /// [`START_EVENT_PREFIX`], `keys.start_timestamp`, `keys.timestamp` and
    /// [`DURATION_KEY`] in seconds. An unpaired complete event starts when it
    /// ends. Events with other transitions are dropped. Each trace is then
    /// sorted by start timestamp. The log gets `PM4PY_TYPE = interval`.
    ///
    /// Returns a copy unchanged if the log is empty, is already marked
    /// `interval`, or its first event has `keys.start_timestamp`. Fails if an
    /// activity or instance is a list or container, which pm4py cannot hash
    /// either. pm4py's business-hours option is not ported here.
    pub fn to_interval(&self, keys: &EventKeys) -> Result<EventLog> {
        if self.is_empty()
            || is_type(self, "interval")
            || first_event(self).is_some_and(|e| e.get(&keys.start_timestamp).is_some())
        {
            return Ok(self.clone());
        }
        let mut out = header_like(self, "interval");
        let start_key: Arc<str> = keys.start_timestamp.as_str().into();
        let end_key: Arc<str> = keys.timestamp.as_str().into();
        for (t, trace) in self.traces.iter().enumerate() {
            let mut new_trace = Trace {
                attributes: trace.attributes.clone(),
                events: Vec::new(),
            };
            let mut open: FxHashMap<(CaseKey, Option<CaseKey>), VecDeque<&Event>> =
                FxHashMap::default();
            for (e, event) in trace.events.iter().enumerate() {
                let position = Position::Event { trace: t, event: e };
                let activity =
                    event
                        .get(&keys.activity)
                        .ok_or_else(|| Error::MissingAttribute {
                            key: keys.activity.clone(),
                            position,
                        })?;
                let instance = event
                    .get(keys::CONCEPT_INSTANCE)
                    .map(|v| hashable(keys::CONCEPT_INSTANCE, v))
                    .transpose()?;
                let slot = (hashable(&keys.activity, activity)?, instance);
                let transition = event
                    .get(&keys.transition)
                    .map_or_else(|| "complete".to_owned(), |v| v.to_string().to_lowercase());
                let end = date(event, &keys.timestamp, position)?;
                match transition.as_str() {
                    "start" => open.entry(slot).or_default().push_back(event),
                    "complete" => {
                        let start_event = open.get_mut(&slot).and_then(VecDeque::pop_front);
                        let start = match start_event {
                            Some(s) => date(s, &keys.timestamp, position)?,
                            None => end,
                        };
                        let skip = |k: &str| k == keys.timestamp || k == keys.transition;
                        let mut new_event = Event::new();
                        for (k, v) in &event.attributes {
                            if !skip(k) {
                                new_event.insert(k.clone(), v.clone());
                            }
                        }
                        for (k, v) in start_event.into_iter().flat_map(|s| &s.attributes) {
                            if !skip(k) {
                                new_event.insert(format!("{START_EVENT_PREFIX}{k}"), v.clone());
                            }
                        }
                        new_event.insert(start_key.clone(), start);
                        new_event.insert(end_key.clone(), end);
                        new_event.insert(DURATION_KEY, seconds_between(start, end));
                        new_trace.events.push(new_event);
                    }
                    _ => {}
                }
            }
            new_trace.sort_by_timestamp(&keys.start_timestamp, SortOrder::Ascending)?;
            out.traces.push(new_trace);
        }
        Ok(out)
    }

    /// Converts an interval log to a lifecycle log. Port of pm4py's
    /// `to_lifecycle`.
    ///
    /// Each event becomes a `start` event at `keys.start_timestamp` and a
    /// `complete` event at `keys.timestamp`, both with the other attributes,
    /// [`LIFECYCLE_ID_KEY`] (0 or 1) and [`ORIGIN_EVENT_KEY`]. Each trace is
    /// sorted by (timestamp, origin index, lifecycle ID). The log gets
    /// `PM4PY_TYPE = lifecycle`.
    ///
    /// Returns a copy unchanged if the log is empty, is already marked
    /// `lifecycle`, or its first event has `keys.transition`.
    pub fn to_lifecycle(&self, keys: &EventKeys) -> Result<EventLog> {
        if self.is_empty()
            || is_type(self, "lifecycle")
            || first_event(self).is_some_and(|e| e.get(&keys.transition).is_some())
        {
            return Ok(self.clone());
        }
        let mut out = header_like(self, "lifecycle");
        let ts_key: Arc<str> = keys.timestamp.as_str().into();
        let tr_key: Arc<str> = keys.transition.as_str().into();
        for (t, trace) in self.traces.iter().enumerate() {
            let mut keyed: Vec<(LifecycleKey, Event)> = Vec::with_capacity(trace.events.len() * 2);
            for (e, event) in trace.events.iter().enumerate() {
                let position = Position::Event { trace: t, event: e };
                let start = date(event, &keys.start_timestamp, position)?;
                let end = date(event, &keys.timestamp, position)?;
                for (lif, at, transition) in [(0u8, start, "start"), (1u8, end, "complete")] {
                    let mut new_event = Event::new();
                    for (k, v) in &event.attributes {
                        if **k != *keys.timestamp && **k != *keys.start_timestamp {
                            new_event.insert(k.clone(), v.clone());
                        }
                    }
                    new_event.insert(ts_key.clone(), at);
                    new_event.insert(tr_key.clone(), transition);
                    new_event.insert(LIFECYCLE_ID_KEY, i64::from(lif));
                    new_event.insert(ORIGIN_EVENT_KEY, e as i64);
                    keyed.push(((at, e, lif), new_event));
                }
            }
            keyed.sort_by_key(|(k, _)| *k);
            out.traces.push(Trace {
                attributes: trace.attributes.clone(),
                events: keyed.into_iter().map(|(_, e)| e).collect(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn ev(act: &str, transition: Option<&str>, second: i64) -> Event {
        let mut e = Event::from_iter([("concept:name", act)]);
        if let Some(t) = transition {
            e.insert("lifecycle:transition", t);
        }
        e.insert("time:timestamp", Utc.timestamp_opt(second, 0).unwrap());
        e
    }

    fn lifecycle_log() -> EventLog {
        EventLog::from_traces(vec![Trace {
            events: vec![
                ev("a", Some("start"), 0),
                ev("b", Some("start"), 1),
                ev("a", Some("start"), 2),
                ev("b", Some("COMPLETE"), 5),
                ev("a", Some("complete"), 6),
                ev("c", None, 7),
                ev("a", Some("suspend"), 8),
                ev("a", Some("complete"), 9),
            ],
            ..Trace::with_case_id("1")
        }])
    }

    fn get<'a>(e: &'a Event, k: &str) -> &'a AttributeValue {
        e.get(k).unwrap()
    }

    #[test]
    fn to_interval_pairs_oldest_start() {
        let keys = EventKeys::default();
        let log = lifecycle_log().to_interval(&keys).unwrap();
        assert_eq!(
            log.attributes.get(LOG_TYPE_KEY).unwrap().as_str(),
            Some("interval")
        );
        let events = &log.traces[0].events;
        let acts: Vec<String> = events
            .iter()
            .map(|e| get(e, "concept:name").to_string())
            .collect();
        // Sorted by start: a(0..6), b(1..5), a(2..9), c(7..7).
        assert_eq!(acts, ["a", "b", "a", "c"]);
        assert_eq!(get(&events[0], "@@duration").as_f64(), Some(6.0));
        assert_eq!(get(&events[2], "@@duration").as_f64(), Some(7.0));
        assert_eq!(get(&events[3], "@@duration").as_f64(), Some(0.0));
        assert_eq!(
            get(&events[1], "@@startevent_concept:name").as_str(),
            Some("b")
        );
        assert!(events[3].get("@@startevent_concept:name").is_none());
        assert!(
            events
                .iter()
                .all(|e| e.get("lifecycle:transition").is_none())
        );
        // Already interval: unchanged.
        assert_eq!(log.to_interval(&keys).unwrap(), log);
    }

    #[test]
    fn to_lifecycle_splits_and_orders() {
        let keys = EventKeys::default();
        let interval = lifecycle_log().to_interval(&keys).unwrap();
        let log = interval.to_lifecycle(&keys).unwrap();
        // The interval log is marked `interval`; to_lifecycle overwrites the mark.
        assert_eq!(
            log.attributes.get(LOG_TYPE_KEY).unwrap().as_str(),
            Some("lifecycle")
        );
        let events = &log.traces[0].events;
        assert_eq!(events.len(), 8);
        let summary: Vec<String> = events
            .iter()
            .map(|e| {
                format!(
                    "{}:{}",
                    get(e, "concept:name"),
                    get(e, "lifecycle:transition")
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                "a:start",
                "b:start",
                "a:start",
                "b:complete",
                "a:complete",
                "c:start",
                "c:complete",
                "a:complete"
            ]
        );
        assert!(events.iter().all(|e| e.get("start_timestamp").is_none()));
        assert_eq!(get(&events[6], "@@origin_ev_idx").as_i64(), Some(3));
        assert_eq!(get(&events[6], "@@custom_lif_id").as_i64(), Some(1));
    }

    #[test]
    fn missing_timestamp_is_an_error() {
        let keys = EventKeys::default();
        let mut log = lifecycle_log();
        log.traces[0].events[1].attributes.remove("time:timestamp");
        assert!(matches!(
            log.to_interval(&keys),
            Err(Error::MissingAttribute {
                position: Position::Event { trace: 0, event: 1 },
                ..
            })
        ));
    }

    #[test]
    fn to_interval_pairs_by_value_not_text() {
        let keys = EventKeys::default();
        let mut start = ev("a", Some("start"), 0);
        start.insert("concept:instance", 1);
        let mut complete = ev("a", Some("complete"), 5);
        complete.insert("concept:instance", "1");
        let log = EventLog::from_traces(vec![Trace {
            events: vec![start, complete],
            ..Trace::with_case_id("1")
        }]);
        let out = log.to_interval(&keys).unwrap();
        let event = &out.traces[0].events[0];
        // The int 1 and the string "1" differ, so the complete event is unpaired.
        assert_eq!(get(event, "@@duration").as_f64(), Some(0.0));
    }
}
