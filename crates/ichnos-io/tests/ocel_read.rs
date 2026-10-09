//! The OCEL JSON and XML readers against pm4py's.
//!
//! The `ocel/model-*` goldens hold the tables pm4py's readers give for each
//! fixture. This test reads the same fixtures and compares every table.

use std::path::Path;

use chrono::DateTime;
use ichnos_core::AttributeValue;
use ichnos_golden::{cases, golden};
use ichnos_io::{read_ocel_json, read_ocel_xml, read_ocel2_json, read_ocel2_xml};
use ichnos_ocel::Ocel;
use serde_json::Value;

/// Whether one of our values equals one pm4py value. pm4py's XML readers
/// keep integers and booleans as strings, and pandas turns an integer column
/// with gaps into floats, so those compare by value.
fn same(ours: &AttributeValue, theirs: &Value, what: &str) -> bool {
    let pair = theirs
        .as_array()
        .unwrap_or_else(|| panic!("{what}: [type, value]"));
    let x = &pair[1];
    match (pair[0].as_str().unwrap(), ours) {
        ("string", AttributeValue::String(s)) => **s == *x.as_str().unwrap(),
        ("string", AttributeValue::Int(i)) => i.to_string() == x.as_str().unwrap(),
        ("string", AttributeValue::Bool(b)) => {
            x.as_str().unwrap().eq_ignore_ascii_case(&b.to_string())
        }
        ("int" | "float", AttributeValue::Int(i)) => x.as_f64() == Some(*i as f64),
        ("int" | "float", AttributeValue::Float(f)) => x.as_f64() == Some(*f),
        ("boolean", AttributeValue::Bool(b)) => x.as_bool() == Some(*b),
        ("date", AttributeValue::Date(d)) => {
            DateTime::parse_from_rfc3339(x.as_str().unwrap()).unwrap() == *d
        }
        _ => false,
    }
}

