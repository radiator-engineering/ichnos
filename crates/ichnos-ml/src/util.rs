//! Shared helpers: timestamps and the flat event table.

use chrono::{DateTime, FixedOffset};
use ichnos_core::{AttributeValue, Event, EventLog};

use crate::error::{Error, Result};

/// Python's `datetime.timestamp()`: seconds since the epoch.
pub(crate) fn seconds(d: &DateTime<FixedOffset>) -> f64 {
    d.timestamp_micros() as f64 / 1e6
}

/// The timestamp `key` of `event`, in seconds since the epoch.
pub(crate) fn event_seconds(event: &Event, key: &str, trace: usize, index: usize) -> Result<f64> {
    event_date(event, key, trace, index).map(|d| seconds(&d))
}

/// The timestamp `key` of `event`.
pub(crate) fn event_date(
    event: &Event,
    key: &str,
    trace: usize,
    index: usize,
) -> Result<DateTime<FixedOffset>> {
    let value = event.get(key).ok_or_else(|| Error::MissingAttribute {
        trace,
        event: index,
        key: key.to_owned(),
    })?;
    value.as_date().ok_or_else(|| Error::NotTimestamp {
        trace,
        event: index,
        key: key.to_owned(),
    })
}

/// The attribute `key` of `event`, as Python's `str` writes it.
pub(crate) fn event_str(event: &Event, key: &str, trace: usize, index: usize) -> Result<String> {
    event
        .get(key)
        .map(ToString::to_string)
        .ok_or_else(|| Error::MissingAttribute {
            trace,
            event: index,
            key: key.to_owned(),
        })
}

/// The log as pm4py's data frame sees it: one row per event, in log order.
///
/// A column is an event attribute, or a trace attribute under the case
/// prefix. An event attribute of the same name wins.
pub(crate) struct Table<'a> {
    log: &'a EventLog,
    prefix: &'a str,
    /// Column names: event attributes in first-seen order, then prefixed
    /// trace attributes not already present.
    pub columns: Vec<String>,
    /// `(trace, event)` positions of the rows.
    pub rows: Vec<(usize, usize)>,
}

impl<'a> Table<'a> {
    pub(crate) fn new(log: &'a EventLog, prefix: &'a str) -> Self {
        let mut columns: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut rows = Vec::new();
        for (t, trace) in log.traces.iter().enumerate() {
            for (e, event) in trace.events.iter().enumerate() {
                rows.push((t, e));
                for key in event.attributes.keys() {
                    if seen.insert(key.to_string()) {
                        columns.push(key.to_string());
                    }
                }
            }
        }
        for trace in &log.traces {
            if trace.events.is_empty() {
                continue;
            }
            for key in trace.attributes.keys() {
                let name = format!("{prefix}{key}");
                if seen.insert(name.clone()) {
                    columns.push(name);
                }
            }
        }
        Table {
            log,
            prefix,
            columns,
            rows,
        }
    }

    /// The value of `column` in `row`.
    pub(crate) fn value(&self, row: usize, column: &str) -> Option<&'a AttributeValue> {
        let (t, e) = self.rows[row];
        let trace = &self.log.traces[t];
        trace.events[e].get(column).or_else(|| {
            column
                .strip_prefix(self.prefix)
                .and_then(|k| trace.attributes.get(k))
        })
    }

    /// The event of `row`.
    pub(crate) fn event(&self, row: usize) -> &'a Event {
        let (t, e) = self.rows[row];
        &self.log.traces[t].events[e]
    }

    /// The case of each row: the string form of `case_id`. pm4py groups by
    /// this column, so a row without it is an error.
    pub(crate) fn cases(&self, case_id: &str) -> Result<Vec<String>> {
        (0..self.rows.len())
            .map(|r| {
                self.value(r, case_id)
                    .map(ToString::to_string)
                    .ok_or_else(|| Error::MissingAttribute {
                        trace: self.rows[r].0,
                        event: self.rows[r].1,
                        key: case_id.to_owned(),
                    })
            })
            .collect()
    }

    /// The pandas kind of a column, from its non-null values.
    pub(crate) fn kind(&self, column: &str) -> ColumnKind {
        let mut numeric = true;
        let mut dates = true;
        let mut bools = true;
        let mut missing = false;
        for r in 0..self.rows.len() {
            match self.value(r, column).map(AttributeValue::plain) {
                None => missing = true,
                Some(AttributeValue::Int(_) | AttributeValue::Float(_)) => {
                    dates = false;
                    bools = false;
                }
                Some(AttributeValue::Date(_)) => {
                    numeric = false;
                    bools = false;
                }
                Some(AttributeValue::Bool(_)) => {
                    numeric = false;
                    dates = false;
                }
                Some(_) => {
                    numeric = false;
                    dates = false;
                    bools = false;
                }
            }
        }
        if numeric {
            ColumnKind::Numeric
        } else if dates || (bools && !missing) {
            ColumnKind::Other
        } else {
            ColumnKind::Text
        }
    }
}

/// How pandas' dtype of a column decides its features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnKind {
    /// `int` or `float`.
    Numeric,
    /// `str`, `string` or `object`.
    Text,
    /// Dates and booleans, which pm4py leaves out.
    Other,
}

/// The arithmetic mean, adding in order; NaN when empty.
pub(crate) fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// The median, averaging the two middle values of an even count.
pub(crate) fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}
