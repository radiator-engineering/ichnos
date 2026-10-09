//! Object graphs, ported from pm4py's `algo/transformation/ocel/graphs`.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::Ocel;

/// The kinds of object graph of pm4py's `discover_objects_graph`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectGraphKind {
    /// Objects related to the same event (undirected).
    Interaction,
    /// From each object already seen to each object that an event relates
    /// for the first time (directed).
    Descendants,
    /// From an object whose last event is `e` to an object whose first
    /// event is `e` (directed).
    Inheritance,
    /// Objects whose first event is the same (undirected).
    Cobirth,
    /// Objects whose last event is the same (undirected).
    Codeath,
}

impl ObjectGraphKind {
    /// Every kind, in the order of pm4py's documentation.
    pub const ALL: [ObjectGraphKind; 5] = [
        ObjectGraphKind::Interaction,
        ObjectGraphKind::Descendants,
        ObjectGraphKind::Inheritance,
        ObjectGraphKind::Cobirth,
        ObjectGraphKind::Codeath,
    ];

    /// pm4py's name of the graph type, such as `object_interaction`.
    pub fn name(self) -> &'static str {
        match self {
            ObjectGraphKind::Interaction => "object_interaction",
            ObjectGraphKind::Descendants => "object_descendants",
            ObjectGraphKind::Inheritance => "object_inheritance",
            ObjectGraphKind::Cobirth => "object_cobirth",
            ObjectGraphKind::Codeath => "object_codeath",
        }
    }

    /// The kind with pm4py's name, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// An object graph: a set of `(object, object)` pairs. An undirected graph
/// holds each pair once, smaller id first.
pub type ObjectGraph = BTreeSet<(String, String)>;

/// The object graph of `kind`, as pm4py's `discover_objects_graph` computes
/// it.
///
/// The graphs that follow object lifecycles walk the events in the order of
/// `ocel.events`. pm4py fails on an event without related objects; such an
/// event relates no objects here.
pub fn discover_objects_graph(ocel: &Ocel, kind: ObjectGraphKind) -> ObjectGraph {
    let mut by_event: HashMap<&str, Vec<&str>> = HashMap::new();
    for r in &ocel.relations {
        by_event.entry(&r.event).or_default().push(&r.object);
    }
    let mut graph = ObjectGraph::new();
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    if kind == ObjectGraphKind::Interaction {
        for objects in by_event.values() {
            unordered(&mut graph, &objects.iter().copied().collect());
        }
        return graph;
    }
    let empty = Vec::new();
    let events: Vec<(&str, &Vec<&str>)> = ocel
        .events
        .iter()
        .map(|e| (&*e.id, by_event.get(&*e.id).unwrap_or(&empty)))
        .collect();
    match kind {
        ObjectGraphKind::Interaction => unreachable!(),
        ObjectGraphKind::Cobirth => walk(events.iter(), |_, _, new| unordered(&mut graph, new)),
        ObjectGraphKind::Codeath => {
            walk(events.iter().rev(), |_, _, new| unordered(&mut graph, new));
        }
        ObjectGraphKind::Descendants => walk(events.iter(), |_, old, new| {
            for &a in old {
                for &b in new {
                    graph.insert(pair(a, b));
                }
            }
        }),
        ObjectGraphKind::Inheritance => {
            let mut last: HashMap<&str, &str> = HashMap::new();
            walk(events.iter().rev(), |id, _, new| {
                for &o in new {
                    last.insert(o, id);
                }
            });
            walk(events.iter(), |id, old, new| {
                let ending = old.iter().chain(new).filter(|o| last.get(*o) == Some(&id));
                let ending: BTreeSet<&str> = ending.copied().collect();
                for &b in new {
                    for &a in &ending {
                        if a != b {
                            graph.insert(pair(a, b));
                        }
                    }
                }
            });
            // pm4py drops the pairs that hold both ways.
            let both: Vec<(String, String)> = graph
                .iter()
                .filter(|(a, b)| graph.contains(&(b.clone(), a.clone())))
                .cloned()
                .collect();
            for p in both {
                graph.remove(&p);
            }
        }
    }
    graph
}

/// Walks `events`, splitting the objects of each event into those an
/// earlier event relates and the new ones, each without repeats.
fn walk<'a>(
    events: impl Iterator<Item = &'a (&'a str, &'a Vec<&'a str>)>,
    mut visit: impl FnMut(&'a str, &BTreeSet<&'a str>, &BTreeSet<&'a str>),
) {
    let mut seen: HashSet<&str> = HashSet::new();
    for &(id, objects) in events {
        let old: BTreeSet<&str> = objects
            .iter()
            .copied()
            .filter(|o| seen.contains(o))
            .collect();
        let new: BTreeSet<&str> = objects
            .iter()
            .copied()
            .filter(|o| !old.contains(o))
            .collect();
        visit(id, &old, &new);
        seen.extend(new.iter().copied());
    }
}

/// Adds each pair of `objects`, smaller id first.
fn unordered(graph: &mut ObjectGraph, objects: &BTreeSet<&str>) {
    for &a in objects {
        for &b in objects {
            if a < b {
                graph.insert((a.to_owned(), b.to_owned()));
            }
        }
    }
}
