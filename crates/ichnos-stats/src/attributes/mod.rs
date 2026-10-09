mod kde;
pub use kde::*;

use crate::{Error, Result};
use ichnos_core::{AttributeValue, EventKeys, EventLog};
use indexmap::IndexMap;
use std::collections::BTreeSet;

/// Hashable scalar attribute, preserving its type. Numbers follow Python equality.
#[derive(Clone, Debug)]
pub enum Scalar {
    /// String or XES identifier.
    String(String),
    /// Integral number.
    Int(i64),
    /// Boolean, equal to the corresponding integer under Python equality.
    Bool(bool),
    /// Floating point number, represented by its bits.
    Float(u64),
    /// UTC timestamp in seconds and nanoseconds.
    Date(i64, u32),
}
#[derive(PartialEq, Eq, Hash)]
enum ScalarKey<'a> {
    String(&'a str),
    Int(i64),
    Float(u64),
    Date(i64, u32),
}
impl Scalar {
    fn equality_key(&self) -> ScalarKey<'_> {
        match self {
            Self::String(v) => ScalarKey::String(v),
            Self::Bool(v) => ScalarKey::Int(i64::from(*v)),
            Self::Int(v) => ScalarKey::Int(*v),
            Self::Float(bits) => {
                let v = f64::from_bits(*bits);
                if v.is_finite()
                    && v.fract() == 0.0
                    && v >= i64::MIN as f64
                    && v < -(i64::MIN as f64)
                {
                    ScalarKey::Int(v as i64)
                } else {
                    ScalarKey::Float(*bits)
                }
            }
            Self::Date(s, ns) => ScalarKey::Date(*s, *ns),
        }
    }
}
impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        self.equality_key() == other.equality_key()
    }
}
impl Eq for Scalar {}
impl std::hash::Hash for Scalar {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.equality_key(), state);
    }
}
impl Scalar {
    /// Convert a scalar, looking through meta-attributes.
    pub fn from_value(value: &AttributeValue) -> Option<Self> {
        if let Some(v) = value.as_str() {
            return Some(Self::String(v.to_owned()));
        }
        if let Some(v) = value.as_bool() {
            return Some(Self::Bool(v));
        }
        if let Some(v) = value.as_i64() {
            return Some(Self::Int(v));
        }
        if let Some(v) = value.as_f64() {
            return Some(Self::Float(v.to_bits()));
        }
        value
            .as_date()
            .map(|v| Self::Date(v.timestamp(), v.timestamp_subsec_nanos()))
    }
}
/// Count each value once per trace rather than once per event.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttributeCountOptions {
    /// Whether duplicate values within a trace are ignored.
    pub keep_once_per_case: bool,
}
/// Event attribute names, excluding the lifecycle transition as pm4py does.
pub fn get_event_attributes(log: &EventLog) -> BTreeSet<String> {
    log.traces
        .iter()
        .flat_map(|t| &t.events)
        .flat_map(|e| e.attributes.iter())
        .filter(|(k, _)| k.as_ref() != "lifecycle:transition")
        .map(|(k, _)| k.to_string())
        .collect()
}
/// Trace attribute names, excluding the trace identifier.
pub fn get_trace_attributes(log: &EventLog) -> BTreeSet<String> {
    log.traces
        .iter()
        .flat_map(|t| t.attributes.iter())
        .filter(|(k, _)| k.as_ref() != "concept:name")
        .map(|(k, _)| k.to_string())
        .collect()
}
/// Count values of an event attribute; missing attributes are skipped.
pub fn get_event_attribute_values(
    log: &EventLog,
    attribute: &str,
    options: AttributeCountOptions,
) -> Result<IndexMap<Scalar, usize>> {
    let mut counts = IndexMap::new();
    for trace in &log.traces {
        let mut seen = std::collections::HashSet::new();
        for event in &trace.events {
            if let Some(value) = event.get(attribute) {
                let value = Scalar::from_value(value).ok_or_else(|| Error::NonScalar {
                    key: attribute.to_owned(),
                })?;
                if !options.keep_once_per_case || seen.insert(value.clone()) {
                    *counts.entry(value).or_default() += 1;
                }
            }
        }
    }
    Ok(counts)
}
/// Count values of a trace attribute; missing attributes are skipped.
pub fn get_trace_attribute_values(
    log: &EventLog,
    attribute: &str,
) -> Result<IndexMap<Scalar, usize>> {
    let mut counts = IndexMap::new();
    for trace in &log.traces {
        if let Some(value) = trace.attributes.get(attribute) {
            let value = Scalar::from_value(value).ok_or_else(|| Error::NonScalar {
                key: attribute.to_owned(),
            })?;
            *counts.entry(value).or_default() += 1;
        }
    }
    Ok(counts)
}
fn endpoints(log: &EventLog, keys: &EventKeys, start: bool) -> Result<IndexMap<String, usize>> {
    let sequences = log.activity_sequences(keys)?;
    let mut counts = IndexMap::new();
    for trace in &sequences.traces {
        if let Some(&a) = if start { trace.first() } else { trace.last() } {
            *counts
                .entry(sequences.activities.name(a).to_owned())
                .or_default() += 1;
        }
    }
    Ok(counts)
}
/// Count the first activity of each nonempty trace.
pub fn get_start_activities(log: &EventLog, keys: &EventKeys) -> Result<IndexMap<String, usize>> {
    endpoints(log, keys, true)
}
/// Count the last activity of each nonempty trace.
pub fn get_end_activities(log: &EventLog, keys: &EventKeys) -> Result<IndexMap<String, usize>> {
    endpoints(log, keys, false)
}
/// Stable descending count order, keeping insertion order on ties.
pub fn get_sorted_attributes_list<K: Clone>(counts: &IndexMap<K, usize>) -> Vec<(K, usize)> {
    let mut result: Vec<_> = counts.iter().map(|(k, &v)| (k.clone(), v)).collect();
    result.sort_by_key(|a| std::cmp::Reverse(a.1));
    result
}
/// pm4py's decreasing-factor cutoff, including its inclusive maximum index.
pub fn get_attributes_threshold<K>(
    sorted: &[(K, usize)],
    factor: f64,
    minimum: usize,
    maximum: usize,
) -> Result<usize> {
    if sorted.is_empty() {
        return Err(Error::InvalidOption("empty counts"));
    }
    let index = minimum.saturating_sub(1).min(sorted.len() - 1);
    let mut threshold = sorted[index].1;
    for (i, (_, count)) in sorted.iter().enumerate().skip(index + 1) {
        if *count as f64 > threshold as f64 * factor {
            threshold = *count;
        }
        if i >= maximum {
            break;
        }
    }
    Ok(threshold)
}
/// Decreasing-factor cutoff for start or end activity counts.
pub fn get_activities_threshold<K>(sorted: &[(K, usize)], factor: f64) -> Result<usize> {
    get_attributes_threshold(sorted, factor, 1, usize::MAX)
}
/// Start activity cutoff.
pub use get_activities_threshold as get_start_activities_threshold;
/// End activity cutoff.
pub use get_activities_threshold as get_end_activities_threshold;
/// Event attribute counting.
pub use get_event_attribute_values as get_attribute_values;
/// Event attribute enumeration.
pub use get_event_attributes as get_all_event_attributes_from_log;
/// Stable descending start activity counts.
pub use get_sorted_attributes_list as get_sorted_start_activities_list;
/// Stable descending end activity counts.
pub use get_sorted_attributes_list as get_sorted_end_activities_list;
/// Trace attribute enumeration.
pub use get_trace_attributes as get_all_trace_attributes_from_log;
/// Whether each trace has at least one event carrying an attribute.
pub fn verify_if_event_attribute_is_in_each_trace(log: &EventLog, attribute: &str) -> bool {
    log.traces
        .iter()
        .all(|t| t.events.iter().any(|e| e.get(attribute).is_some()))
}
/// Whether each trace carries an attribute.
pub fn verify_if_trace_attribute_is_in_each_trace(log: &EventLog, attribute: &str) -> bool {
    log.traces
        .iter()
        .all(|t| t.attributes.get(attribute).is_some())
}
/// Retain attributes present in at least one event of every trace.
pub fn check_event_attributes_presence(log: &EventLog, attributes: &[String]) -> Vec<String> {
    attributes
        .iter()
        .filter(|a| verify_if_event_attribute_is_in_each_trace(log, a))
        .cloned()
        .collect()
}
/// Retain attributes present in every trace.
pub fn check_trace_attributes_presence(log: &EventLog, attributes: &[String]) -> Vec<String> {
    attributes
        .iter()
        .filter(|a| verify_if_trace_attribute_is_in_each_trace(log, a))
        .cloned()
        .collect()
}

