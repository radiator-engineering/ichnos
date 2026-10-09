//! OCEL enrichment, sampling, duplicate handling and event ordering.
use crate::graphs::interaction_neighbours;
use crate::{
    EventField, ObjectFilterOptions, ObjectGraphKind, ObjectObject, Ocel, OcelEvent,
    discover_objects_graph, filter_ocel_objects,
};
use chrono::TimeDelta;
use ichnos_core::AttributeValue;
use rand::{Rng, seq::SliceRandom};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Failure of an OCEL transformation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransformationError {
    /// A relation refers to an event absent from the log.
    #[error("missing event {0}")]
    MissingEvent(String),
    /// A relation refers to an object absent from the log.
    #[error("missing object {0}")]
    MissingObject(String),
    /// An existing event has no E2O relation required by the operation.
    #[error("event {0} has no relations")]
    UnrelatedEvent(String),
    /// An existing object has no lifecycle required by the operation.
    #[error("object {0} has no relations")]
    UnrelatedObject(String),
    /// A required object type is absent.
    #[error("missing object type {0}")]
    MissingObjectType(String),
    /// A requested additional column is absent throughout the table.
    #[error("missing attribute {0}")]
    MissingAttribute(String),
    /// Sorting requires comparable scalar values.
    #[error("incomparable values in sort field {0}")]
    IncomparableField(String),
    /// Adding the event-index time delta exceeded the timestamp range.
    #[error("timestamp overflow at event {0}")]
    TimestampOverflow(String),
}

pub(crate) type Id = Arc<str>;

/// Appends graph-qualified O2O rows. `None` includes all five graphs. The
/// qualifier is the graph's pm4py name with `_graph` appended.
/// Interaction rows hold both orientations of each pair. As in pm4py's
/// enrichment entry point, [`ObjectGraphKind::Inheritance`] reuses the
/// descendants graph, unlike [`discover_objects_graph`].
/// Existing rows are retained, including duplicates. New rows are sorted.
/// Events without relations are harmless, whereas pm4py's graph lookup raises.
pub fn ocel_o2o_enrichment(log: &Ocel, graphs: Option<&[ObjectGraphKind]>) -> Ocel {
    let mut rows = BTreeSet::new();
    for &kind in graphs.unwrap_or(&ObjectGraphKind::ALL) {
        let source = match kind {
            ObjectGraphKind::Inheritance => ObjectGraphKind::Descendants,
            kind => kind,
        };
        let qualifier = format!("{}_graph", kind.name());
        for (a, b) in discover_objects_graph(log, source) {
            if kind == ObjectGraphKind::Interaction {
                rows.insert((b.clone(), a.clone(), qualifier.clone()));
            }
            rows.insert((a, b, qualifier.clone()));
        }
    }
    let mut result = log.clone();
    result
        .o2o
        .extend(rows.into_iter().map(|(source, target, q)| ObjectObject {
            source: source.into(),
            target: target.into(),
            qualifier: Some(q.into()),
        }));
    result
}

/// Replaces E2O qualifiers using first, last and interior relation occurrences.
/// Creation overrides termination; interior occurrences then override both.
/// Typed pairs avoid pm4py's `@@` concatenation collisions.
pub fn ocel_e2o_lifecycle_enrichment(log: &Ocel) -> Ocel {
    let mut lifecycles = BTreeMap::<Id, Vec<Id>>::new();
    for r in &log.relations {
        lifecycles
            .entry(r.object.clone())
            .or_default()
            .push(r.event.clone());
    }
    let mut qualifiers = BTreeMap::new();
    for (object, events) in lifecycles {
        if let Some(last) = events.last() {
            qualifiers.insert((last.clone(), object.clone()), "termination");
        }
        if let Some(first) = events.first() {
            qualifiers.insert((first.clone(), object.clone()), "creation");
        }
        for event in events.iter().skip(1).take(events.len().saturating_sub(2)) {
            qualifiers.insert((event.clone(), object.clone()), "other");
        }
    }
    let mut result = log.clone();
    for r in &mut result.relations {
        if let Some(q) = qualifiers.get(&(r.event.clone(), r.object.clone())) {
            r.qualifier = Some((*q).into());
        }
    }
    result
}

/// Samples distinct object ids uniformly without replacement, then propagates.
/// Supply an RNG for repeatable draws; Python's random stream differs.
pub fn sample_ocel_objects<R: Rng + ?Sized>(log: &Ocel, count: usize, rng: &mut R) -> Ocel {
    let mut seen = BTreeSet::new();
    let mut ids: Vec<_> = log
        .objects
        .iter()
        .filter_map(|o| seen.insert(o.id.clone()).then_some(o.id.clone()))
        .collect();
    ids.shuffle(rng);
    ids.truncate(count);
    let ids: Vec<_> = ids.iter().map(|s| &**s).collect();
    filter_ocel_objects(log, &ids, ObjectFilterOptions::default())
}

