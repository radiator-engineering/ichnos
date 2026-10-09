//! Event log to OCEL, ported from pm4py's `convert_log_to_ocel`
//! (`objects/ocel/util/log_ocel.py`, `log_to_ocel_multiple_obj_types`).

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use ichnos_core::{AttributeValue, Attributes, Error, EventLog, Position};

use crate::{EventObject, Ocel, OcelEvent, OcelObject};

/// Options for [`convert_log_to_ocel`], with the defaults of pm4py's
/// `convert_log_to_ocel`.
///
/// A column that starts with `case:` reads the trace attribute after the
/// prefix, as in pm4py's data frame; any other column reads the event
/// attribute.
#[derive(Debug, Clone)]
pub struct LogToOcelOptions {
    /// The activity column. Default `concept:name`.
    pub activity_column: String,
    /// The timestamp column. Default `time:timestamp`.
    pub timestamp_column: String,
    /// The columns whose values are objects; each column is an object type.
    /// `None` takes `case:concept:name` and every column that starts with
    /// `ocel:type`, in name order.
    pub object_types: Option<Vec<String>>,
    /// The separator between objects in one value. Default `" AND "`.
    pub obj_separator: String,
    /// Columns copied to the events.
    pub additional_event_attributes: Vec<String>,
    /// For each object type, columns copied to its objects from the event
    /// that first relates them.
    pub additional_object_attributes: Vec<(String, Vec<String>)>,
}

impl Default for LogToOcelOptions {
    fn default() -> Self {
        LogToOcelOptions {
            activity_column: "concept:name".to_owned(),
            timestamp_column: "time:timestamp".to_owned(),
            object_types: None,
            obj_separator: " AND ".to_owned(),
            additional_event_attributes: Vec::new(),
            additional_object_attributes: Vec::new(),
        }
    }
}

/// The value of `column` for an event; pandas' missing values (a float NaN)
/// count as absent.
fn column<'a>(log: &'a EventLog, t: usize, e: usize, column: &str) -> Option<&'a AttributeValue> {
    let v = match column.strip_prefix("case:") {
        Some(name) => log.traces[t].attributes.get(name),
        None => log.traces[t].events[e].get(column),
    };
    v.filter(|v| !matches!(v.plain(), AttributeValue::Float(x) if x.is_nan()))
}

/// An object-centric event log with one event per event of `log`, as
/// pm4py's `convert_log_to_ocel` builds it.
///
/// Event ids count the events from `0`, trace by trace. Each object-type
/// column splits its value on `obj_separator`; each non-blank piece is an
/// object, of the type of the first column that holds it, related to the
/// event. A value that is not a string relates no objects, as in pm4py, where
/// splitting it fails silently. An event relates an object once.
///
/// pm4py keeps the column's object type on each relation; here a relation
/// takes the type of its object.
///
/// # Errors
///
/// An event without the activity or timestamp column, or with a timestamp
/// that is not a date.
pub fn convert_log_to_ocel(log: &EventLog, options: &LogToOcelOptions) -> Result<Ocel, Error> {
    let object_types: Vec<String> = match &options.object_types {
        Some(types) => types.clone(),
        None => {
            let mut names: BTreeSet<String> = BTreeSet::new();
            for trace in &log.traces {
                if trace.attributes.get("concept:name").is_some() {
                    names.insert("case:concept:name".to_owned());
                }
                for event in &trace.events {
                    for (k, _) in &event.attributes {
                        if k.starts_with("ocel:type") {
                            names.insert(k.to_string());
                        }
                    }
                }
            }
            names.into_iter().collect()
        }
    };
    let mut ocel = Ocel::new();
    let mut seen_objects: HashSet<String> = HashSet::new();
    let mut seen_relations: HashSet<(Arc<str>, String)> = HashSet::new();
    let mut index = 0_usize;
    for (t, trace) in log.traces.iter().enumerate() {
        for e in 0..trace.events.len() {
            let position = Position::Event { trace: t, event: e };
            let missing = |key: &str| Error::MissingAttribute {
                key: key.to_owned(),
                position,
            };
            let activity = column(log, t, e, &options.activity_column)
                .ok_or_else(|| missing(&options.activity_column))?;
            let stamp = column(log, t, e, &options.timestamp_column)
                .ok_or_else(|| missing(&options.timestamp_column))?;
            let timestamp = stamp.as_date().ok_or_else(|| Error::AttributeType {
                key: options.timestamp_column.clone(),
                position,
                expected: "date",
                found: stamp.type_name(),
            })?;
            let id: Arc<str> = index.to_string().into();
            index += 1;
            let mut attributes = Attributes::default();
            for name in &options.additional_event_attributes {
                if let Some(v) = column(log, t, e, name) {
                    attributes.insert(name.as_str(), v.clone());
                }
            }
            ocel.events.push(OcelEvent {
                id: id.clone(),
                activity: activity.to_string().into(),
                timestamp,
                attributes,
            });
            for ot in &object_types {
                let Some(value) = column(log, t, e, ot).and_then(AttributeValue::as_str) else {
                    continue;
                };
                if options.obj_separator.is_empty() {
                    continue;
                }
                for obj in value.split(options.obj_separator.as_str()) {
                    if obj.trim().is_empty() {
                        continue;
                    }
                    if seen_objects.insert(obj.to_owned()) {
                        let mut attributes = Attributes::default();
                        let names = options
                            .additional_object_attributes
                            .iter()
                            .filter(|(t, _)| t == ot)
                            .flat_map(|(_, names)| names);
                        for name in names {
                            if let Some(v) = column(log, t, e, name) {
                                attributes.insert(name.as_str(), v.clone());
                            }
                        }
                        ocel.objects.push(OcelObject {
                            id: obj.into(),
                            object_type: ot.as_str().into(),
                            attributes,
                        });
                    }
                    if seen_relations.insert((id.clone(), obj.to_owned())) {
                        ocel.relations.push(EventObject {
                            event: id.clone(),
                            object: obj.into(),
                            qualifier: None,
                        });
                    }
                }
            }
        }
    }
    Ok(ocel)
}