/// Calendar dimension for event counts.
#[derive(Clone, Copy, Debug)]
pub enum Distribution {
    /// Day of month, 1–31.
    DaysMonth,
    /// Month, 1–12.
    Months,
    /// Inclusive range of years present.
    Years,
    /// Hour, 0–23.
    Hours,
    /// Monday through Sunday.
    DaysWeek,
    /// ISO week, including pm4py's zero bin.
    Weeks,
}
/// Count timestamps in calendar bins, retaining zero-count bins.
pub fn get_events_distribution(
    log: &EventLog,
    keys: &EventKeys,
    dimension: Distribution,
) -> Result<Vec<(String, usize)>> {
    use ichnos_core::chrono::{Datelike, Timelike};
    let mut counts = std::collections::BTreeMap::<i32, usize>::new();
    for (ti, trace) in log.traces.iter().enumerate() {
        for (ei, event) in trace.events.iter().enumerate() {
            let position = ichnos_core::Position::Event {
                trace: ti,
                event: ei,
            };
            let v =
                event
                    .get(&keys.timestamp)
                    .ok_or_else(|| ichnos_core::Error::MissingAttribute {
                        key: keys.timestamp.clone(),
                        position,
                    })?;
            let d = v
                .as_date()
                .ok_or_else(|| ichnos_core::Error::AttributeType {
                    key: keys.timestamp.clone(),
                    position,
                    expected: "date",
                    found: v.type_name(),
                })?;
            let bin = match dimension {
                Distribution::DaysMonth => d.day() as i32,
                Distribution::Months => d.month() as i32,
                Distribution::Years => d.year(),
                Distribution::Hours => d.hour() as i32,
                Distribution::DaysWeek => d.weekday().num_days_from_monday() as i32,
                Distribution::Weeks => d.iso_week().week() as i32,
            };
            *counts.entry(bin).or_default() += 1;
        }
    }
    let range = match dimension {
        Distribution::DaysMonth => 1..=31,
        Distribution::Months => 1..=12,
        Distribution::Hours => 0..=23,
        Distribution::DaysWeek => 0..=6,
        Distribution::Weeks => 0..=52,
        Distribution::Years => {
            if counts.is_empty() {
                return Ok(Vec::new());
            }
            *counts.first_key_value().unwrap().0..=*counts.last_key_value().unwrap().0
        }
    };
    for i in range {
        counts.entry(i).or_default();
    }
    let days = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    Ok(counts
        .into_iter()
        .map(|(i, n)| {
            (
                if matches!(dimension, Distribution::DaysWeek) {
                    days[i as usize].to_owned()
                } else {
                    format!("{i:02}")
                },
                n,
            )
        })
        .collect())
}
/// Attribute selection options for decision trees.
#[derive(Clone, Copy, Debug)]
pub struct SelectionOptions {
    /// Maximum sampled cases.
    pub max_cases: usize,
    /// String attributes must have fewer distinct values than this bound.
    pub max_distinct: f64,
    /// Explicit repeatable sample seed.
    pub seed: u64,
}
impl Default for SelectionOptions {
    fn default() -> Self {
        Self {
            max_cases: 50,
            max_distinct: 12.5,
            seed: 0,
        }
    }
}
/// Attribute groups usable in decision trees.
#[derive(Clone, Debug, Default)]
pub struct SelectedAttributes {
    /// Categorical trace attributes.
    pub string_trace: Vec<String>,
    /// Categorical event attributes.
    pub string_event: Vec<String>,
    /// Numeric trace attributes.
    pub numeric_trace: Vec<String>,
    /// Numeric event attributes.
    pub numeric_event: Vec<String>,
}
/// Discover candidates from sampled cases, then check values and presence over the full log.
pub fn select_attributes_from_log_for_tree(
    log: &EventLog,
    options: SelectionOptions,
) -> Result<SelectedAttributes> {
    let sample = log.sample_cases(options.max_cases, options.seed);
    let mut result = SelectedAttributes::default();
    for trace in [false, true] {
        let names = if trace {
            get_trace_attributes(&sample)
        } else {
            get_event_attributes(&sample)
        };
        for name in names {
            let present = if trace {
                verify_if_trace_attribute_is_in_each_trace(log, &name)
            } else {
                verify_if_event_attribute_is_in_each_trace(log, &name)
            };
            if !present {
                continue;
            }
            let values = if trace {
                get_trace_attribute_values(log, &name)?
            } else {
                get_event_attribute_values(log, &name, AttributeCountOptions::default())?
            };
            let no_bools = if trace {
                log.traces
                    .iter()
                    .filter_map(|t| t.attributes.get(&name))
                    .all(|v| v.as_bool().is_none())
            } else {
                log.traces
                    .iter()
                    .flat_map(|t| &t.events)
                    .filter_map(|e| e.get(&name))
                    .all(|v| v.as_bool().is_none())
            };
            let numeric = no_bools
                && values
                    .keys()
                    .all(|v| matches!(v, Scalar::Int(_) | Scalar::Float(_)));
            let string = values.keys().all(|v| matches!(v, Scalar::String(_)))
                && (values.len() as f64) < options.max_distinct;
            if numeric {
                if trace {
                    result.numeric_trace.push(name);
                } else {
                    result.numeric_event.push(name);
                }
            } else if string {
                if trace {
                    result.string_trace.push(name);
                } else {
                    result.string_event.push(name);
                }
            }
        }
    }
    Ok(result)
}