/// Limits applied before connected-component sampling.
#[derive(Debug, Clone, Copy)]
pub struct ComponentSampleOptions {
    /// Number of components to draw, capped to the eligible count.
    pub count: usize,
    /// Maximum events per eligible component.
    pub max_events: usize,
    /// Maximum objects per eligible component.
    pub max_objects: usize,
    /// Maximum E2O rows per eligible component, including duplicates.
    pub max_relations: usize,
}
impl Default for ComponentSampleOptions {
    fn default() -> Self {
        Self {
            count: 1,
            max_events: usize::MAX,
            max_objects: usize::MAX,
            max_relations: usize::MAX,
        }
    }
}
fn components(log: &Ocel) -> Vec<BTreeSet<Id>> {
    let graph = interaction_neighbours(log);
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for node in graph.keys() {
        if seen.contains(node) {
            continue;
        }
        let mut component = BTreeSet::new();
        let mut pending = vec![node.clone()];
        while let Some(n) = pending.pop() {
            if seen.insert(n.clone()) {
                component.insert(n.clone());
                pending.extend(graph[&n].iter().cloned());
            }
        }
        result.push(component);
    }
    result
}
/// Samples eligible interaction components. Isolated objects are excluded,
/// matching pm4py's edge-only graph. Only events, objects and E2O rows remain;
/// OCEL 2.0 tables and metadata are discarded by this entry point in pm4py.
pub fn sample_ocel_connected_components<R: Rng + ?Sized>(
    log: &Ocel,
    options: ComponentSampleOptions,
    rng: &mut R,
) -> Ocel {
    let mut candidates: Vec<_> = components(log)
        .into_iter()
        .map(|ids| {
            let relations: Vec<_> = log
                .relations
                .iter()
                .filter(|r| ids.contains(&r.object))
                .cloned()
                .collect();
            let events: BTreeSet<_> = relations.iter().map(|r| r.event.clone()).collect();
            Ocel {
                events: log
                    .events
                    .iter()
                    .filter(|e| events.contains(&e.id))
                    .cloned()
                    .collect(),
                objects: log
                    .objects
                    .iter()
                    .filter(|o| ids.contains(&o.id))
                    .cloned()
                    .collect(),
                relations,
                ..Ocel::default()
            }
        })
        .filter(|c| {
            c.events.len() <= options.max_events
                && c.objects.len() <= options.max_objects
                && c.relations.len() <= options.max_relations
        })
        .collect();
    candidates.shuffle(rng);
    candidates.truncate(options.count);
    let mut result = Ocel::default();
    for c in candidates {
        result.events.extend(c.events);
        result.objects.extend(c.objects);
        result.relations.extend(c.relations);
    }
    result
}

/// Drops E2O rows with repeated (activity, timestamp, object), retaining the
/// first row and propagating the remaining relation selection to all tables.
pub fn ocel_drop_duplicates(log: &Ocel) -> Result<Ocel, TransformationError> {
    let index = log.event_index();
    let mut seen = BTreeSet::new();
    let mut result = log.clone();
    let mut keep = Vec::with_capacity(log.relations.len());
    for r in &log.relations {
        let e = &log.events[*index
            .get(&*r.event)
            .ok_or_else(|| TransformationError::MissingEvent(r.event.to_string()))?];
        keep.push(seen.insert((e.activity.clone(), e.timestamp, r.object.clone())));
    }
    let mut i = 0;
    result.relations.retain(|_| {
        let yes = keep[i];
        i += 1;
        yes
    });
    result.retain_related();
    Ok(result)
}

/// Merges events selected by activity/timestamp groups, optionally by object.
/// An event chooses its largest relation group, breaking ties by descending
/// (object, activity, timestamp). New ids are deterministic and collision-free,
/// rather than random UUIDs. Unrelated events return a typed error.
/// E2E rows keep their original event ids, as in pm4py.
pub fn ocel_merge_duplicates(log: &Ocel, common_object: bool) -> Result<Ocel, TransformationError> {
    let index = log.event_index();
    let mut groups = BTreeMap::new();
    let mut keys = Vec::new();
    for r in &log.relations {
        let e = &log.events[*index
            .get(&*r.event)
            .ok_or_else(|| TransformationError::MissingEvent(r.event.to_string()))?];
        let key = (
            if common_object {
                Some(r.object.clone())
            } else {
                None
            },
            e.activity.clone(),
            e.timestamp,
        );
        *groups.entry(key.clone()).or_insert(0usize) += 1;
        keys.push(key);
    }
    let mut chosen = BTreeMap::new();
    for (r, key) in log.relations.iter().zip(&keys) {
        let score = (groups[key], key.clone());
        chosen
            .entry(r.event.clone())
            .and_modify(|old| {
                if score > *old {
                    *old = score.clone();
                }
            })
            .or_insert(score);
    }
    let occupied: BTreeSet<_> = log.events.iter().map(|e| e.id.clone()).collect();
    let mut new_ids = BTreeMap::new();
    let mut n = 0usize;
    for key in groups.keys() {
        loop {
            let id: Id = format!("merged-{n}").into();
            n += 1;
            if !occupied.contains(&id) {
                new_ids.insert(key.clone(), id);
                break;
            }
        }
    }
    let mut map = BTreeMap::new();
    for e in &log.events {
        let (_, key) = chosen
            .get(&e.id)
            .ok_or_else(|| TransformationError::UnrelatedEvent(e.id.to_string()))?;
        map.insert(e.id.clone(), new_ids[key].clone());
    }
    let mut result = log.clone();
    let mut seen = BTreeSet::new();
    for e in &mut result.events {
        e.id = map[&e.id].clone();
    }
    result.events.retain(|e| seen.insert(e.id.clone()));
    let mut seen = BTreeSet::new();
    for r in &mut result.relations {
        r.event = map[&r.event].clone();
    }
    result
        .relations
        .retain(|r| seen.insert((r.event.clone(), r.object.clone())));
    Ok(result)
}

