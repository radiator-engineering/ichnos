//! The object-centric event log (pm4py's `OCEL`, `objects/ocel/obj.py`).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use ichnos_core::{AttributeValue, Attributes};

/// One event of an object-centric event log.
#[derive(Debug, Clone, PartialEq)]
pub struct OcelEvent {
    /// The event id (pm4py's `ocel:eid` column).
    pub id: Arc<str>,
    /// The activity (pm4py's `ocel:activity` column).
    pub activity: Arc<str>,
    /// The timestamp (pm4py's `ocel:timestamp` column).
    pub timestamp: DateTime<FixedOffset>,
    /// The other attributes. A missing value has no entry.
    pub attributes: Attributes,
}

/// One object of an object-centric event log.
#[derive(Debug, Clone, PartialEq)]
pub struct OcelObject {
    /// The object id (pm4py's `ocel:oid` column).
    pub id: Arc<str>,
    /// The object type (pm4py's `ocel:type` column).
    pub object_type: Arc<str>,
    /// The other attributes. A missing value has no entry.
    pub attributes: Attributes,
}

/// A relation between an event and an object (a row of pm4py's
/// `relations` table).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventObject {
    /// The event id.
    pub event: Arc<str>,
    /// The object id.
    pub object: Arc<str>,
    /// The qualifier of the relation, if it has one. An empty qualifier is
    /// `Some("")`, which pm4py also counts as a qualifier.
    pub qualifier: Option<Arc<str>>,
}

/// A relation between two objects (a row of pm4py's `o2o` table).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectObject {
    /// The source object id.
    pub source: Arc<str>,
    /// The target object id.
    pub target: Arc<str>,
    /// The qualifier of the relation, if it has one.
    pub qualifier: Option<Arc<str>>,
}

/// A relation between two events (a row of pm4py's `e2e` table).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventEvent {
    /// The source event id.
    pub source: Arc<str>,
    /// The target event id.
    pub target: Arc<str>,
    /// The qualifier of the relation, if it has one.
    pub qualifier: Option<Arc<str>>,
}

/// A change of one object attribute at a point in time (a row of pm4py's
/// `object_changes` table).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectChange {
    /// The object id.
    pub object: Arc<str>,
    /// The object type.
    pub object_type: Arc<str>,
    /// When the attribute changed.
    pub timestamp: DateTime<FixedOffset>,
    /// The name of the changed attribute (pm4py's `ocel:field` column).
    pub field: Arc<str>,
    /// The new value, if there is one.
    pub value: Option<AttributeValue>,
}

/// An object-centric event log: events, objects and the relations between
/// them.
///
/// pm4py keeps each table as a pandas data frame whose column names are
/// parameters. Here each table is a list of typed rows, so the column names
/// matter only to the readers and writers.
///
/// pm4py's `relations` table repeats each event's activity and timestamp
/// and each object's type. [`EventObject`] keeps only the two ids and the
/// qualifier, and [`Ocel::summary`] and [`Ocel::extended_table`] look the
/// rest up in [`Ocel::events`] and [`Ocel::objects`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ocel {
    /// The events, in log order.
    pub events: Vec<OcelEvent>,
    /// The objects.
    pub objects: Vec<OcelObject>,
    /// The event-to-object relations.
    pub relations: Vec<EventObject>,
    /// The object-to-object relations (OCEL 2.0).
    pub o2o: Vec<ObjectObject>,
    /// The event-to-event relations.
    pub e2e: Vec<EventEvent>,
    /// The object attribute changes (OCEL 2.0).
    pub object_changes: Vec<ObjectChange>,
    /// Log-level metadata, such as OCEL 1.0's `ocel:global-log`.
    pub globals: Attributes,
}

/// The events with their related objects, grouped by object type (pm4py's
/// `OCEL.get_extended_table`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedTable {
    /// The object types, in the order they first appear in the relations.
    pub object_types: Vec<Arc<str>>,
    /// One row per event, in the order of [`Ocel::events`].
    pub rows: Vec<ExtendedRow>,
}

/// One row of an [`ExtendedTable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedRow {
    /// The index of the event in [`Ocel::events`].
    pub event: usize,
    /// The related object ids for each entry of
    /// [`ExtendedTable::object_types`], in relation order. A list is empty
    /// where pandas has a missing value.
    pub objects: Vec<Vec<Arc<str>>>,
}

impl Ocel {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the log uses OCEL 2.0 features (pm4py's `OCEL.is_ocel20`):
    /// object-to-object relations, object changes, or a relation with a
    /// qualifier.
    pub fn is_ocel20(&self) -> bool {
        !self.o2o.is_empty()
            || !self.object_changes.is_empty()
            || self.relations.iter().any(|r| r.qualifier.is_some())
    }

    /// The index of each event id in [`Ocel::events`]. When ids repeat, the
    /// first event wins.
    pub fn event_index(&self) -> HashMap<&str, usize> {
        let mut index = HashMap::with_capacity(self.events.len());
        for (i, e) in self.events.iter().enumerate() {
            index.entry(&*e.id).or_insert(i);
        }
        index
    }

    /// The index of each object id in [`Ocel::objects`]. When ids repeat,
    /// the first object wins.
    pub fn object_index(&self) -> HashMap<&str, usize> {
        let mut index = HashMap::with_capacity(self.objects.len());
        for (i, o) in self.objects.iter().enumerate() {
            index.entry(&*o.id).or_insert(i);
        }
        index
    }

    /// The events with their related objects, one list per object type
    /// (pm4py's `OCEL.get_extended_table`).
    ///
    /// The object types are those of the related objects, in the order they
    /// first appear in [`Ocel::relations`]. A relation whose event or
    /// object is not in the log is left out.
    pub fn extended_table(&self) -> ExtendedTable {
        let events = self.event_index();
        let objects = self.object_index();
        let mut object_types: Vec<Arc<str>> = Vec::new();
        let mut type_index: HashMap<&str, usize> = HashMap::new();
        let mut rows: Vec<ExtendedRow> = (0..self.events.len())
            .map(|event| ExtendedRow {
                event,
                objects: Vec::new(),
            })
            .collect();
        for r in &self.relations {
            let Some(&o) = objects.get(&*r.object) else {
                continue;
            };
            let object_type = &self.objects[o].object_type;
            let t = *type_index.entry(object_type).or_insert_with(|| {
                object_types.push(object_type.clone());
                object_types.len() - 1
            });
            let Some(&e) = events.get(&*r.event) else {
                continue;
            };
            let row = &mut rows[e].objects;
            if row.len() <= t {
                row.resize(t + 1, Vec::new());
            }
            row[t].push(r.object.clone());
        }
        for row in &mut rows {
            row.objects.resize(object_types.len(), Vec::new());
        }
        ExtendedTable { object_types, rows }
    }
}
