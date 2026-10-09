use super::{Retention, select};
use crate::{
    Error, Result,
    attributes::{AttributeCountOptions, Scalar, get_event_attribute_values},
};
use ichnos_core::EventLog;
use std::collections::HashSet;

/// Scope of an event-attribute filter.
#[derive(Clone, Copy, Debug, Default)]
pub enum FilterLevel {
    /// Keep complete cases containing a matching event.
    #[default]
    Cases,
    /// Keep only matching events, dropping empty cases.
    Events,
}
fn member(
    value: Option<&ichnos_core::AttributeValue>,
    values: &HashSet<Scalar>,
    key: &str,
) -> Result<bool> {
    value
        .map(|v| {
            Scalar::from_value(v)
                .ok_or_else(|| Error::NonScalar {
                    key: key.to_owned(),
                })
                .map(|v| values.contains(&v))
        })
        .transpose()
        .map(|v| v.unwrap_or(false))
}
/// Filter event values, treating absent attributes as nonmatches.
/// Event scope preserves trace metadata and drops cases with no retained events.
pub fn filter_event_attribute_values(
    log: &EventLog,
    attribute: &str,
    values: &[Scalar],
    level: FilterLevel,
    retention: Retention,
) -> Result<EventLog> {
    let values = values.iter().cloned().collect();
    let mut result = log.filter_traces(|_| false);
    for trace in &log.traces {
        let matches = trace
            .events
            .iter()
            .map(|e| member(e.get(attribute), &values, attribute))
            .collect::<Result<Vec<_>>>()?;
        match level {
            FilterLevel::Cases => {
                if !trace.is_empty() && retention.accepts(matches.iter().any(|v| *v)) {
                    result.traces.push(trace.clone());
                }
            }
            FilterLevel::Events => {
                let events = trace
                    .events
                    .iter()
                    .zip(matches)
                    .filter(|(_, v)| retention.accepts(*v))
                    .map(|(e, _)| e.clone())
                    .collect::<Vec<_>>();
                if !events.is_empty() {
                    result.traces.push(ichnos_core::Trace {
                        attributes: trace.attributes.clone(),
                        events,
                    });
                }
            }
        }
    }
    Ok(result)
}
/// Filter trace attributes; absent attributes are retained only in exclusion mode.
pub fn filter_trace_attribute_values(
    log: &EventLog,
    attribute: &str,
    values: &[Scalar],
    retention: Retention,
) -> Result<EventLog> {
    let values = values.iter().cloned().collect();
    select(log, |_, t| {
        Ok(retention.accepts(member(t.attributes.get(attribute), &values, attribute)?))
    })
}
/// Retain events whose value occurs in at least the given fraction of cases or events.
pub fn filter_log_relative_occurrence_event_attribute(
    log: &EventLog,
    attribute: &str,
    minimum: f64,
    level: FilterLevel,
) -> Result<EventLog> {
    super::dfg::fraction(minimum)?;
    let cases = matches!(level, FilterLevel::Cases);
    let counts = get_event_attribute_values(
        log,
        attribute,
        AttributeCountOptions {
            keep_once_per_case: cases,
        },
    )?;
    let total = if cases {
        log.len()
    } else {
        log.traces.iter().map(|t| t.len()).sum()
    };
    let values = counts
        .into_iter()
        .filter(|(_, n)| *n as f64 >= minimum * total as f64)
        .map(|(v, _)| v)
        .collect::<Vec<_>>();
    filter_event_attribute_values(
        log,
        attribute,
        &values,
        FilterLevel::Events,
        Retention::Retain,
    )
}