fn field(e: &OcelEvent, f: EventField<'_>) -> Option<AttributeValue> {
    match f {
        EventField::Id => Some(AttributeValue::String(e.id.clone())),
        EventField::Activity => Some(AttributeValue::String(e.activity.clone())),
        EventField::Timestamp => Some(AttributeValue::Date(e.timestamp)),
        EventField::Attribute(k) => e.attributes.get(k).cloned(),
    }
}
fn compare(a: &AttributeValue, b: &AttributeValue) -> Option<Ordering> {
    match (a.plain(), b.plain()) {
        (AttributeValue::String(a), AttributeValue::String(b))
        | (AttributeValue::Id(a), AttributeValue::Id(b)) => Some(a.cmp(b)),
        (AttributeValue::Date(a), AttributeValue::Date(b)) => Some(a.cmp(b)),
        (AttributeValue::Int(a), AttributeValue::Int(b)) => Some(a.cmp(b)),
        _ => {
            let num = |v: &AttributeValue| match v.plain() {
                AttributeValue::Int(n) => Some(*n as f64),
                AttributeValue::Float(n) => Some(*n),
                AttributeValue::Bool(b) => Some(u8::from(*b).into()),
                _ => None,
            };
            num(a).zip(num(b)).and_then(|(a, b)| a.partial_cmp(&b))
        }
    }
}
/// Stably sorts events by two typed fields, placing absent attributes last.
/// Relations retain their order. Mixed incomparable or nested values error.
pub fn ocel_sort_by_additional_column(
    log: &Ocel,
    additional: EventField<'_>,
    primary: EventField<'_>,
) -> Result<Ocel, TransformationError> {
    for f in [primary, additional] {
        let values: Vec<_> = log
            .events
            .iter()
            .filter_map(|e| field(e, f))
            .filter(|v| !matches!(v.plain(),AttributeValue::Float(n) if n.is_nan()))
            .collect();
        if let EventField::Attribute(k) = f
            && values.is_empty()
            && !log.events.iter().any(|e| e.attributes.contains_key(k))
        {
            return Err(TransformationError::MissingAttribute(k.into()));
        }
        if let Some(first) = values.first()
            && values.iter().any(|v| compare(first, v).is_none())
        {
            return Err(TransformationError::IncomparableField(format!("{f:?}")));
        }
    }
    let mut result = log.clone();
    let cmp = |a: &OcelEvent, b: &OcelEvent, f| {
        let present = |v: Option<AttributeValue>| {
            v.filter(|v| !matches!(v.plain(),AttributeValue::Float(n) if n.is_nan()))
        };
        match (present(field(a, f)), present(field(b, f))) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => compare(&a, &b).unwrap_or(Ordering::Equal),
        }
    };
    result
        .events
        .sort_by(|a, b| cmp(a, b, primary).then_with(|| cmp(a, b, additional)));
    Ok(result)
}
/// Adds one millisecond per event index. Duplicate ids use their last index,
/// matching pm4py. Relations derive timestamps from events without duplication.
pub fn ocel_add_index_based_timedelta(log: &Ocel) -> Result<Ocel, TransformationError> {
    let mut indices = BTreeMap::new();
    for (i, e) in log.events.iter().enumerate() {
        indices.insert(e.id.clone(), i);
    }
    let mut result = log.clone();
    for e in &mut result.events {
        let delta = i64::try_from(indices[&e.id])
            .ok()
            .and_then(TimeDelta::try_milliseconds);
        e.timestamp = delta
            .and_then(|d| e.timestamp.checked_add_signed(d))
            .ok_or_else(|| TransformationError::TimestampOverflow(e.id.to_string()))?;
    }
    Ok(result)
}
