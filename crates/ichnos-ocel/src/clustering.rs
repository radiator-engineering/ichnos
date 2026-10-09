//! Equivalent executions after ancestor/descendant selection and object renaming.
use crate::transformations::Id;
use crate::{
    ObjectFilterOptions, ObjectGraphKind, Ocel, TransformationError, discover_objects_graph,
    filter_ocel_objects,
};
use std::collections::{BTreeMap, BTreeSet};

/// Timestamp-free execution description used as an equivalence key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExecutionDescription {
    /// Events sorted by timestamp, activity and original event id. Each row
    /// starts with the activity followed by canonical related object ids.
    pub events: Vec<Vec<String>>,
    /// Canonical object ids and types, sorted by the canonical id.
    pub objects: Vec<(String, String)>,
}
/// Options for equivalent-execution clustering.
#[derive(Debug, Clone, Default)]
pub struct EquivalentOcelOptions<'a> {
    /// Reference object type; each matching object contributes an execution.
    pub object_type: &'a str,
    /// Cap on reference objects, in object-table order. `None` has no limit.
    pub max_objects: Option<usize>,
    /// Object types whose ids remain unchanged in the equivalence key.
    pub exclude_object_types_from_renaming: BTreeSet<&'a str>,
}
/// One original execution and its reference object.
#[derive(Debug, Clone, PartialEq)]
pub struct OcelExecution {
    /// The reference object id, formerly pm4py's `@@central_object` parameter.
    pub central_object: Id,
    /// The original rows filtered to the reference object's ancestors and
    /// descendants. Canonical renaming is used only for the grouping key.
    pub log: Ocel,
}
fn reachable(graph: &BTreeMap<Id, BTreeSet<Id>>, start: &Id) -> BTreeSet<Id> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![start.clone()];
    while let Some(id) = pending.pop() {
        if seen.insert(id.clone())
            && let Some(next) = graph.get(&id)
        {
            pending.extend(next.iter().cloned());
        }
    }
    seen
}
fn describe(
    log: &Ocel,
    excluded: &BTreeSet<&str>,
) -> Result<ExecutionDescription, TransformationError> {
    let events = log.event_index();
    let mut times = BTreeMap::new();
    for r in &log.relations {
        let e = &log.events[*events
            .get(&*r.event)
            .ok_or_else(|| TransformationError::MissingEvent(r.event.to_string()))?];
        times
            .entry(r.object.clone())
            .and_modify(|pair: &mut (_, _)| pair.1 = e.timestamp)
            .or_insert((e.timestamp, e.timestamp));
    }
    let mut types = BTreeMap::<Id, Vec<Id>>::new();
    for o in &log.objects {
        types
            .entry(o.object_type.clone())
            .or_default()
            .push(o.id.clone());
    }
    let mut names = BTreeMap::new();
    for (typ, mut ids) in types {
        for id in &ids {
            if !times.contains_key(id) {
                return Err(TransformationError::UnrelatedObject(id.to_string()));
            }
        }
        ids.sort_by_key(|id| (times[id], id.clone()));
        for (i, id) in ids.into_iter().enumerate() {
            let name = if excluded.contains(&*typ) {
                id.to_string()
            } else {
                format!("{typ}_{}", i + 1)
            };
            names.insert(id, name);
        }
    }
    let mut objects: Vec<_> = log
        .objects
        .iter()
        .map(|o| (names[&o.id].clone(), o.object_type.to_string()))
        .collect();
    objects.sort_by(|a, b| a.0.cmp(&b.0));
    let mut relations = log.relations.clone();
    for r in &relations {
        if !names.contains_key(&r.object) {
            return Err(TransformationError::MissingObject(r.object.to_string()));
        }
    }
    relations.sort_by_key(|r| {
        let e = &log.events[events[&*r.event]];
        (
            e.timestamp,
            e.activity.clone(),
            names[&r.object].clone(),
            r.event.clone(),
        )
    });
    let mut per_event = BTreeMap::<Id, Vec<String>>::new();
    for r in relations {
        per_event
            .entry(r.event)
            .or_default()
            .push(names[&r.object].clone());
    }
    let mut sorted = log.events.clone();
    sorted.sort_by_key(|e| (e.timestamp, e.activity.clone(), e.id.clone()));
    let mut descriptions = Vec::new();
    for e in sorted {
        let mut row = vec![e.activity.to_string()];
        row.extend(
            per_event
                .get(&e.id)
                .ok_or_else(|| TransformationError::MissingEvent(e.id.to_string()))?
                .iter()
                .cloned(),
        );
        descriptions.push(row);
    }
    Ok(ExecutionDescription {
        events: descriptions,
        objects,
    })
}
/// Groups equivalent executions using pm4py's ancestor/descendant selection,
/// lifecycle-time/lexical object renaming and timestamp-free variant2 key.
/// Unrelated reference objects return an error rather than a Python KeyError.
pub fn cluster_equivalent_ocel(
    log: &Ocel,
    options: &EquivalentOcelOptions<'_>,
) -> Result<BTreeMap<ExecutionDescription, Vec<OcelExecution>>, TransformationError> {
    let events = log.event_index();
    let objects = log.object_index();
    for relation in &log.relations {
        if !objects.contains_key(&*relation.object) {
            return Err(TransformationError::MissingObject(
                relation.object.to_string(),
            ));
        }
    }
    let mut starts = BTreeMap::new();
    for r in &log.relations {
        let e = &log.events[*events
            .get(&*r.event)
            .ok_or_else(|| TransformationError::MissingEvent(r.event.to_string()))?];
        starts.entry(r.object.clone()).or_insert(e.timestamp);
    }
    let refs: Vec<_> = log
        .objects
        .iter()
        .filter(|o| &*o.object_type == options.object_type)
        .map(|o| o.id.clone())
        .collect();
    let reference: BTreeSet<_> = refs.iter().cloned().collect();
    let mut forward = BTreeMap::<Id, BTreeSet<Id>>::new();
    let mut reverse = BTreeMap::<Id, BTreeSet<Id>>::new();
    for (a, b) in discover_objects_graph(log, ObjectGraphKind::Interaction) {
        let (a, b): (Id, Id) = (a.into(), b.into());
        let ta = starts[&a];
        let tb = starts[&b];
        let edge = if ta < tb || (ta == tb && reference.contains(&a)) {
            Some((a, b))
        } else if ta > tb || (ta == tb && reference.contains(&b)) {
            Some((b, a))
        } else {
            None
        };
        if let Some((a, b)) = edge {
            forward.entry(a.clone()).or_default().insert(b.clone());
            reverse.entry(b).or_default().insert(a);
        }
    }
    let mut result = BTreeMap::<ExecutionDescription, Vec<OcelExecution>>::new();
    for id in refs
        .into_iter()
        .take(options.max_objects.unwrap_or(usize::MAX))
    {
        let mut selected = reachable(&forward, &id);
        selected.extend(reachable(&reverse, &id));
        let ids: Vec<_> = selected.iter().map(|s| &**s).collect();
        let execution = filter_ocel_objects(log, &ids, ObjectFilterOptions::default());
        let key = describe(&execution, &options.exclude_object_types_from_renaming)?;
        result.entry(key).or_default().push(OcelExecution {
            central_object: id,
            log: execution,
        });
    }
    Ok(result)
}
