//! The graphs that object-centric conformance compares: object-type graphs
//! (pm4py's `algo/discovery/ocel/otg`), event type–object type graphs
//! (`algo/discovery/ocel/etot`) and the measures of an object-centric DFG
//! (`algo/discovery/ocel/ocdfg`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use ichnos_ocel::{ObjectGraphKind, Ocel, discover_objects_graph};

use crate::{Error, Result};

/// An edge of an object-type graph: objects of type `source` relate to
/// objects of type `target` by `relation`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OtgEdge {
    /// The type of the first object.
    pub source: String,
    /// The relation between the objects.
    pub relation: ObjectGraphKind,
    /// The type of the second object.
    pub target: String,
}

/// An object-type graph (pm4py's OTG tuple).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Otg {
    /// The object types.
    pub object_types: BTreeSet<String>,
    /// How many pairs of objects each edge stands for.
    pub edges: BTreeMap<OtgEdge, u64>,
}

/// An event type–object type graph (pm4py's ET-OT tuple).
///
/// pm4py keeps the set of edges and their frequencies apart; here the
/// edges are the keys of [`Etot::edges`], which is what pm4py's discovery
/// gives.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Etot {
    /// The activities.
    pub activities: BTreeSet<String>,
    /// The object types.
    pub object_types: BTreeSet<String>,
    /// How many event–object relations link each `(activity, object type)`
    /// pair.
    pub edges: BTreeMap<(String, String), u64>,
}

/// The measures of an object-centric DFG that [`conformance_ocdfg`]
/// compares, from the dictionary of pm4py's `discover_ocdfg`.
///
/// ichnos-ocel does not port `discover_ocdfg` yet; this type can switch to
/// its result when it does.
///
/// [`conformance_ocdfg`]: super::conformance_ocdfg
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OcdfgMeasures {
    /// The activities (pm4py's `activities`).
    pub activities: BTreeSet<String>,
    /// The events of each activity that relate to some object (the sizes of
    /// pm4py's `activities_indep["events"]`). A missing activity has 0.
    pub events: BTreeMap<String, u64>,
    /// The directly-following event pairs of each flow, summed over the
    /// object types (the sizes of pm4py's `edges["event_couples"]`). The
    /// keys are the flows.
    pub flows: BTreeMap<(String, String), u64>,
}

/// An object type and the activities of a flow.
type Flow<'a> = (&'a str, &'a str, &'a str);

/// Lookups shared by the discoveries.
struct Index<'a> {
    activity: HashMap<&'a str, &'a str>,
    object_type: HashMap<&'a str, &'a str>,
    /// The objects of each event, in relation order, with repeats.
    objects: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Index<'a> {
    fn new(ocel: &'a Ocel) -> Self {
        let mut activity = HashMap::new();
        for e in &ocel.events {
            activity.entry(&*e.id).or_insert(&*e.activity);
        }
        let mut object_type = HashMap::new();
        for o in &ocel.objects {
            object_type.entry(&*o.id).or_insert(&*o.object_type);
        }
        let mut objects: HashMap<&str, Vec<&str>> = HashMap::new();
        for r in &ocel.relations {
            objects.entry(&*r.event).or_default().push(&*r.object);
        }
        Index {
            activity,
            object_type,
            objects,
        }
    }

    fn activity(&self, event: &str) -> Result<&'a str> {
        self.activity
            .get(event)
            .copied()
            .ok_or_else(|| Error::UnknownOcelEvent(event.to_owned()))
    }

    fn object_type(&self, object: &str) -> Result<&'a str> {
        self.object_type
            .get(object)
            .copied()
            .ok_or_else(|| Error::UnknownOcelObject(object.to_owned()))
    }
}

