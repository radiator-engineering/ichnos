//! Object types, attribute names, flattening and the temporal, object and
//! interaction summaries of `pm4py/ocel.py`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use ichnos_core::{AttributeValue, Event, EventStream};

use crate::constants::PREFIX;
use crate::ocel::{EventObject, Ocel};

/// The events and objects at one timestamp (a row of pm4py's
/// `ocel_temporal_summary`).
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalSummaryRow {
    /// The timestamp.
    pub timestamp: DateTime<FixedOffset>,
    /// The activity of each relation at this timestamp, in relation order.
    pub activities: Vec<Arc<str>>,
    /// The object of each relation at this timestamp, in relation order.
    pub objects: Vec<Arc<str>>,
}

/// The lifecycle of one object (a row of pm4py's `ocel_objects_summary`).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSummaryRow {
    /// The object id.
    pub object: Arc<str>,
    /// The activity of each related event, in relation order (pm4py's
    /// `activities_lifecycle`).
    pub activities: Vec<Arc<str>>,
    /// The earliest related event time (pm4py's `lifecycle_start`).
    pub start: DateTime<FixedOffset>,
    /// The latest related event time (pm4py's `lifecycle_end`).
    pub end: DateTime<FixedOffset>,
    /// `end - start` in seconds (pm4py's `lifecycle_duration`).
    pub duration: f64,
    /// The other objects that share an event with this one (pm4py's
    /// `interacting_objects`). `None` when the object is not in
    /// [`Ocel::objects`], where pandas has a missing value.
    pub interacting: Option<BTreeSet<Arc<str>>>,
}

/// Two objects related to the same event (a row of pm4py's
/// `ocel_objects_interactions_summary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionRow {
    /// The event id.
    pub event: Arc<str>,
    /// The activity of the event.
    pub activity: Arc<str>,
    /// The first object id.
    pub object: Arc<str>,
    /// The type of the first object.
    pub object_type: Arc<str>,
    /// The second object id.
    pub object_2: Arc<str>,
    /// The type of the second object.
    pub object_type_2: Arc<str>,
}

impl Ocel {
    /// The object types, in the order they first appear in
    /// [`Ocel::objects`] (pm4py's `ocel_get_object_types`).
    pub fn object_types(&self) -> Vec<Arc<str>> {
        let mut seen = HashSet::new();
        self.objects
            .iter()
            .filter(|o| seen.insert(&o.object_type))
            .map(|o| o.object_type.clone())
            .collect()
    }

    /// The names of the event and object attributes, sorted, without names
    /// that start with `ocel:` (pm4py's `ocel_get_attribute_names`).
    ///
    /// pm4py lists every column of its events and objects tables, even a
    /// column with no values. Here a name counts only when some event or
    /// object holds a value for it.
    pub fn attribute_names(&self) -> Vec<Arc<str>> {
        let names: BTreeSet<&Arc<str>> = self
            .events
            .iter()
            .flat_map(|e| e.attributes.iter().map(|(k, _)| k))
            .chain(
                self.objects
                    .iter()
                    .flat_map(|o| o.attributes.iter().map(|(k, _)| k)),
            )
            .filter(|k| !k.starts_with(PREFIX))
            .collect();
        names.into_iter().cloned().collect()
    }

