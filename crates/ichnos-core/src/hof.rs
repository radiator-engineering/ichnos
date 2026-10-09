//! Filtering and sorting with closures. Port of pm4py's `hof` module
//! (`filter_log`, `filter_trace`, `sort_log`, `sort_trace`).
//!
//! The filters return a new log, stream or trace and copy the metadata, as
//! pm4py does. The sorts work in place, like
//! [`sort_by_timestamp`](EventLog::sort_by_timestamp), and are stable in both
//! orders, like Python's `sorted`. With owned data, `log.traces.retain(...)`
//! filters in place.

use std::cmp::Reverse;

use crate::log::{Event, EventLog, EventStream, Trace};
use crate::sort::SortOrder;

fn sort_by_key<T, K: Ord>(items: &mut [T], mut key: impl FnMut(&T) -> K, order: SortOrder) {
    match order {
        SortOrder::Ascending => items.sort_by_key(|item| key(item)),
        SortOrder::Descending => items.sort_by_key(|item| Reverse(key(item))),
    }
}

impl EventLog {
    /// A log with the traces for which `keep` returns `true`, in log order.
    /// The log metadata is copied. Port of pm4py's `hof.filter_log`.
    pub fn filter_traces(&self, mut keep: impl FnMut(&Trace) -> bool) -> EventLog {
        EventLog {
            attributes: self.attributes.clone(),
            extensions: self.extensions.clone(),
            globals: self.globals.clone(),
            classifiers: self.classifiers.clone(),
            traces: self.traces.iter().filter(|t| keep(t)).cloned().collect(),
        }
    }

    /// Sorts the traces stably by `key`. Port of pm4py's `hof.sort_log`.
    pub fn sort_traces_by_key<K: Ord>(&mut self, key: impl FnMut(&Trace) -> K, order: SortOrder) {
        sort_by_key(&mut self.traces, key, order);
    }
}

impl EventStream {
    /// A stream with the events for which `keep` returns `true`, in stream
    /// order. The metadata is copied. Port of pm4py's `hof.filter_log` on an
    /// event stream.
    pub fn filter_events(&self, mut keep: impl FnMut(&Event) -> bool) -> EventStream {
        EventStream {
            attributes: self.attributes.clone(),
            extensions: self.extensions.clone(),
            globals: self.globals.clone(),
            classifiers: self.classifiers.clone(),
            events: self.events.iter().filter(|e| keep(e)).cloned().collect(),
        }
    }

    /// Sorts the events stably by `key`. Port of pm4py's `hof.sort_log` on an
    /// event stream.
    pub fn sort_events_by_key<K: Ord>(&mut self, key: impl FnMut(&Event) -> K, order: SortOrder) {
        sort_by_key(&mut self.events, key, order);
    }
}

impl Trace {
    /// A trace with the events for which `keep` returns `true`, in trace
    /// order, and the same trace attributes. Port of pm4py's
    /// `hof.filter_trace`.
    pub fn filter_events(&self, mut keep: impl FnMut(&Event) -> bool) -> Trace {
        Trace {
            attributes: self.attributes.clone(),
            events: self.events.iter().filter(|e| keep(e)).cloned().collect(),
        }
    }

    /// Sorts the events stably by `key`. Port of pm4py's `hof.sort_trace`,
    /// except that the trace keeps its attributes; pm4py's drops them.
    pub fn sort_events_by_key<K: Ord>(&mut self, key: impl FnMut(&Event) -> K, order: SortOrder) {
        sort_by_key(&mut self.events, key, order);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::EventKeys;

    fn names(trace: &Trace) -> Vec<&str> {
        trace
            .iter()
            .map(|e| e.get("concept:name").unwrap().as_str().unwrap())
            .collect()
    }

    fn case_ids(log: &EventLog) -> Vec<String> {
        log.iter()
            .map(|t| t.case_id().unwrap().to_string())
            .collect()
    }

    #[test]
    fn filters_copy_metadata_and_keep_order() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B,C", "A", "B,C"], ",", &keys);
        log.attributes.insert("source", "test");
        let long = log.filter_traces(|t| t.len() > 1);
        assert_eq!(case_ids(&long), ["0", "2"]);
        assert_eq!(long.attributes, log.attributes);

        let trace =
            log.traces[0].filter_events(|e| e.get("concept:name").unwrap().as_str() != Some("B"));
        assert_eq!(names(&trace), ["A", "C"]);
        assert_eq!(trace.attributes, log.traces[0].attributes);

        let stream = log.to_event_stream(&keys);
        assert_eq!(
            stream
                .filter_events(|e| e.get("concept:name").unwrap().as_str() == Some("C"))
                .len(),
            2
        );
    }

    #[test]
    fn sorts_are_stable_both_ways() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B", "C", "D,E", "F"], ",", &keys);
        log.sort_traces_by_key(Trace::len, SortOrder::Descending);
        assert_eq!(case_ids(&log), ["0", "2", "1", "3"]);
        log.sort_traces_by_key(Trace::len, SortOrder::Ascending);
        assert_eq!(case_ids(&log), ["1", "3", "0", "2"]);

        let mut trace = log.traces[2].clone();
        trace.sort_events_by_key(
            |e| e.get("concept:name").unwrap().to_string(),
            SortOrder::Descending,
        );
        assert_eq!(names(&trace), ["B", "A"]);
        assert_eq!(trace.case_id(), log.traces[2].case_id());
    }
}
