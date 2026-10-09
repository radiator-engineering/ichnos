//! OCEL filters with propagation to relations and OCEL 2.0 tables.
//!
//! Every filter returns a new log and preserves the input row order and globals.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use ichnos_core::AttributeValue;

use crate::graphs::interaction_neighbours;
use crate::{Ocel, OcelEvent, OcelObject};

/// Whether matching rows are kept or removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Selection {
    /// Keep matching rows (pm4py's `positive=True`).
    #[default]
    Keep,
    /// Remove matching rows (pm4py's `positive=False`).
    Remove,
}

impl Selection {
    fn accepts(self, matches: bool) -> bool {
        matches == (self == Self::Keep)
    }
}

/// An event field to match. Typed fields replace pm4py's configurable columns.
#[derive(Debug, Clone, Copy)]
pub enum EventField<'a> {
    /// The event id.
    Id,
    /// The event activity.
    Activity,
    /// The event timestamp.
    Timestamp,
    /// An additional event attribute; absent attributes do not match.
    Attribute(&'a str),
}

/// A time field usable for timestamp filtering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EventTimeField<'a> {
    /// The event timestamp.
    #[default]
    Timestamp,
    /// An additional date-valued event attribute.
    /// Missing attributes and values of another type do not match.
    DateAttribute(&'a str),
}

/// An object field to match.
#[derive(Debug, Clone, Copy)]
pub enum ObjectField<'a> {
    /// The object id.
    Id,
    /// The object type.
    Type,
    /// An additional object attribute; absent attributes do not match.
    Attribute(&'a str),
}

/// Selection and interaction-graph expansion for object filters.
#[derive(Debug, Clone, Copy)]
pub struct ObjectFilterOptions {
    /// Whether to keep or remove the expanded object set.
    pub selection: Selection,
    /// One selects only the given objects; each further level adds one hop.
    /// Zero behaves like one, as in pm4py.
    pub level: usize,
}

impl Default for ObjectFilterOptions {
    fn default() -> Self {
        Self {
            selection: Selection::Keep,
            level: 1,
        }
    }
}

fn event_value(event: &OcelEvent, field: EventField<'_>) -> Option<AttributeValue> {
    match field {
        EventField::Id => Some(AttributeValue::String(event.id.clone())),
        EventField::Activity => Some(AttributeValue::String(event.activity.clone())),
        EventField::Timestamp => Some(AttributeValue::Date(event.timestamp)),
        EventField::Attribute(key) => event.attributes.get(key).cloned(),
    }
}

fn object_value(object: &OcelObject, field: ObjectField<'_>) -> Option<AttributeValue> {
    match field {
        ObjectField::Id => Some(AttributeValue::String(object.id.clone())),
        ObjectField::Type => Some(AttributeValue::String(object.object_type.clone())),
        ObjectField::Attribute(key) => object.attributes.get(key).cloned(),
    }
}

// pandas membership treats bools and numeric scalar values by numeric equality.
fn equal(a: &AttributeValue, b: &AttributeValue) -> bool {
    fn numeric(value: &AttributeValue) -> Option<f64> {
        match value {
            AttributeValue::Int(n) => Some(*n as f64),
            AttributeValue::Float(n) => Some(*n),
            AttributeValue::Bool(b) => Some(u8::from(*b).into()),
            _ => None,
        }
    }
    match (a, b) {
        (AttributeValue::Int(a), AttributeValue::Int(b)) => a == b,
        _ => match (numeric(a), numeric(b)) {
            (Some(a), Some(b)) => a == b,
            _ => a == b,
        },
    }
}

fn clear_tables(log: &mut Ocel) {
    log.events.clear();
    log.objects.clear();
    log.relations.clear();
    log.o2o.clear();
    log.e2e.clear();
    log.object_changes.clear();
}

fn propagate_events(mut log: Ocel) -> Ocel {
    let events: HashSet<_> = log.events.iter().map(|e| e.id.clone()).collect();
    if events.is_empty() {
        clear_tables(&mut log);
        return log;
    }
    log.relations.retain(|r| events.contains(&r.event));
    let objects: HashSet<_> = log.relations.iter().map(|r| r.object.clone()).collect();
    log.objects.retain(|o| objects.contains(&o.id));
    log.o2o
        .retain(|r| objects.contains(&r.source) && objects.contains(&r.target));
    log.object_changes.retain(|c| objects.contains(&c.object));
    log.e2e
        .retain(|r| events.contains(&r.source) && events.contains(&r.target));
    log
}

