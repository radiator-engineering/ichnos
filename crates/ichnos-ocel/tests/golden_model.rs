//! pm4py's `OCEL` object on the OCEL fixtures: `is_ocel20`, `get_summary`
//! and `get_extended_table`.
//!
//! Each golden holds the tables pm4py read from one fixture. The test builds
//! an [`Ocel`] from them and compares what it computes with pm4py.

use std::sync::Arc;

use chrono::DateTime;
use ichnos_core::{AttributeValue, Attributes};
use ichnos_golden::{cases, golden};
use ichnos_ocel::{
    EventEvent, EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject,
};
use serde_json::Value;

fn text(v: &Value) -> Arc<str> {
    v.as_str().expect("string").into()
}

fn qualifier(v: &Value) -> Option<Arc<str>> {
    v.as_str().map(Into::into)
}

fn value(v: &Value) -> AttributeValue {
    let pair = v.as_array().expect("[type, value]");
    let x = &pair[1];
    match pair[0].as_str().expect("type") {
        "string" => AttributeValue::String(text(x)),
        "int" => AttributeValue::Int(x.as_i64().expect("int")),
        "float" => AttributeValue::Float(x.as_f64().expect("float")),
        "boolean" => AttributeValue::Bool(x.as_bool().expect("bool")),
        "date" => AttributeValue::Date(
            DateTime::parse_from_rfc3339(x.as_str().expect("date")).expect("date"),
        ),
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

fn rows(v: &Value, key: &str) -> Vec<Value> {
    v[key].as_array().expect(key).clone()
}

fn timestamp(v: &Value) -> chrono::DateTime<chrono::FixedOffset> {
    DateTime::parse_from_rfc3339(v.as_str().expect("timestamp")).expect("timestamp")
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
                qualifier: qualifier(&r["qualifier"]),
            })
            .collect(),
        o2o: rows(v, "o2o")
            .iter()
            .map(|r| ObjectObject {
                source: text(&r["source"]),
                target: text(&r["target"]),
                qualifier: qualifier(&r["qualifier"]),
            })
            .collect(),
        e2e: rows(v, "e2e")
            .iter()
            .map(|r| EventEvent {
                source: text(&r["source"]),
                target: text(&r["target"]),
                qualifier: qualifier(&r["qualifier"]),
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
        globals: Attributes::default(),
        naive_times: false,
    }
}

fn counts(v: &Value) -> Vec<(String, u64)> {
    v.as_object()
        .expect("counts")
        .iter()
        .map(|(k, n)| (k.clone(), n.as_u64().expect("count")))
        .collect()
}

fn ours(c: &[(Arc<str>, usize)]) -> Vec<(String, u64)> {
    c.iter().map(|(k, n)| (k.to_string(), *n as u64)).collect()
}

fn sorted(mut c: Vec<(String, u64)>) -> Vec<(String, u64)> {
    c.sort();
    c
}

#[test]
fn ocel_goldens() {
    let ids: Vec<String> = cases("ocel")
        .into_iter()
        .filter(|c| c.starts_with("model-"))
        .collect();
    assert_eq!(ids.len(), 16, "cases: {ids:?}");
    for id in &ids {
        let g = golden("ocel", id);
        let e = &g.expected;
        let ocel = build(&e["ocel"]);

        // The relations table repeats the activity, timestamp and type.
        let events = ocel.event_index();
        let objects = ocel.object_index();
        for r in rows(&e["ocel"], "relations") {
            let ev = &ocel.events[events[r["event"].as_str().unwrap()]];
            let ob = &ocel.objects[objects[r["object"].as_str().unwrap()]];
            assert_eq!(*ev.activity, *r["activity"].as_str().unwrap(), "{id}");
            assert_eq!(ev.timestamp, timestamp(&r["timestamp"]), "{id}");
            assert_eq!(*ob.object_type, *r["type"].as_str().unwrap(), "{id}");
        }

        assert_eq!(
            ocel.is_ocel20(),
            e["is_ocel20"].as_bool().unwrap(),
            "{id}: is_ocel20"
        );

        let s = ocel.summary();
        let x = &e["summary"];
        assert_eq!(s.events as u64, x["events"].as_u64().unwrap(), "{id}");
        assert_eq!(s.objects as u64, x["objects"].as_u64().unwrap(), "{id}");
        assert_eq!(
            s.activities as u64,
            x["activities"].as_u64().unwrap(),
            "{id}"
        );
        assert_eq!(
            s.object_types as u64,
            x["object_types"].as_u64().unwrap(),
            "{id}"
        );
        assert_eq!(s.relations as u64, x["relations"].as_u64().unwrap(), "{id}");
        // The golden's maps lose pandas' order, so the counts compare as
        // sets and the summary text checks the order.
        assert_eq!(
            sorted(ours(&s.activity_counts)),
            sorted(counts(&x["activity_counts"])),
            "{id}"
        );
        assert_eq!(
            sorted(ours(&s.object_type_counts)),
            sorted(counts(&x["object_type_counts"])),
            "{id}"
        );
        assert_eq!(
            sorted(ours(&s.activities_per_object_type)),
            sorted(counts(&x["activities_per_object_type"])),
            "{id}"
        );
        assert_eq!(
            s.to_string(),
            x["text"].as_str().unwrap(),
            "{id}: summary text"
        );

        let t = ocel.extended_table();
        let x = &e["extended_table"];
        let types: Vec<&str> = t.object_types.iter().map(|t| &**t).collect();
        let expected: Vec<&str> = x["object_types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        assert_eq!(types, expected, "{id}: object types");
        let expected_rows = x["rows"].as_array().unwrap();
        assert_eq!(t.rows.len(), expected_rows.len(), "{id}: rows");
        for (row, want) in t.rows.iter().zip(expected_rows) {
            assert_eq!(
                *ocel.events[row.event].id,
                *want["event"].as_str().unwrap(),
                "{id}"
            );
            for (ty, objs) in t.object_types.iter().zip(&row.objects) {
                let want: Vec<&str> = want["objects"]
                    .get(&**ty)
                    .map(|v| {
                        v.as_array()
                            .unwrap()
                            .iter()
                            .map(|o| o.as_str().unwrap())
                            .collect()
                    })
                    .unwrap_or_default();
                let got: Vec<&str> = objs.iter().map(|o| &**o).collect();
                assert_eq!(
                    got, want,
                    "{id}: event {} type {ty}",
                    ocel.events[row.event].id
                );
            }
        }
    }
}
