//! Conversions against the pm4py goldens in `fixtures/golden/convert`:
//! `convert_log_to_ocel`, `convert_ocel_to_networkx` and
//! `discover_objects_graph`.

mod common;

use chrono::Utc;
use ichnos_core::AttributeValue;
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_ocel::{
    LogToOcelOptions, ObjectGraphKind, Ocel, OcelFeaturesToNxOptions, OcelGraph, OcelToNxOptions,
    convert_log_to_ocel, convert_ocel_features_to_networkx, convert_ocel_to_networkx,
    discover_objects_graph,
};
use serde_json::{Value, json};

fn value(v: &AttributeValue) -> Value {
    match v.plain() {
        AttributeValue::Date(d) => json!([
            "date",
            d.with_timezone(&Utc)
                .format("%Y-%m-%dT%H:%M:%S%.6f+00:00")
                .to_string()
        ]),
        AttributeValue::Bool(b) => json!(b),
        AttributeValue::Int(i) => json!(i),
        AttributeValue::Float(f) => json!(f),
        other => json!(other.to_string()),
    }
}

fn attrs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a AttributeValue)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_owned(), value(v)))
            .collect(),
    )
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_owned())
        .collect()
}

fn read(path: &std::path::Path) -> Ocel {
    if path.to_string_lossy().contains("ocel20") {
        ichnos_io::read_ocel2_json(path).unwrap()
    } else {
        ichnos_io::read_ocel_json(path).unwrap()
    }
}

#[test]
fn log_to_ocel_matches_pm4py() {
    let log = common::load("running-example");
    for case in [
        "log-to-ocel-running-example",
        "log-to-ocel-running-example-attributes",
        "log-to-ocel-running-example-two-types",
        "log-to-ocel-running-example-separator",
    ] {
        let g = golden("convert", case);
        let p = &g.meta().params;
        let mut o = LogToOcelOptions::default();
        if let Some(v) = p.get("object_types") {
            o.object_types = Some(strings(v));
        }
        if let Some(v) = p.get("obj_separator") {
            o.obj_separator = v.as_str().unwrap().to_owned();
        }
        if let Some(v) = p.get("additional_event_attributes") {
            o.additional_event_attributes = strings(v);
        }
        if let Some(v) = p.get("additional_object_attributes") {
            o.additional_object_attributes = v
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), strings(v)))
                .collect();
        }
        let ocel = convert_log_to_ocel(&log, &o).unwrap();
        let types: std::collections::HashMap<&str, &str> = ocel
            .objects
            .iter()
            .map(|x| (&*x.id, &*x.object_type))
            .collect();
        let actual = json!({
            "events": ocel.events.iter().map(|e| json!([
                &*e.id, &*e.activity, value(&AttributeValue::Date(e.timestamp)),
                attrs(e.attributes.iter().map(|(k, v)| (&**k, v))),
            ])).collect::<Vec<_>>(),
            "objects": ocel.objects.iter().map(|x| json!([
                &*x.id, &*x.object_type, attrs(x.attributes.iter().map(|(k, v)| (&**k, v))),
            ])).collect::<Vec<_>>(),
            "relations": ocel.relations.iter().map(|r| json!([
                &*r.event, &*r.object, types[&*r.object],
            ])).collect::<Vec<_>>(),
        });
        assert_json_eq(&actual, &g.expected, &JsonCompare::default());
    }
}

fn graph_json(g: &OcelGraph, sort_nodes: bool) -> Value {
    let row = |v: &Value| serde_json::to_string(v).unwrap();
    let mut nodes: Vec<Value> = g
        .nodes
        .iter()
        .map(|n| {
            json!([
                n.id,
                attrs(n.attributes.iter().map(|(k, v)| (k.as_str(), v)))
            ])
        })
        .collect();
    if sort_nodes {
        nodes.sort_by_key(row);
    }
    let mut edges: Vec<Value> = g
        .edges
        .iter()
        .map(|e| {
            json!([
                e.source,
                e.target,
                attrs(e.attributes.iter().map(|(k, v)| (k.as_str(), v)))
            ])
        })
        .collect();
    edges.sort_by_key(row);
    json!({"multigraph": g.multigraph, "nodes": nodes, "edges": edges})
}

#[test]
fn ocel_graphs_match_pm4py() {
    for name in ["example-log", "ocel20-example"] {
        let g = golden("convert", &format!("networkx-{name}-ocel-to-nx"));
        let ocel = read(&g.fixture("log"));
        let graph = convert_ocel_to_networkx(&ocel, &OcelToNxOptions::default());
        // serde_json sorts object keys, as Python's sort_keys does.
        assert_json_eq(
            &graph_json(&graph, false),
            &g.expected,
            &JsonCompare::default(),
        );

        let g = golden("convert", &format!("networkx-{name}-ocel-features-to-nx"));
        let graph = convert_ocel_features_to_networkx(&ocel, &OcelFeaturesToNxOptions::default());
        assert_json_eq(
            &graph_json(&graph, true),
            &g.expected,
            &JsonCompare::default(),
        );

        for kind in ObjectGraphKind::ALL {
            let case = format!("objects-graph-{name}-{}", kind.name().replace('_', "-"));
            let g = golden("convert", &case);
            let pairs: Vec<Value> = discover_objects_graph(&ocel, kind)
                .into_iter()
                .map(|(a, b)| json!([a, b]))
                .collect();
            assert_json_eq(&json!(pairs), &g.expected, &JsonCompare::default());
            assert_eq!(ObjectGraphKind::from_name(kind.name()), Some(kind));
        }
    }
}

#[test]
fn values_that_are_not_strings_relate_no_objects() {
    let mut log = common::load("running-example");
    log.traces[0]
        .attributes
        .insert("concept:name", AttributeValue::Int(3));
    let ocel = convert_log_to_ocel(&log, &LogToOcelOptions::default()).unwrap();
    let first = log.traces[0].events.len();
    assert!(
        ocel.relations
            .iter()
            .all(|r| r.event.parse::<usize>().unwrap() >= first)
    );
    let empty = LogToOcelOptions {
        obj_separator: String::new(),
        ..LogToOcelOptions::default()
    };
    assert!(
        convert_log_to_ocel(&log, &empty)
            .unwrap()
            .relations
            .is_empty()
    );
}