fn propagate_objects(mut log: Ocel) -> Ocel {
    let objects: HashSet<_> = log.objects.iter().map(|o| o.id.clone()).collect();
    if objects.is_empty() {
        clear_tables(&mut log);
        return log;
    }
    log.relations.retain(|r| objects.contains(&r.object));
    let events: HashSet<_> = log.relations.iter().map(|r| r.event.clone()).collect();
    log.events.retain(|e| events.contains(&e.id));
    log.e2e
        .retain(|r| events.contains(&r.source) && events.contains(&r.target));
    log.o2o
        .retain(|r| objects.contains(&r.source) && objects.contains(&r.target));
    log.object_changes.retain(|c| objects.contains(&c.object));
    log
}

/// Filters an event field by membership and propagates to other tables.
/// Missing custom attributes do not match, including when the key is absent
/// throughout the log (pm4py instead raises for an absent column).
/// Numeric and boolean scalar membership follows pandas numeric equality.
/// Nested attributes use Rust's structural equality.
pub fn filter_ocel_event_attribute(
    log: &Ocel,
    field: EventField<'_>,
    values: &[AttributeValue],
    selection: Selection,
) -> Ocel {
    let mut result = log.clone();
    result.events.retain(|e| {
        selection
            .accepts(event_value(e, field).is_some_and(|v| values.iter().any(|x| equal(&v, x))))
    });
    propagate_events(result)
}

/// Filters an object field by membership and propagates to other tables.
/// Missing custom attributes do not match; nested values use structural equality.
pub fn filter_ocel_object_attribute(
    log: &Ocel,
    field: ObjectField<'_>,
    values: &[AttributeValue],
    selection: Selection,
) -> Ocel {
    let mut result = log.clone();
    result.objects.retain(|o| {
        selection
            .accepts(object_value(o, field).is_some_and(|v| values.iter().any(|x| equal(&v, x))))
    });
    propagate_objects(result)
}

/// Keeps relations with an allowed `(object type, activity)` pair.
/// Pairs are compared directly, avoiding pm4py's temporary separator collision.
pub fn filter_ocel_object_types_allowed_activities(
    log: &Ocel,
    allowed: &BTreeMap<String, BTreeSet<String>>,
) -> Ocel {
    let mut result = log.clone();
    let events = log.event_index();
    let objects = log.object_index();
    result
        .relations
        .retain(|r| match (events.get(&*r.event), objects.get(&*r.object)) {
            (Some(&e), Some(&o)) => allowed
                .get(&*log.objects[o].object_type)
                .is_some_and(|activities| activities.contains(&*log.events[e].activity)),
            _ => false,
        });
    result.retain_related();
    result
}

/// Keeps events with at least each requested number of relations per type.
/// Duplicate relations count separately. A requested type must be present even
/// when its minimum is zero. An empty map keeps only events with relations.
pub fn filter_ocel_object_per_type_count(log: &Ocel, minimum: &BTreeMap<String, usize>) -> Ocel {
    let objects = log.object_index();
    let mut counts: HashMap<&str, HashMap<&str, usize>> = HashMap::new();
    for r in &log.relations {
        if let Some(&o) = objects.get(&*r.object) {
            *counts
                .entry(&r.event)
                .or_default()
                .entry(&log.objects[o].object_type)
                .or_default() += 1;
        }
    }
    let ids: Vec<_> = counts
        .into_iter()
        .filter(|(_, c)| {
            minimum
                .iter()
                .all(|(t, n)| c.get(t.as_str()).is_some_and(|count| count >= n))
        })
        .map(|(id, _)| id)
        .collect();
    filter_ocel_events(log, &ids, Selection::Keep)
}

