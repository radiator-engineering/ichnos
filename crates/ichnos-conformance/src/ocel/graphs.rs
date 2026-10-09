//! The graphs that object-centric conformance compares: object-type graphs
//! (pm4py's `algo/discovery/ocel/otg`), event type–object type graphs
//! (`algo/discovery/ocel/etot`) and the measures of an object-centric DFG
//! (`algo/discovery/ocel/ocdfg`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use ichnos_ocel::Ocel;

use crate::{Error, Result};

/// How two objects are related in an object graph (pm4py's `graph_type`
/// of `discover_objects_graph`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectRelation {
    /// Both objects are related to some event (`object_interaction`).
    Interaction,
    /// The second object first appears in an event that the first object
    /// was already part of (`object_descendants`).
    Descendants,
    /// The first object's last event is the second object's first event
    /// (`object_inheritance`).
    Inheritance,
    /// Both objects first appear in the same event (`object_cobirth`).
    Cobirth,
    /// Both objects last appear in the same event (`object_codeath`).
    Codeath,
}

impl ObjectRelation {
    /// Every relation, in pm4py's order.
    pub const ALL: [ObjectRelation; 5] = [
        ObjectRelation::Interaction,
        ObjectRelation::Descendants,
        ObjectRelation::Inheritance,
        ObjectRelation::Cobirth,
        ObjectRelation::Codeath,
    ];

    /// pm4py's name of the relation, such as `object_interaction`.
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectRelation::Interaction => "object_interaction",
            ObjectRelation::Descendants => "object_descendants",
            ObjectRelation::Inheritance => "object_inheritance",
            ObjectRelation::Cobirth => "object_cobirth",
            ObjectRelation::Codeath => "object_codeath",
        }
    }
}

impl fmt::Display for ObjectRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An edge of an object-type graph: objects of type `source` relate to
/// objects of type `target` by `relation`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OtgEdge {
    /// The type of the first object.
    pub source: String,
    /// The relation between the objects.
    pub relation: ObjectRelation,
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

    /// The objects of each event, in log order. An event without objects
    /// has none.
    fn lifecycle_order(&self, ocel: &Ocel) -> Vec<&[&'a str]> {
        ocel.events
            .iter()
            .map(|e| self.objects.get(&*e.id).map_or(&[][..], Vec::as_slice))
            .collect()
    }
}

/// Splits each event's objects into those seen in an earlier event and
/// those seen for the first time, in the given order.
fn seen_unseen<'s, 'a: 's>(
    order: impl Iterator<Item = &'s [&'a str]>,
    mut visit: impl FnMut(&BTreeSet<&'a str>, &BTreeSet<&'a str>),
) {
    let mut set_objects: HashSet<&str> = HashSet::new();
    for objects in order {
        let (seen, unseen): (BTreeSet<&str>, BTreeSet<&str>) =
            objects.iter().partition(|o| set_objects.contains(*o));
        visit(&seen, &unseen);
        set_objects.extend(unseen);
    }
}

fn unordered_pairs<'a>(objects: &BTreeSet<&'a str>, graph: &mut BTreeSet<(&'a str, &'a str)>) {
    for &o1 in objects {
        for &o2 in objects {
            if o1 < o2 {
                graph.insert((o1, o2));
            }
        }
    }
}

/// One object graph, as pm4py's `algo/transformation/ocel/graphs` computes
/// it.
fn object_graph<'s, 'a: 's>(
    index: &'s Index<'a>,
    order: &[&'s [&'a str]],
    relation: ObjectRelation,
) -> BTreeSet<(&'a str, &'a str)> {
    let mut graph = BTreeSet::new();
    match relation {
        ObjectRelation::Interaction => {
            for objects in index.objects.values() {
                for &o1 in objects {
                    for &o2 in objects {
                        if o1 < o2 {
                            graph.insert((o1, o2));
                        }
                    }
                }
            }
        }
        ObjectRelation::Descendants => seen_unseen(order.iter().copied(), |seen, unseen| {
            for &o1 in seen {
                for &o2 in unseen {
                    graph.insert((o1, o2));
                }
            }
        }),
        ObjectRelation::Cobirth => seen_unseen(order.iter().copied(), |_, unseen| {
            unordered_pairs(unseen, &mut graph);
        }),
        ObjectRelation::Codeath => seen_unseen(order.iter().rev().copied(), |_, unseen| {
            unordered_pairs(unseen, &mut graph);
        }),
        ObjectRelation::Inheritance => {
            // The last event of each object, by its index in the order.
            let mut last: HashMap<&str, usize> = HashMap::new();
            for (i, objects) in order.iter().enumerate() {
                for &o in *objects {
                    last.insert(o, i);
                }
            }
            let mut i = 0;
            seen_unseen(order.iter().copied(), |_, unseen| {
                let ending: BTreeSet<&str> =
                    order[i].iter().copied().filter(|o| last[o] == i).collect();
                for &o2 in unseen {
                    for &o1 in &ending {
                        if o1 != o2 {
                            graph.insert((o1, o2));
                        }
                    }
                }
                i += 1;
            });
            // pm4py drops the pairs that go both ways.
            let both: Vec<(&str, &str)> = graph
                .iter()
                .filter(|(a, b)| graph.contains(&(*b, *a)))
                .copied()
                .collect();
            for pair in both {
                graph.remove(&pair);
            }
        }
    }
    graph
}

/// The object-type graph of a log, as pm4py's `discover_otg` computes it.
///
/// For each [`ObjectRelation`], each pair of related objects adds 1 to the
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
    let order = index.lifecycle_order(ocel);
    let mut edges: BTreeMap<OtgEdge, u64> = BTreeMap::new();
    for relation in ObjectRelation::ALL {
        for (o1, o2) in object_graph(&index, &order, relation) {
            let edge = OtgEdge {
                source: index.object_type(o1)?.to_owned(),
                relation,
                target: index.object_type(o2)?.to_owned(),
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
