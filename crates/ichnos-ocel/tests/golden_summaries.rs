//! pm4py's OCEL summaries (`pm4py/ocel.py`): object types, attribute names,
//! flattening, and the temporal, object and interaction summaries.
use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, Utc};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_golden::{cases, golden};
use ichnos_ocel::{EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject};
use serde_json::{Value, json};

fn text(v: &Value) -> Arc<str> {
    v.as_str().expect("string").into()
}

fn timestamp(v: &Value) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(v.as_str().expect("timestamp")).expect("timestamp")
}

fn value(v: &Value) -> AttributeValue {
    let x = &v[1];
    match v[0].as_str().expect("type") {
        "string" => AttributeValue::String(text(x)),
        "int" => AttributeValue::Int(x.as_i64().expect("int")),
        "float" => AttributeValue::Float(x.as_f64().expect("float")),
        "boolean" => AttributeValue::Bool(x.as_bool().expect("bool")),
        "date" => AttributeValue::Date(timestamp(x)),
        other => panic!("unknown type {other}"),
    }
}

fn attributes(v: &Value) -> Attributes {
    v.as_object()
        .expect("attributes")
        .iter()
        .map(|(k, v)| (k.as_str(), value(v)))
        .collect()
}

fn rows<'a>(v: &'a Value, key: &str) -> &'a Vec<Value> {
    v[key].as_array().expect(key)
}

fn build(v: &Value) -> Ocel {
    Ocel {
        events: rows(v, "events")
            .iter()
            .map(|e| OcelEvent {
                id: text(&e["id"]),
                activity: text(&e["activity"]),
                timestamp: timestamp(&e["timestamp"]),
                attributes: attributes(&e["attributes"]),
            })
            .collect(),
        objects: rows(v, "objects")
            .iter()
            .map(|o| OcelObject {
                id: text(&o["id"]),
                object_type: text(&o["type"]),
                attributes: attributes(&o["attributes"]),
            })
            .collect(),
        relations: rows(v, "relations")
            .iter()
            .map(|r| EventObject {
                event: text(&r["event"]),
                object: text(&r["object"]),
                qualifier: r["qualifier"].as_str().map(Into::into),
            })
            .collect(),
        o2o: rows(v, "o2o")
            .iter()
            .map(|r| ObjectObject {
                source: text(&r["source"]),
                target: text(&r["target"]),
                qualifier: r["qualifier"].as_str().map(Into::into),
            })
            .collect(),
        object_changes: rows(v, "object_changes")
            .iter()
            .map(|c| ObjectChange {
                object: text(&c["object"]),
                object_type: text(&c["type"]),
                timestamp: timestamp(&c["timestamp"]),
                field: text(&c["field"]),
                value: (!c["value"].is_null()).then(|| value(&c["value"])),
            })
            .collect(),
        ..Ocel::default()
    }
}

fn date(t: &DateTime<FixedOffset>) -> Value {
    json!(
        t.with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%S%.6f+00:00")
            .to_string()
    )
}

fn typed(v: &AttributeValue) -> Value {
    match v {
        AttributeValue::String(s) => json!(["string", &**s]),
        AttributeValue::Int(i) => json!(["int", i]),
        AttributeValue::Float(f) => json!(["float", f]),
        AttributeValue::Bool(b) => json!(["boolean", b]),
        AttributeValue::Date(d) => json!(["date", date(d)]),
        other => panic!("unexpected value {other:?}"),
    }
}

fn names(v: &[Arc<str>]) -> Value {
    json!(v.iter().map(|s| &**s).collect::<Vec<_>>())
}

fn ok(expected: &Value, key: &str) -> Value {
    expected[key]
        .get("ok")
        .unwrap_or_else(|| panic!("{key}: pm4py raised {}", expected[key]))
        .clone()
}