fn endpoints(log: &Ocel, object_type: &str, last: bool) -> Ocel {
    let objects = log.object_index();
    let mut endpoint: HashMap<&str, &str> = HashMap::new();
    for r in &log.relations {
        if objects
            .get(&*r.object)
            .is_some_and(|&o| &*log.objects[o].object_type == object_type)
        {
            if last {
                endpoint.insert(&r.object, &r.event);
            } else {
                endpoint.entry(&r.object).or_insert(&r.event);
            }
        }
    }
    filter_ocel_events(
        log,
        &endpoint.into_values().collect::<Vec<_>>(),
        Selection::Keep,
    )
}

/// Keeps each object's first related event for the specified type.
/// First means relation order, not timestamp order.
pub fn filter_ocel_start_events_per_object_type(log: &Ocel, object_type: &str) -> Ocel {
    endpoints(log, object_type, false)
}

/// Keeps each object's last related event for the specified type, in relation order.
pub fn filter_ocel_end_events_per_object_type(log: &Ocel, object_type: &str) -> Ocel {
    endpoints(log, object_type, true)
}

/// Keeps events within an inclusive timestamp interval.
/// Use [`EventTimeField::Timestamp`] for the event timestamp, or
/// [`EventTimeField::DateAttribute`] for an additional date attribute. A custom
/// attribute must hold a date; missing or non-date values do not match.
/// Bounds are typed instants, so string parsing and local-time assumptions are
/// the caller's responsibility. Reversed bounds produce an empty log.
/// Event ids and activities are not time-field choices:
///
/// ```compile_fail
/// use chrono::{DateTime, FixedOffset};
/// use ichnos_ocel::{Ocel, EventField, filter_ocel_events_timestamp};
/// fn invalid(log: &Ocel, start: DateTime<FixedOffset>, end: DateTime<FixedOffset>) {
///     filter_ocel_events_timestamp(log, start, end, EventField::Id);
/// }
/// ```
pub fn filter_ocel_events_timestamp(
    log: &Ocel,
    minimum: DateTime<FixedOffset>,
    maximum: DateTime<FixedOffset>,
    field: EventTimeField<'_>,
) -> Ocel {
    let field = match field {
        EventTimeField::Timestamp => EventField::Timestamp,
        EventTimeField::DateAttribute(key) => EventField::Attribute(key),
    };
    let mut result = log.clone();
    result.events.retain(|e| matches!(event_value(e, field), Some(AttributeValue::Date(t)) if t >= minimum && t <= maximum));
    propagate_events(result)
}

/// Object interaction components, sorted by object id and component minimum.
/// Only objects that share an event with another object are included, matching
/// pm4py's edge-built graph. Isolated objects are omitted.
pub fn object_connected_components(log: &Ocel) -> Vec<BTreeSet<Arc<str>>> {
    let graph = interaction_neighbours(log);
    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    for start in graph.keys() {
        if !visited.insert(start.clone()) {
            continue;
        }
        let mut component = BTreeSet::from([start.clone()]);
        let mut pending = vec![start.clone()];
        while let Some(object) = pending.pop() {
            for neighbor in &graph[&object] {
                if visited.insert(neighbor.clone()) {
                    component.insert(neighbor.clone());
                    pending.push(neighbor.clone());
                }
            }
        }
        components.push(component);
    }
    components
}

/// Filters object ids, expanding their interaction neighborhood before selection.
/// Unknown ids are harmless, even during expansion (pm4py can raise there).
pub fn filter_ocel_objects(log: &Ocel, ids: &[&str], options: ObjectFilterOptions) -> Ocel {
    let mut selected: BTreeSet<Arc<str>> = ids.iter().map(|id| Arc::from(*id)).collect();
    if options.level > 1 {
        let graph = interaction_neighbours(log);
        for _ in 1..options.level {
            let before = selected.len();
            let neighbors: Vec<_> = selected
                .iter()
                .filter_map(|id| graph.get(id))
                .flat_map(|s| s.iter().cloned())
                .collect();
            selected.extend(neighbors);
            if selected.len() == before {
                break;
            }
        }
    }
    let mut result = log.clone();
    result
        .objects
        .retain(|o| options.selection.accepts(selected.contains(&o.id)));
    propagate_objects(result)
}

