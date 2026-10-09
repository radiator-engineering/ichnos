//! The OCEL JSON and XML readers against pm4py's.
//!
//! The `ocel/model-*` goldens hold the tables pm4py's readers give for each
//! fixture. This test reads the same fixtures and compares every table.

use std::path::Path;

use chrono::DateTime;
use ichnos_core::AttributeValue;
use ichnos_golden::{cases, golden};
use ichnos_io::{
    OcelReadOptions, read_ocel, read_ocel_json, read_ocel_json_from_reader, read_ocel_xml,
    read_ocel2, read_ocel2_json, read_ocel2_xml,
};
use ichnos_ocel::Ocel;
use serde_json::Value;

/// Whether one of our values equals one pm4py value. Values must have the
/// same type, except for the documented behaviour changes:
/// - pm4py's XML readers keep `int`, `boolean`, `double` and `time` values as
///   strings, and read an OCEL 2.0 `float` of `null` as 0. ichnos types the
///   first four and keeps `null` as a string.
/// - pandas turns a JSON integer column with gaps into floats; ichnos keeps
///   the integer.
fn same(ours: &AttributeValue, theirs: &Value, xml: bool, what: &str) -> bool {
    let pair = theirs
        .as_array()
        .unwrap_or_else(|| panic!("{what}: [type, value]"));
    let x = &pair[1];
    match (pair[0].as_str().unwrap(), ours) {
        ("string", AttributeValue::String(s)) => **s == *x.as_str().unwrap(),
        ("int", AttributeValue::Int(i)) => x.as_i64() == Some(*i),
        ("float", AttributeValue::Float(f)) => x.as_f64() == Some(*f),
        ("boolean", AttributeValue::Bool(b)) => x.as_bool() == Some(*b),
        ("date", AttributeValue::Date(d)) => {
            DateTime::parse_from_rfc3339(x.as_str().unwrap()).unwrap() == *d
        }
        ("float", AttributeValue::Int(i)) if !xml => x.as_f64() == Some(*i as f64),
        ("string", AttributeValue::Int(i)) if xml => i.to_string() == x.as_str().unwrap(),
        ("string", AttributeValue::Bool(b)) if xml => {
            x.as_str().unwrap().eq_ignore_ascii_case(&b.to_string())
        }
        ("string", AttributeValue::Float(f)) if xml => {
            x.as_str().unwrap().parse::<f64>().ok() == Some(*f)
        }
        ("string", AttributeValue::Date(d)) if xml => {
            DateTime::parse_from_rfc3339(x.as_str().unwrap()).ok() == Some(*d)
        }
        ("int" | "float", AttributeValue::String(s)) if xml && &**s == "null" => {
            x.as_f64() == Some(0.0)
        }
        _ => false,
    }
}