#[test]
fn ocel_summaries_match_pm4py() {
    let ids = cases("ocel_summaries");
    assert!(ids.len() >= 7, "{ids:?}");
    for id in ids {
        let g = golden("ocel_summaries", &id);
        let expected = g.expected_at("");
        let log = build(&expected["input"]);

        assert_eq!(
            names(&log.object_types()),
            ok(expected, "object_types"),
            "{id}"
        );

        // pm4py also lists columns that hold no value; ichnos keeps only
        // attributes with a value.
        let held: BTreeSet<&str> = ["events", "objects"]
            .iter()
            .flat_map(|t| rows(&expected["input"], t))
            .flat_map(|r| r["attributes"].as_object().unwrap().keys())
            .map(String::as_str)
            .collect();
        let pm4py_names = ok(expected, "attribute_names");
        let pm4py_held: Vec<&str> = pm4py_names
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .filter(|n| held.contains(n))
            .collect();
        assert_eq!(names(&log.attribute_names()), json!(pm4py_held), "{id}");

        for (object_type, result) in expected["flattening"].as_object().unwrap() {
            let actual: Vec<Value> = log
                .flatten(object_type)
                .events
                .iter()
                .map(|e| {
                    Value::Object(
                        e.attributes
                            .iter()
                            .map(|(k, v)| (k.to_string(), typed(v)))
                            .collect(),
                    )
                })
                .collect();
            assert_eq!(json!(actual), result["ok"], "{id} flatten {object_type}");
        }

        let temporal: Vec<Value> = log
            .temporal_summary()
            .iter()
            .map(|r| {
                json!({
                    "timestamp": date(&r.timestamp),
                    "activities": names(&r.activities),
                    "objects": names(&r.objects),
                })
            })
            .collect();
        assert_eq!(json!(temporal), ok(expected, "temporal_summary"), "{id}");

        let objects: Vec<Value> = log
            .objects_summary()
            .iter()
            .map(|r| {
                json!({
                    "object": &*r.object,
                    "activities": names(&r.activities),
                    "start": date(&r.start),
                    "end": date(&r.end),
                    "duration": r.duration,
                    "interacting": r.interacting.as_ref().map(|s| s.iter().map(|o| &**o).collect::<Vec<_>>()),
                })
            })
            .collect();
        match expected["objects_summary"].get("ok") {
            Some(rows) => assert_eq!(&json!(objects), rows, "{id}"),
            // pm4py raises AttributeError on a log without relations.
            None => assert!(objects.is_empty() && log.relations.is_empty(), "{id}"),
        }

        let interactions: Vec<Value> = log
            .objects_interactions_summary()
            .iter()
            .map(|r| {
                json!({
                    "event": &*r.event,
                    "activity": &*r.activity,
                    "object": &*r.object,
                    "type": &*r.object_type,
                    "object_2": &*r.object_2,
                    "type_2": &*r.object_type_2,
                })
            })
            .collect();
        assert_eq!(
            json!(interactions),
            ok(expected, "objects_interactions_summary"),
            "{id}"
        );
    }
}

#[test]
fn objects_outside_the_log_are_left_out() {
    let t = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
    let log = Ocel {
        events: vec![OcelEvent {
            id: "e1".into(),
            activity: "a".into(),
            timestamp: t,
            attributes: Attributes::default(),
        }],
        objects: vec![OcelObject {
            id: "o1".into(),
            object_type: "order".into(),
            attributes: Attributes::default(),
        }],
        relations: ["o1", "ghost"]
            .iter()
            .map(|o| EventObject {
                event: "e1".into(),
                object: (*o).into(),
                qualifier: None,
            })
            .chain([EventObject {
                event: "unknown".into(),
                object: "o1".into(),
                qualifier: None,
            }])
            .collect(),
        ..Ocel::default()
    };
    assert!(log.objects_interactions_summary().is_empty());
    let summary = log.objects_summary();
    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0].object.as_ref(), "ghost");
    assert_eq!(summary[0].interacting, None);
    assert_eq!(summary[1].activities.len(), 1);
    assert_eq!(
        summary[1].interacting,
        Some(BTreeSet::from(["ghost".into()]))
    );
}