/// Filters object types, with the same expansion and selection as object ids.
pub fn filter_ocel_object_types(log: &Ocel, types: &[&str], options: ObjectFilterOptions) -> Ocel {
    let ids: Vec<_> = log
        .objects
        .iter()
        .filter(|o| types.contains(&&*o.object_type))
        .map(|o| &*o.id)
        .collect();
    filter_ocel_objects(log, &ids, options)
}

/// Filters event ids and propagates to related objects and auxiliary tables.
/// Selected events without relations remain in the result.
pub fn filter_ocel_events(log: &Ocel, ids: &[&str], selection: Selection) -> Ocel {
    let ids: HashSet<_> = ids.iter().copied().collect();
    let mut result = log.clone();
    result
        .events
        .retain(|e| selection.accepts(ids.contains(&*e.id)));
    propagate_events(result)
}

/// Keeps all activities observed on the specified type, including their
/// relations to other object types.
pub fn filter_ocel_activities_connected_object_type(log: &Ocel, object_type: &str) -> Ocel {
    let objects = log.object_index();
    let events = log.event_index();
    let activities: HashSet<_> = log
        .relations
        .iter()
        .filter(|r| {
            objects
                .get(&*r.object)
                .is_some_and(|&o| &*log.objects[o].object_type == object_type)
        })
        .filter_map(|r| events.get(&*r.event).map(|&e| &*log.events[e].activity))
        .collect();
    let mut result = log.clone();
    result.relations.retain(|r| {
        events
            .get(&*r.event)
            .is_some_and(|&e| activities.contains(&*log.events[e].activity))
    });
    result.retain_related();
    result
}

fn component_objects(log: &Ocel, components: &[BTreeSet<Arc<str>>]) -> Ocel {
    let ids: Vec<_> = components
        .iter()
        .flat_map(|c| c.iter().map(|id| &**id))
        .collect();
    filter_ocel_objects(log, &ids, ObjectFilterOptions::default())
}

/// Keeps the component containing an object, or the object itself if isolated.
/// Supply previously computed components to reuse them; [`object_connected_components`]
/// replaces pm4py's optional tuple return with a separately reusable result.
pub fn filter_ocel_cc_object(
    log: &Ocel,
    object_id: &str,
    components: Option<&[BTreeSet<Arc<str>>]>,
) -> Ocel {
    let computed;
    let components = match components {
        Some(c) => c,
        None => {
            computed = object_connected_components(log);
            &computed
        }
    };
    let ids = components
        .iter()
        .find(|c| c.contains(object_id))
        .map(|c| c.iter().map(|id| &**id).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![object_id]);
    filter_ocel_objects(log, &ids, ObjectFilterOptions::default())
}

/// Keeps components whose size lies within the inclusive bounds.
pub fn filter_ocel_cc_length(log: &Ocel, minimum: usize, maximum: usize) -> Ocel {
    let components: Vec<_> = object_connected_components(log)
        .into_iter()
        .filter(|c| c.len() >= minimum && c.len() <= maximum)
        .collect();
    component_objects(log, &components)
}

/// Keeps components containing a selected object type. With `Remove`, keeps
/// components containing any other type; this is not the complement when a
/// component mixes types, and matches pm4py's `positive=False` behavior.
pub fn filter_ocel_cc_otype(log: &Ocel, object_type: &str, selection: Selection) -> Ocel {
    let selected: HashSet<_> = log
        .objects
        .iter()
        .filter(|o| selection.accepts(&*o.object_type == object_type))
        .map(|o| o.id.clone())
        .collect();
    let components: Vec<_> = object_connected_components(log)
        .into_iter()
        .filter(|c| c.iter().any(|id| selected.contains(id)))
        .collect();
    component_objects(log, &components)
}

/// Keeps components touching an event with the requested activity.
pub fn filter_ocel_cc_activity(log: &Ocel, activity: &str) -> Ocel {
    let events: HashSet<_> = log
        .events
        .iter()
        .filter(|e| &*e.activity == activity)
        .map(|e| e.id.clone())
        .collect();
    let objects: HashSet<_> = log
        .relations
        .iter()
        .filter(|r| events.contains(&r.event))
        .map(|r| r.object.clone())
        .collect();
    let components: Vec<_> = object_connected_components(log)
        .into_iter()
        .filter(|c| c.iter().any(|id| objects.contains(id)))
        .collect();
    component_objects(log, &components)
}