fn check_attributes(ours: &ichnos_core::Attributes, theirs: &Value, what: &str) {
    let theirs = theirs.as_object().unwrap();
    let mut keys: Vec<&str> = ours.keys().map(|k| &**k).collect();
    keys.sort_unstable();
    let mut want: Vec<&str> = theirs.keys().map(String::as_str).collect();
    want.sort_unstable();
    assert_eq!(keys, want, "{what}: attribute names");
    for (k, v) in theirs {
        let o = ours.get(k).unwrap();
        assert!(same(o, v, what), "{what}: {k} is {o:?}, pm4py {v}");
    }
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn ts(v: &Value) -> DateTime<chrono::FixedOffset> {
    DateTime::parse_from_rfc3339(s(v)).unwrap()
}

fn check(ocel: &Ocel, want: &Value, id: &str, relations_in_order: bool) {
    let events = want["events"].as_array().unwrap();
    assert_eq!(ocel.events.len(), events.len(), "{id}: events");
    for (e, w) in ocel.events.iter().zip(events) {
        let what = format!("{id}: event {}", e.id);
        assert_eq!(*e.id, *s(&w["id"]), "{what}");
        assert_eq!(*e.activity, *s(&w["activity"]), "{what}");
        assert_eq!(e.timestamp, ts(&w["timestamp"]), "{what}");
        check_attributes(&e.attributes, &w["attributes"], &what);
    }
    let objects = want["objects"].as_array().unwrap();
    assert_eq!(ocel.objects.len(), objects.len(), "{id}: objects");
    for (o, w) in ocel.objects.iter().zip(objects) {
        let what = format!("{id}: object {}", o.id);
        assert_eq!(*o.id, *s(&w["id"]), "{what}");
        assert_eq!(*o.object_type, *s(&w["type"]), "{what}");
        check_attributes(&o.attributes, &w["attributes"], &what);
    }
    let mut ours: Vec<(String, String, Option<String>)> = ocel
        .relations
        .iter()
        .map(|r| {
            (
                r.event.to_string(),
                r.object.to_string(),
                r.qualifier.as_deref().map(String::from),
            )
        })
        .collect();
    let mut theirs: Vec<(String, String, Option<String>)> = want["relations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                s(&r["event"]).into(),
                s(&r["object"]).into(),
                r["qualifier"].as_str().map(String::from),
            )
        })
        .collect();
    if !relations_in_order {
        ours.sort();
        theirs.sort();
    }
    assert_eq!(ours, theirs, "{id}: relations");
    let pairs = |rows: &Value| -> Vec<(String, String, Option<String>)> {
        rows.as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    s(&r["source"]).into(),
                    s(&r["target"]).into(),
                    r["qualifier"].as_str().map(String::from),
                )
            })
            .collect()
    };
    let o2o: Vec<_> = ocel
        .o2o
        .iter()
        .map(|r| {
            (
                r.source.to_string(),
                r.target.to_string(),
                r.qualifier.as_deref().map(String::from),
            )
        })
        .collect();
    assert_eq!(o2o, pairs(&want["o2o"]), "{id}: o2o");
    let e2e: Vec<_> = ocel
        .e2e
        .iter()
        .map(|r| {
            (
                r.source.to_string(),
                r.target.to_string(),
                r.qualifier.as_deref().map(String::from),
            )
        })
        .collect();
    assert_eq!(e2e, pairs(&want["e2e"]), "{id}: e2e");
    let changes = want["object_changes"].as_array().unwrap();
    assert_eq!(
        ocel.object_changes.len(),
        changes.len(),
        "{id}: object changes"
    );
    for (c, w) in ocel.object_changes.iter().zip(changes) {
        let what = format!("{id}: change of {}", c.object);
        assert_eq!(*c.object, *s(&w["object"]), "{what}");
        assert_eq!(*c.object_type, *s(&w["type"]), "{what}");
        assert_eq!(c.timestamp, ts(&w["timestamp"]), "{what}");
        assert_eq!(*c.field, *s(&w["field"]), "{what}");
        match (&c.value, &w["value"]) {
            (None, Value::Null) => {}
            (Some(v), w) if !w.is_null() => assert!(same(v, w, &what), "{what}: {v:?} vs {w}"),
            (v, w) => panic!("{what}: {v:?} vs {w}"),
        }
    }
}

#[test]
fn readers_match_pm4py() {
    let mut checked = 0;
    for id in cases("ocel") {
        let g = golden("ocel", &id);
        let functions: Vec<&str> = g.meta["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if g.meta["fixtures"].get("log").is_none() {
            continue;
        }
        let path = g.fixture("log");
        let path: &Path = path.as_ref();
        let (ocel, in_order) = if functions.contains(&"pm4py.read_ocel_json") {
            (read_ocel_json(path), true)
        } else if functions.contains(&"pm4py.read_ocel_xml") {
            (read_ocel_xml(path), true)
        } else if functions.contains(&"pm4py.read_ocel2_json") {
            // pm4py orders each event's relations by set iteration.
            (read_ocel2_json(path), false)
        } else if functions.contains(&"pm4py.read_ocel2_xml") {
            (read_ocel2_xml(path), true)
        } else {
            continue;
        };
        let ocel = ocel.unwrap_or_else(|e| panic!("{id}: {e}"));
        check(&ocel, &g.expected["ocel"], &id, in_order);
        checked += 1;
    }
    assert_eq!(checked, 5, "fixtures checked");
}

#[test]
fn reads_globals_of_ocel_1_json() {
    let g = golden("ocel", "model-example-log-jsonocel");
    let ocel = read_ocel_json(g.fixture("log")).unwrap();
    let log = ocel.globals.get("ocel:global-log").expect("global log");
    let AttributeValue::Container(log) = log else {
        panic!("{log:?}")
    };
    assert_eq!(
        log.get("ocel:version"),
        Some(&AttributeValue::String("1.0".into()))
    );
}
