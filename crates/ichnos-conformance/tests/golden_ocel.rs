//! Object-centric conformance against the pm4py goldens
//! `conformance/ocel-{otg,etot,ocdfg}-*`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ichnos_conformance::ocel::{
    Etot, EtotConformanceOptions, ObjectRelation, OcdfgConformanceOptions, OcdfgMeasures, Otg,
    OtgConformanceOptions, OtgEdge, conformance_etot, conformance_etot_ocel, conformance_ocdfg,
    conformance_ocdfg_ocel, conformance_otg, conformance_otg_ocel, discover_etot, discover_otg,
};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_ocel::Ocel;
use serde_json::{Value, json};

const LOGS: [&str; 2] = ["example-log", "ocel20-example"];

fn read(path: &Path) -> Ocel {
    if path.to_string_lossy().contains("ocel20") {
        ichnos_io::read_ocel2_json(path).unwrap()
    } else {
        ichnos_io::read_ocel_json(path).unwrap()
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap().to_owned()
}

fn strings(v: &Value) -> BTreeSet<String> {
    v.as_array().unwrap().iter().map(s).collect()
}

fn relation(name: &str) -> ObjectRelation {
    ObjectRelation::ALL
        .into_iter()
        .find(|r| r.as_str() == name)
        .unwrap()
}

fn edge_of(v: &Value) -> OtgEdge {
    OtgEdge {
        source: s(&v["source"]),
        relation: relation(v["relation"].as_str().unwrap()),
        target: s(&v["target"]),
    }
}

fn otg_from(v: &Value) -> Otg {
    Otg {
        object_types: strings(&v["object_types"]),
        edges: v["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (edge_of(e), e["count"].as_u64().unwrap()))
            .collect(),
    }
}

fn edge_json(e: &OtgEdge) -> Value {
    json!({"source": e.source, "relation": e.relation.as_str(), "target": e.target})
}

fn etot_from(v: &Value) -> Etot {
    Etot {
        activities: strings(&v["activities"]),
        object_types: strings(&v["object_types"]),
        edges: v["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    (s(&e["activity"]), s(&e["object_type"])),
                    e["count"].as_u64().unwrap(),
                )
            })
            .collect(),
    }
}

fn measures_from(v: &Value) -> OcdfgMeasures {
    OcdfgMeasures {
        activities: strings(&v["activities"]),
        events: v["events"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(a, n)| (a.clone(), n.as_u64().unwrap()))
            .collect(),
        flows: v["flows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    (s(&f["source"]), s(&f["target"])),
                    f["count"].as_u64().unwrap(),
                )
            })
            .collect(),
    }
}

fn measures_json(m: &OcdfgMeasures) -> Value {
    json!({
        "activities": m.activities,
        "events": m.events,
        "flows": m.flows.iter()
            .map(|((a, b), n)| json!({"source": a, "target": b, "count": n}))
            .collect::<Vec<_>>(),
    })
}

fn pairs(set: &BTreeSet<(String, String)>) -> Value {
    set.iter().map(|(a, b)| json!([a, b])).collect()
}

fn check(id: &str, actual: &Value, expected: &Value) {
    let opts = JsonCompare::default().unordered();
    for (key, value) in expected.as_object().unwrap() {
        assert!(actual.get(key).is_some(), "{id}: no {key}");
        assert_json_eq(&actual[key], value, &opts);
    }
}

/// The cases of one kind: id, whether the graph of the first half is the
/// real side, and whether the options are the non-default ones.
fn cases(kind: &str) -> Vec<(String, bool, bool)> {
    let mut out = Vec::new();
    for log in LOGS {
        out.push((format!("ocel-{kind}-{log}"), false, false));
        out.push((format!("ocel-{kind}-{log}-reversed"), true, false));
    }
    out.push((format!("ocel-{kind}-example-log-options"), false, true));
    out
}

