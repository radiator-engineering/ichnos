use std::collections::{BTreeMap, BTreeSet};

use chrono::DateTime;
use ichnos_ocel::{EventObject, Ocel, OcelEvent, OcelObject};

use super::*;
use crate::Error;

/// A log with the given events `(id, activity, objects)` and objects
/// `(id, type)`.
fn ocel(events: &[(&str, &str, &[&str])], objects: &[(&str, &str)]) -> Ocel {
    let mut log = Ocel::default();
    for (i, (id, activity, objs)) in events.iter().enumerate() {
        log.events.push(OcelEvent {
            id: (*id).into(),
            activity: (*activity).into(),
            timestamp: DateTime::from_timestamp(i64::try_from(i).unwrap(), 0)
                .unwrap()
                .fixed_offset(),
            attributes: Default::default(),
        });
        for o in *objs {
            log.relations.push(EventObject {
                event: (*id).into(),
                object: (*o).into(),
                qualifier: None,
            });
        }
    }
    for (id, ot) in objects {
        log.objects.push(OcelObject {
            id: (*id).into(),
            object_type: (*ot).into(),
            attributes: Default::default(),
        });
    }
    log
}

fn small() -> Ocel {
    ocel(
        &[
            ("e1", "a", &["o1", "o2"]),
            ("e2", "b", &["o2", "o3"]),
            ("e3", "c", &["o3"]),
            ("e4", "d", &[]),
        ],
        &[("o1", "A"), ("o2", "B"), ("o3", "A")],
    )
}

fn edge(source: &str, relation: ObjectGraphKind, target: &str) -> OtgEdge {
    OtgEdge {
        source: source.to_owned(),
        relation,
        target: target.to_owned(),
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_owned(), b.to_owned())
}

#[test]
fn object_type_graph_by_hand() {
    use ObjectGraphKind::*;
    let otg = discover_otg(&small()).unwrap();
    assert_eq!(otg.object_types, set(&["A", "B"]));
    let expected: BTreeMap<OtgEdge, u64> = [
        edge("A", Interaction, "B"),
        edge("B", Interaction, "A"),
        edge("B", Descendants, "A"),
        edge("A", Inheritance, "B"),
        edge("B", Inheritance, "A"),
        edge("A", Cobirth, "B"),
    ]
    .into_iter()
    .map(|e| (e, 1))
    .collect();
    assert_eq!(otg.edges, expected);
}

#[test]
fn inheritance_drops_pairs_that_go_both_ways() {
    use ObjectGraphKind::*;
    let log = ocel(&[("e1", "a", &["x", "y"])], &[("x", "T"), ("y", "T")]);
    let otg = discover_otg(&log).unwrap();
    let relations: Vec<ObjectGraphKind> = otg.edges.keys().map(|e| e.relation).collect();
    assert_eq!(relations, [Interaction, Cobirth, Codeath]);
}

#[test]
fn unknown_objects_are_errors() {
    let log = ocel(&[("e1", "a", &["o1", "o2"])], &[("o1", "A")]);
    assert!(matches!(discover_otg(&log), Err(Error::UnknownOcelObject(o)) if o == "o2"));
    assert!(matches!(
        discover_etot(&log),
        Err(Error::UnknownOcelObject(_))
    ));
}

#[test]
fn event_type_object_type_graph_by_hand() {
    let etot = discover_etot(&small()).unwrap();
    assert_eq!(etot.activities, set(&["a", "b", "c"]));
    assert_eq!(etot.object_types, set(&["A", "B"]));
    let expected: BTreeMap<(String, String), u64> = [
        (pair("a", "A"), 1),
        (pair("a", "B"), 1),
        (pair("b", "A"), 1),
        (pair("b", "B"), 1),
        (pair("c", "A"), 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(etot.edges, expected);
}

#[test]
fn ocdfg_measures_by_hand() {
    let m = OcdfgMeasures::from_ocel(&small()).unwrap();
    assert_eq!(m.activities, set(&["a", "b", "c", "d"]));
    let events: BTreeMap<String, u64> = [("a", 1), ("b", 1), ("c", 1)]
        .into_iter()
        .map(|(a, n)| (a.to_owned(), n))
        .collect();
    assert_eq!(m.events, events);
    let flows: BTreeMap<(String, String), u64> = [(pair("a", "b"), 1), (pair("b", "c"), 1)]
        .into_iter()
        .collect();
    assert_eq!(m.flows, flows);
}

#[test]
fn an_empty_model_has_no_fitness() {
    let real = discover_otg(&small()).unwrap();
    let err = conformance_otg(&real, &Otg::default(), &OtgConformanceOptions::default());
    assert!(matches!(err, Err(Error::ZeroNormalization)));
    let real = discover_etot(&small()).unwrap();
    let err = conformance_etot(&real, &Etot::default(), &EtotConformanceOptions::default());
    assert!(matches!(err, Err(Error::ZeroNormalization)));
    // The OC-DFG comparison gives 1 instead, as pm4py does.
    let empty = OcdfgMeasures::default();
    let d = conformance_ocdfg(&empty, &empty, &OcdfgConformanceOptions::default());
    assert!((d.fitness - 1.0).abs() < f64::EPSILON);
}

#[test]
fn a_zero_model_frequency_is_an_error_for_etot_only() {
    let real = discover_etot(&small()).unwrap();
    let mut model = real.clone();
    model.edges.insert(pair("a", "A"), 0);
    let err = conformance_etot(&real, &model, &EtotConformanceOptions::default());
    assert!(matches!(err, Err(Error::ZeroEdgeFrequency { .. })));

    let real = discover_otg(&small()).unwrap();
    let mut model = real.clone();
    let first = model.edges.keys().next().unwrap().clone();
    model.edges.insert(first.clone(), 0);
    let d = conformance_otg(&real, &model, &OtgConformanceOptions::default()).unwrap();
    assert_eq!(d.non_conforming_edges[&first], f64::INFINITY);
    // 6 edges and 2 types: 1 - 1 / (2 + 6 + 6).
    assert!((d.fitness - (1.0 - 1.0 / 14.0)).abs() < 1e-12);
}