fn check_attributes(ours: &ichnos_core::Attributes, theirs: &Value, xml: bool, what: &str) {
    let theirs = theirs.as_object().unwrap();
    let mut keys: Vec<&str> = ours.keys().map(|k| &**k).collect();
    keys.sort_unstable();
    let mut want: Vec<&str> = theirs.keys().map(String::as_str).collect();
    want.sort_unstable();
    assert_eq!(keys, want, "{what}: attribute names");
    for (k, v) in theirs {
        let o = ours.get(k).unwrap();
        assert!(same(o, v, xml, what), "{what}: {k} is {o:?}, pm4py {v}");
    }
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn ts(v: &Value) -> DateTime<chrono::FixedOffset> {
    DateTime::parse_from_rfc3339(s(v)).unwrap()
}

fn check(ocel: &Ocel, want: &Value, id: &str, relations_in_order: bool, xml: bool) {
    let events = want["events"].as_array().unwrap();
    assert_eq!(ocel.events.len(), events.len(), "{id}: events");
    for (e, w) in ocel.events.iter().zip(events) {
        let what = format!("{id}: event {}", e.id);
        assert_eq!(*e.id, *s(&w["id"]), "{what}");
        assert_eq!(*e.activity, *s(&w["activity"]), "{what}");
        assert_eq!(e.timestamp, ts(&w["timestamp"]), "{what}");
        check_attributes(&e.attributes, &w["attributes"], xml, &what);
    }
    let objects = want["objects"].as_array().unwrap();
    assert_eq!(ocel.objects.len(), objects.len(), "{id}: objects");
    for (o, w) in ocel.objects.iter().zip(objects) {
        let what = format!("{id}: object {}", o.id);
        assert_eq!(*o.id, *s(&w["id"]), "{what}");
        assert_eq!(*o.object_type, *s(&w["type"]), "{what}");
        check_attributes(&o.attributes, &w["attributes"], xml, &what);
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
            (Some(v), w) if !w.is_null() => assert!(same(v, w, xml, &what), "{what}: {v:?} vs {w}"),
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
            (read_ocel_xml(path, &OcelReadOptions::default()), true)
        } else if functions.contains(&"pm4py.read_ocel2_json") {
            // pm4py orders each event's relations by set iteration.
            (read_ocel2_json(path), false)
        } else if functions.contains(&"pm4py.read_ocel2_xml") {
            (read_ocel2_xml(path, &OcelReadOptions::default()), true)
        } else {
            continue;
        };
        let ocel = ocel.unwrap_or_else(|e| panic!("{id}: {e}"));
        let xml = functions.iter().any(|f| f.ends_with("_xml"));
        check(&ocel, &g.expected["ocel"], &id, in_order, xml);
        checked += 1;
    }
    assert_eq!(checked, 9, "fixtures checked");
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

/// A repeated id keeps its first position and its last value, as pm4py's
/// `dict` does (checked against pm4py 2.7.23.8).
#[test]
fn repeated_json_ids_follow_python_dict() {
    let doc = r#"{"ocel:global-log": {}, "ocel:global-event": {}, "ocel:global-object": {},
     "ocel:events": {
      "e1": {"ocel:activity": "a", "ocel:timestamp": "2020-01-01T00:00:00", "ocel:omap": ["o1"], "ocel:vmap": {}},
      "e2": {"ocel:activity": "a", "ocel:timestamp": "2020-01-02T00:00:00", "ocel:omap": ["o1"], "ocel:vmap": {}},
      "e1": {"ocel:activity": "b", "ocel:timestamp": "2020-01-03T00:00:00", "ocel:omap": ["o1"], "ocel:vmap": {}}},
     "ocel:objects": {
      "o1": {"ocel:type": "t", "ocel:ovmap": {}},
      "o1": {"ocel:type": "u", "ocel:ovmap": {}}}}"#;
    let ocel = read_ocel_json_from_reader(doc.as_bytes()).unwrap();
    let events: Vec<(&str, &str)> = ocel.events.iter().map(|e| (&*e.id, &*e.activity)).collect();
    assert_eq!(events, [("e2", "a"), ("e1", "b")]);
    let objects: Vec<(&str, &str)> = ocel
        .objects
        .iter()
        .map(|o| (&*o.id, &*o.object_type))
        .collect();
    assert_eq!(objects, [("o1", "u")]);
    let relations: Vec<&str> = ocel.relations.iter().map(|r| &*r.event).collect();
    assert_eq!(relations, ["e2", "e1"]);
}

/// `read_ocel` and `read_ocel2` choose a reader by the name's ending, as
/// pm4py's do.
#[test]
fn dispatch_follows_pm4py() {
    let g = golden("ocel", "model-typed-jsonocel");
    let json = g.fixture("log");
    let options = OcelReadOptions::default();
    let dir = std::env::temp_dir().join(format!("ichnos-ocel-dispatch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let copy = |name: &str| {
        let path = dir.join(name);
        std::fs::copy(&json, &path).unwrap();
        path
    };
    let want = read_ocel_json(&json).unwrap();
    // OCEL 1.0: only `jsonocel` and `xmlocel`.
    assert_eq!(read_ocel(copy("a.JSONOCEL"), &options).unwrap(), want);
    assert!(read_ocel(copy("a.json"), &options).is_err());
    assert!(read_ocel(copy("a.csv"), &options).is_err());
    // OCEL 2.0: `json` and `jsonocel`, also before `.gz`.
    let doc = golden("ocel", "model-typed20-jsonocel").fixture("log");
    let want2 = read_ocel2_json(&doc).unwrap();
    let path = dir.join("b.json");
    std::fs::copy(&doc, &path).unwrap();
    assert_eq!(read_ocel2(&path, &options).unwrap(), want2);
    assert!(read_ocel2(dir.join("b.ocel.csv"), &options).is_err());
    assert!(read_ocel2(dir.join("b.txt"), &options).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn xml_node_limit() {
    let path = golden("ocel", "model-typed-xmlocel").fixture("log");
    let tight = OcelReadOptions {
        max_nodes: 10,
        ..OcelReadOptions::default()
    };
    assert!(read_ocel_xml(&path, &tight).is_err());
    assert!(read_ocel_xml(&path, &OcelReadOptions::default()).is_ok());
}