#[test]
fn oracle_conformance_otg() {
    for (id, reversed, custom) in cases("otg") {
        let g = golden("conformance", &id);
        let e: Value = g.expected_as();
        let ocel = read(&g.fixture("log"));
        let mut options = OtgConformanceOptions::default();
        if custom {
            options.theta = BTreeMap::from([
                (ObjectRelation::Interaction, 0.5),
                (ObjectRelation::Cobirth, 0.0),
            ]);
            options.alpha = 2.0;
            options.gamma = 0.5;
        }
        let d = if reversed {
            let real = otg_from(&e["real"]);
            conformance_otg(&real, &discover_otg(&ocel).unwrap(), &options).unwrap()
        } else {
            conformance_otg_ocel(&ocel, &otg_from(&e["model"]), &options).unwrap()
        };
        let actual = json!({
            "missing_object_types": d.missing_object_types,
            "additional_object_types": d.additional_object_types,
            "missing_edges": d.missing_edges.iter().map(edge_json).collect::<Vec<_>>(),
            "additional_edges": d.additional_edges.iter().map(edge_json).collect::<Vec<_>>(),
            "non_conforming_edges": d.non_conforming_edges.iter().map(|(e, v)| {
                let mut j = edge_json(e);
                j["delta"] = json!(v);
                j
            }).collect::<Vec<_>>(),
            "fitness": d.fitness,
        });
        let mut expected = e.clone();
        let map = expected.as_object_mut().unwrap();
        map.remove("real");
        map.remove("model");
        check(&id, &actual, &expected);
    }
}

#[test]
fn oracle_conformance_etot() {
    for (id, reversed, custom) in cases("etot") {
        let g = golden("conformance", &id);
        let e: Value = g.expected_as();
        let ocel = read(&g.fixture("log"));
        let mut options = EtotConformanceOptions::default();
        if custom {
            options.theta_rel = 0.5;
            options.alpha = 2.0;
            options.beta = 0.5;
        }
        let d = if reversed {
            let real = etot_from(&e["real"]);
            conformance_etot(&real, &discover_etot(&ocel).unwrap(), &options).unwrap()
        } else {
            conformance_etot_ocel(&ocel, &etot_from(&e["model"]), &options).unwrap()
        };
        let actual = json!({
            "activities_missing": d.activities_missing,
            "activities_additional": d.activities_additional,
            "object_types_missing": d.object_types_missing,
            "object_types_additional": d.object_types_additional,
            "edges_missing": pairs(&d.edges_missing),
            "edges_additional": pairs(&d.edges_additional),
            "delta_rel": d.delta_rel.iter()
                .map(|((a, ot), v)| json!({"activity": a, "object_type": ot, "delta": v}))
                .collect::<Vec<_>>(),
            "fitness": d.fitness,
        });
        let mut expected = e.clone();
        let map = expected.as_object_mut().unwrap();
        map.remove("real");
        map.remove("model");
        check(&id, &actual, &expected);
    }
}

#[test]
fn oracle_conformance_ocdfg() {
    for (id, reversed, custom) in cases("ocdfg") {
        let g = golden("conformance", &id);
        let e: Value = g.expected_as();
        let ocel = read(&g.fixture("log"));
        let mut options = OcdfgConformanceOptions::default();
        if custom {
            options.theta_act = 1.0;
            options.theta_flow = 1.0;
            options.alpha = 2.0;
            options.delta = 0.5;
        }
        let log = OcdfgMeasures::from_ocel(&ocel).unwrap();
        let d = if reversed {
            conformance_ocdfg(&measures_from(&e["real"]), &log, &options)
        } else {
            check(
                &id,
                &json!({"log": measures_json(&log)}),
                &json!({"log": e["log"]}),
            );
            conformance_ocdfg_ocel(&ocel, &measures_from(&e["model"]), &options).unwrap()
        };
        let actual = json!({
            "missing_activities": d.missing_activities,
            "additional_activities": d.additional_activities,
            "missing_flows": pairs(&d.missing_flows),
            "additional_flows": pairs(&d.additional_flows),
            "activity_measure_differences": d.activity_measure_differences,
            "non_conforming_activities_in_measure": d.non_conforming_activities_in_measure,
            "flow_measure_differences": d.flow_measure_differences.iter()
                .map(|((a, b), n)| json!({"source": a, "target": b, "difference": n}))
                .collect::<Vec<_>>(),
            "non_conforming_flows_in_measure": pairs(&d.non_conforming_flows_in_measure),
            "fitness": d.fitness,
        });
        let mut expected = e.clone();
        let map = expected.as_object_mut().unwrap();
        map.remove("real");
        map.remove("model");
        map.remove("log");
        check(&id, &actual, &expected);
    }
}
