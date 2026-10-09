//! Object-centric activity, edge and object-type statistics.
//!
//! Activity endpoints follow relation order; edges follow event order.
//! Identifiers and map/set iteration are ordered deterministically.

use crate::{Error, Result};
use ichnos_ocel::Ocel;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub mod act_ot_dependent;
pub mod act_utils;
pub mod edge_metrics;
pub mod objects_ot_count;
pub mod ot_activities;

/// Event, object, activity or object-type identifier.
pub type Id = Arc<str>;
/// Event/object association.
pub type Association = (Id, Id);
/// Ordered occurrences for each activity.
pub type ActivityAssociations = BTreeMap<Id, Vec<Association>>;
/// Unique identifiers for each activity.
pub type ActivityIdentifiers = BTreeMap<Id, BTreeSet<Id>>;
/// Activity associations grouped by object type.
pub type TypedActivityAssociations = BTreeMap<Id, ActivityAssociations>;
/// Unique identifiers per object type and activity.
pub type TypedActivityIdentifiers = BTreeMap<Id, ActivityIdentifiers>;
/// Source/target activity pair.
pub type ActivityEdge = (Id, Id);
/// An edge metric grouped by object type and activity pair.
pub type EdgeMetric<T> = BTreeMap<Id, BTreeMap<ActivityEdge, T>>;
/// Source event, target event and object occurrence.
pub type EdgeOccurrence = (Id, Id, Id);
/// Edge occurrences, preserving event and relation order.
pub type EdgeAssociations = EdgeMetric<Vec<EdgeOccurrence>>;

/// Optional per-object activity endpoint selection after pair deduplication.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Prefilter {
    /// Keep every distinct event/object pair, in relation order.
    #[default]
    None,
    /// Keep each object's first relation, ordered by object id.
    Start,
    /// Keep each object's last relation, ordered by object id.
    End,
}

/// A typed row for activity association queries without an OCEL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityRelation {
    /// Event identifier.
    pub event: Id,
    /// Object identifier.
    pub object: Id,
    /// Activity of the relation.
    pub activity: Id,
}

fn relation_rows(log: &Ocel) -> Result<Vec<(ActivityRelation, Id)>> {
    let events = log.event_index();
    let objects = log.object_index();
    log.relations
        .iter()
        .map(|r| {
            let e = events
                .get(r.event.as_ref())
                .ok_or_else(|| Error::MissingOcelEvent(r.event.to_string()))?;
            let o = objects
                .get(r.object.as_ref())
                .ok_or_else(|| Error::MissingOcelObject(r.object.to_string()))?;
            Ok((
                ActivityRelation {
                    event: r.event.clone(),
                    object: r.object.clone(),
                    activity: log.events[*e].activity.clone(),
                },
                log.objects[*o].object_type.clone(),
            ))
        })
        .collect()
}