/// The object-type graph of a log, as pm4py's `discover_otg` computes it.
///
/// For each [`ObjectGraphKind`], each pair of related objects adds 1 to the
/// edge between their types. The object types are those of the objects
/// table.
///
/// The events are taken in log order. pm4py fails on an event without
/// objects; here it relates no objects.
///
/// # Errors
///
/// [`Error::UnknownOcelObject`] when a relation names an object that is not
/// in the objects table.
pub fn discover_otg(ocel: &Ocel) -> Result<Otg> {
    let index = Index::new(ocel);
    let mut edges: BTreeMap<OtgEdge, u64> = BTreeMap::new();
    for relation in ObjectGraphKind::ALL {
        for (o1, o2) in discover_objects_graph(ocel, relation) {
            let edge = OtgEdge {
                source: index.object_type(&o1)?.to_owned(),
                relation,
                target: index.object_type(&o2)?.to_owned(),
            };
            *edges.entry(edge).or_default() += 1;
        }
    }
    Ok(Otg {
        object_types: ocel
            .objects
            .iter()
            .map(|o| o.object_type.to_string())
            .collect(),
        edges,
    })
}

/// The event type–object type graph of a log, as pm4py's `discover_etot`
/// computes it.
///
/// Each event–object relation adds 1 to the edge between the event's
/// activity and the object's type. The activities and object types are
/// those that some relation names.
///
/// # Errors
///
/// [`Error::UnknownOcelEvent`] or [`Error::UnknownOcelObject`] when a
/// relation names an event or object that is not in its table.
pub fn discover_etot(ocel: &Ocel) -> Result<Etot> {
    let index = Index::new(ocel);
    let mut etot = Etot::default();
    for r in &ocel.relations {
        let a = index.activity(&r.event)?;
        let ot = index.object_type(&r.object)?;
        etot.activities.insert(a.to_owned());
        etot.object_types.insert(ot.to_owned());
        *etot.edges.entry((a.to_owned(), ot.to_owned())).or_default() += 1;
    }
    Ok(etot)
}

impl OcdfgMeasures {
    /// The measures of a log's object-centric DFG, as pm4py's
    /// `discover_ocdfg` computes them.
    ///
    /// The activities are those of the events table. An activity's events
    /// are its events that relate to some object. An object's events, in
    /// log order, give its event couples; a pair of events counts once for
    /// each object type that links them.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownOcelEvent`] or [`Error::UnknownOcelObject`] when a
    /// relation names an event or object that is not in its table.
    pub fn from_ocel(ocel: &Ocel) -> Result<OcdfgMeasures> {
        let index = Index::new(ocel);
        let mut events: BTreeMap<&str, HashSet<&str>> = BTreeMap::new();
        for r in &ocel.relations {
            events
                .entry(index.activity(&r.event)?)
                .or_default()
                .insert(&r.event);
        }
        let mut history: HashMap<&str, &str> = HashMap::new();
        // The event pairs of each (object type, source activity, target activity).
        let mut couples: HashMap<Flow<'_>, HashSet<(&str, &str)>> = HashMap::new();
        for e in &ocel.events {
            let Some(objects) = index.objects.get(&*e.id) else {
                continue;
            };
            let mut done = HashSet::new();
            for &o in objects {
                if !done.insert(o) {
                    continue;
                }
                if let Some(prev) = history.insert(o, &e.id) {
                    let key = (index.object_type(o)?, index.activity(prev)?, &*e.activity);
                    couples.entry(key).or_default().insert((prev, &e.id));
                }
            }
        }
        let mut flows: BTreeMap<(String, String), u64> = BTreeMap::new();
        for ((_, a, b), pairs) in couples {
            *flows.entry((a.to_owned(), b.to_owned())).or_default() += pairs.len() as u64;
        }
        Ok(OcdfgMeasures {
            activities: ocel.events.iter().map(|e| e.activity.to_string()).collect(),
            events: events
                .into_iter()
                .map(|(a, evs)| (a.to_owned(), evs.len() as u64))
                .collect(),
            flows,
        })
    }
}