    /// The log flattened on `object_type` (pm4py's `ocel_flattening`): one
    /// event for each pair of an event and a related object of that type.
    ///
    /// The events come in log order, and the objects of one event in the
    /// order of [`Ocel::objects`]. Each event holds `ocel:eid`,
    /// `concept:name`, `time:timestamp` and its own attributes, then
    /// `case:concept:name` (the object id), `case:ocel:type` and the object
    /// attributes with the `case:` prefix. A repeated relation gives one
    /// event. The object type of a relation is the type of the first object
    /// with its id.
    pub fn flatten(&self, object_type: &str) -> EventStream {
        let objects = self.object_index();
        let mut related: HashMap<&str, HashSet<&str>> = HashMap::new();
        for r in &self.relations {
            if objects
                .get(&*r.object)
                .is_some_and(|&o| &*self.objects[o].object_type == object_type)
            {
                related.entry(&*r.event).or_default().insert(&*r.object);
            }
        }
        let mut rows: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, o) in self.objects.iter().enumerate() {
            if &*o.object_type == object_type {
                rows.entry(&*o.id).or_default().push(i);
            }
        }
        let mut events = Vec::new();
        for e in &self.events {
            let Some(ids) = related.get(&*e.id) else {
                continue;
            };
            let mut matched: Vec<usize> = ids
                .iter()
                .flat_map(|id| rows.get(id).into_iter().flatten().copied())
                .collect();
            matched.sort_unstable();
            for o in matched {
                let o = &self.objects[o];
                let mut event = Event::new();
                event.insert("ocel:eid", e.id.clone());
                event.insert("concept:name", e.activity.clone());
                event.insert("time:timestamp", AttributeValue::Date(e.timestamp));
                for (k, v) in e.attributes.iter() {
                    event.insert(k.clone(), v.clone());
                }
                event.insert("case:concept:name", o.id.clone());
                event.insert("case:ocel:type", o.object_type.clone());
                for (k, v) in o.attributes.iter() {
                    event.insert(format!("case:{k}"), v.clone());
                }
                events.push(event);
            }
        }
        EventStream {
            events,
            ..EventStream::default()
        }
    }

    /// The relations at each timestamp, earliest first (pm4py's
    /// `ocel_temporal_summary`).
    ///
    /// A row groups the relations whose events share a time instant. Its
    /// timestamp keeps the offset of the first such event. A relation whose
    /// event is not in the log is left out.
    pub fn temporal_summary(&self) -> Vec<TemporalSummaryRow> {
        let mut groups: BTreeMap<DateTime<FixedOffset>, TemporalSummaryRow> = BTreeMap::new();
        for (r, e) in self.relation_events() {
            let row = groups
                .entry(e.timestamp)
                .or_insert_with(|| TemporalSummaryRow {
                    timestamp: e.timestamp,
                    activities: Vec::new(),
                    objects: Vec::new(),
                });
            row.activities.push(e.activity.clone());
            row.objects.push(r.object.clone());
        }
        groups.into_values().collect()
    }

    /// The lifecycle of each related object, by object id (pm4py's
    /// `ocel_objects_summary`).
    ///
    /// A relation whose event is not in the log is left out of the
    /// activities and times, but still counts for the interacting objects,
    /// as in pm4py. An object outside [`Ocel::objects`] has no interacting
    /// set, and is not added to the sets of the others; pm4py raises a
    /// `KeyError` there. An empty log gives no rows, where pm4py raises an
    /// `AttributeError`.
    pub fn objects_summary(&self) -> Vec<ObjectSummaryRow> {
        let mut lifecycles: BTreeMap<&Arc<str>, ObjectSummaryRow> = BTreeMap::new();
        for (r, e) in self.relation_events() {
            let row = lifecycles
                .entry(&r.object)
                .or_insert_with(|| ObjectSummaryRow {
                    object: r.object.clone(),
                    activities: Vec::new(),
                    start: e.timestamp,
                    end: e.timestamp,
                    duration: 0.0,
                    interacting: None,
                });
            row.activities.push(e.activity.clone());
            row.start = row.start.min(e.timestamp);
            row.end = row.end.max(e.timestamp);
        }
        let mut graph: HashMap<&str, BTreeSet<Arc<str>>> = self
            .objects
            .iter()
            .map(|o| (&*o.id, BTreeSet::new()))
            .collect();
        for objects in self.objects_per_event().values() {
            for o1 in objects {
                if let Some(set) = graph.get_mut(&***o1) {
                    set.extend(objects.iter().filter(|o2| o2 != &o1).map(|&o2| o2.clone()));
                }
            }
        }
        lifecycles
            .into_values()
            .map(|mut row| {
                row.duration = total_seconds(row.end - row.start);
                row.interacting = graph.remove(&*row.object);
                row
            })
            .collect()
    }

    /// Each pair of distinct objects related to the same event (pm4py's
    /// `ocel_objects_interactions_summary`).
    ///
    /// Events come in id order, and the pairs of one event in relation
    /// order. A repeated relation repeats its pairs. pm4py raises a
    /// `KeyError` for an event or object outside the log. Here such an
    /// event gives no rows, and a pair with such an object is left out.
    pub fn objects_interactions_summary(&self) -> Vec<InteractionRow> {
        let events = self.event_index();
        let objects = self.object_index();
        let mut rows = Vec::new();
        for (event, related) in self.objects_per_event() {
            let Some(&e) = events.get(event) else {
                continue;
            };
            let typed: Vec<(&Arc<str>, &Arc<str>)> = related
                .iter()
                .filter_map(|&o| {
                    objects
                        .get(&**o)
                        .map(|&i| (o, &self.objects[i].object_type))
                })
                .collect();
            for &(o1, t1) in &typed {
                for &(o2, t2) in &typed {
                    if o1 != o2 {
                        rows.push(InteractionRow {
                            event: self.events[e].id.clone(),
                            activity: self.events[e].activity.clone(),
                            object: o1.clone(),
                            object_type: t1.clone(),
                            object_2: o2.clone(),
                            object_type_2: t2.clone(),
                        });
                    }
                }
            }
        }
        rows
    }

    /// Each relation with its event, in relation order. Relations whose
    /// event is not in the log are left out.
    fn relation_events(&self) -> impl Iterator<Item = (&EventObject, &crate::OcelEvent)> {
        let events = self.event_index();
        self.relations
            .iter()
            .filter_map(move |r| events.get(&*r.event).map(|&e| (r, &self.events[e])))
    }

    /// The related object ids of each event id, in relation order, with
    /// event ids sorted.
    fn objects_per_event(&self) -> BTreeMap<&str, Vec<&Arc<str>>> {
        let mut map: BTreeMap<&str, Vec<&Arc<str>>> = BTreeMap::new();
        for r in &self.relations {
            map.entry(&*r.event).or_default().push(&r.object);
        }
        map
    }
}

/// A duration in seconds, summed as pm4py's `get_total_seconds` does: whole
/// days and seconds, then microseconds, then nanoseconds.
fn total_seconds(d: chrono::TimeDelta) -> f64 {
    let nanos = d.num_nanoseconds().unwrap_or(i64::MAX);
    let whole = nanos.div_euclid(1_000_000_000);
    let rest = nanos.rem_euclid(1_000_000_000);
    (whole as f64 + 1e-6 * (rest / 1000) as f64) + 1e-9 * (rest % 1000) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_seconds_adds_parts_in_pandas_order() {
        let d = chrono::TimeDelta::nanoseconds(90_061_000_250_007);
        assert_eq!(total_seconds(d), (90_061.0 + 1e-6 * 250.0) + 1e-9 * 7.0);
    }
}
