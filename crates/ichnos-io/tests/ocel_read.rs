//! The OCEL JSON, XML, CSV and SQLite readers against pm4py's.
//!
//! The `ocel/model-*` goldens hold the tables pm4py's readers give for each
//! fixture. This test reads the same fixtures and compares every table. The
//! `ocel/read-csv*` goldens hold the tables, or pm4py's error, for short CSV
//! texts, and the `ocel/read-sqlite*` goldens the same for databases built
//! from short SQL scripts. Each `ocel/write-*` golden also holds the
//! databases and bundles pm4py's SQLite and bundle writers give and what
//! pm4py's readers read from them; the test rebuilds each one and reads it.
//! The `ocel/read-bundle-*` goldens hold pm4py's tables, or its error, for
//! short bundles, as directories and as archives. Goldens
//! record times to the microsecond, so the test truncates
//! ichnos' nanoseconds.

use std::path::Path;

use chrono::{DateTime, SubsecRound};
use ichnos_core::AttributeValue;
use ichnos_golden::{cases, golden};
use ichnos_io::{
    OcelReadOptions, read_ocel, read_ocel_csv, read_ocel_csv_from_reader, read_ocel_json,
    read_ocel_json_from_reader, read_ocel_sqlite, read_ocel_xml, read_ocel2, read_ocel2_bundle,
    read_ocel2_csv, read_ocel2_csv_from_reader, read_ocel2_json, read_ocel2_sqlite, read_ocel2_xml,
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
            DateTime::parse_from_rfc3339(x.as_str().unwrap()).unwrap() == d.trunc_subsecs(6)
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
        assert_eq!(e.timestamp.trunc_subsecs(6), ts(&w["timestamp"]), "{what}");
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
        assert_eq!(c.timestamp.trunc_subsecs(6), ts(&w["timestamp"]), "{what}");
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
    for id in cases("ocel")
        .into_iter()
        .filter(|c| c.starts_with("model-"))
    {
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
        let (ocel, in_order) = if functions.contains(&"pm4py.read_ocel_csv") {
            // pm4py orders the objects by set iteration; the golden keeps
            // the order under `PYTHONHASHSEED=0`.
            let ocel =
                read_ocel_csv(path, None).map(|ocel| ordered_like(ocel, &g.expected["ocel"]));
            (ocel, true)
        } else if functions.contains(&"pm4py.read_ocel2_csv") {
            (read_ocel2_csv(path), true)
        } else if functions.contains(&"pm4py.read_ocel_sqlite") {
            (read_ocel_sqlite(path), true)
        } else if functions.contains(&"pm4py.read_ocel2_sqlite") {
            (read_ocel2_sqlite(path), true)
        } else if functions.contains(&"pm4py.read_ocel_json") {
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
    assert_eq!(checked, 15, "fixtures checked");
}

/// The log with its objects in the golden's order, where the golden has the
/// same objects.
fn ordered_like(mut ocel: Ocel, want: &Value) -> Ocel {
    let order: Vec<(&str, &str)> = want["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (s(&o["id"]), s(&o["type"])))
        .collect();
    ocel.objects.sort_by_key(|o| {
        order
            .iter()
            .position(|(id, t)| *id == &*o.id && *t == &*o.object_type)
            .unwrap_or(usize::MAX)
    });
    ocel
}

/// Where pm4py reads a CSV text and ichnos refuses it, as listed under
/// Behaviour changes: a day-first date that pandas guesses, and a `\N{...}`
/// escape in an object id.
const CSV_REFUSED: &[&str] = &["ts-day-first", "list-named-escape"];

/// The `read-csv-texts` and `read-csv2-texts` goldens: each text gives
/// pm4py's tables, with the objects sorted by id and type, or an error.
#[test]
fn csv_texts_match_pm4py() {
    type Read = fn(&[u8]) -> ichnos_io::Result<Ocel>;
    let readers: [(&str, Read); 2] = [
        ("read-csv-texts", |b| read_ocel_csv_from_reader(b, None)),
        ("read-csv2-texts", |b| read_ocel2_csv_from_reader(b)),
    ];
    let mut checked = 0;
    for (id, read) in readers {
        let g = golden("ocel", id);
        for (name, case) in g.expected["cases"].as_object().unwrap() {
            let what = format!("{id} {name}");
            let result = read(case["text"].as_str().unwrap().as_bytes());
            if case.get("error").is_some() {
                assert!(result.is_err(), "{what}: pm4py raises {}", case["error"]);
            } else if CSV_REFUSED.contains(&name.as_str()) && id == "read-csv-texts" {
                assert!(result.is_err(), "{what}: refused");
            } else {
                let mut ocel = result.unwrap_or_else(|e| panic!("{what}: {e}"));
                ocel.objects
                    .sort_by(|a, b| (&a.id, &a.object_type).cmp(&(&b.id, &b.object_type)));
                check(&ocel, &case["ocel"], &what, true, false);
            }
            checked += 1;
        }
    }
    assert!(checked > 100, "{checked} texts");
}

/// A new database path in the temporary directory.
fn temp_db(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ichnos-ocel-read-{}-{name}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

/// Where pm4py reads a database and ichnos refuses it, as listed under
/// Behaviour changes: timestamps stored as integers.
const SQLITE_REFUSED: &[&str] = &["ts-integer"];

/// The `read-sqlite-scripts` and `read-sqlite2-scripts` goldens: each SQL
/// script builds a database, which gives pm4py's tables or an error.
#[test]
fn sqlite_scripts_match_pm4py() {
    type Read = fn(&Path) -> ichnos_io::Result<Ocel>;
    let readers: [(&str, Read); 2] = [
        ("read-sqlite-scripts", |p| read_ocel_sqlite(p)),
        ("read-sqlite2-scripts", |p| read_ocel2_sqlite(p)),
    ];
    let mut checked = 0;
    for (id, read) in readers {
        let g = golden("ocel", id);
        for (name, case) in g.expected["cases"].as_object().unwrap() {
            let what = format!("{id} {name}");
            let path = temp_db(&format!("{id}-{name}"));
            rusqlite::Connection::open(&path)
                .unwrap()
                .execute_batch(case["script"].as_str().unwrap())
                .unwrap();
            let result = read(&path);
            if case.get("error").is_some() {
                assert!(result.is_err(), "{what}: pm4py raises {}", case["error"]);
            } else if SQLITE_REFUSED.contains(&name.as_str()) {
                assert!(result.is_err(), "{what}: refused");
            } else {
                let ocel = result.unwrap_or_else(|e| panic!("{what}: {e}"));
                check(&ocel, &case["ocel"], &what, true, false);
            }
            std::fs::remove_file(&path).unwrap();
            checked += 1;
        }
    }
    assert!(checked > 20, "{checked} scripts");
}

/// A golden cell: `null` or `[storage class, value]`.
fn sql_value(cell: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    let Some(pair) = cell.as_array() else {
        return Sql::Null;
    };
    match pair[0].as_str().unwrap() {
        "integer" => Sql::Integer(pair[1].as_i64().unwrap()),
        "real" => Sql::Real(pair[1].as_f64().unwrap()),
        "text" => Sql::Text(pair[1].as_str().unwrap().into()),
        other => panic!("cell type {other}"),
    }
}

/// Rebuilds a database from a golden's dump of it.
fn build_db(path: &Path, tables: &Value) {
    let conn = rusqlite::Connection::open(path).unwrap();
    for t in tables.as_array().unwrap() {
        conn.execute(t["sql"].as_str().unwrap(), []).unwrap();
        let name = t["name"].as_str().unwrap().replace('"', "\"\"");
        for row in t["rows"].as_array().unwrap() {
            let cells: Vec<_> = row.as_array().unwrap().iter().map(sql_value).collect();
            let marks = vec!["?"; cells.len()].join(",");
            conn.execute(
                &format!("INSERT INTO \"{name}\" VALUES ({marks})"),
                rusqlite::params_from_iter(cells),
            )
            .unwrap();
        }
    }
}

/// pm4py's SQLite writers' databases, rebuilt from the `write-*` goldens,
/// read as pm4py's SQLite readers read them.
#[test]
fn sqlite_files_of_pm4py_read_back() {
    type Read = fn(&Path) -> ichnos_io::Result<Ocel>;
    let readers: [(&str, Read); 2] = [
        ("sqlite", |p| read_ocel_sqlite(p)),
        ("sqlite2", |p| read_ocel2_sqlite(p)),
    ];
    let mut checked = 0;
    for id in cases("ocel")
        .into_iter()
        .filter(|c| c.starts_with("write-"))
    {
        let g = golden("ocel", &id);
        for (name, read) in readers {
            let want = &g.expected["writers"][name];
            if want.get("error").is_some() {
                continue;
            }
            let what = format!("{id} {name}");
            let path = temp_db(&format!("{id}-{name}"));
            build_db(&path, &want["tables"]);
            let result = read(&path);
            let reread = &want["reread"];
            if reread.get("error").is_some() {
                assert!(result.is_err(), "{what}: pm4py raises {}", reread["error"]);
            } else {
                let ocel = result.unwrap_or_else(|e| panic!("{what}: {e}"));
                check(&ocel, &reread["ocel"], &what, true, false);
            }
            std::fs::remove_file(&path).unwrap();
            checked += 1;
        }
    }
    assert_eq!(checked, 36);
}

/// Objects tables where ichnos returns an error and pm4py reads objects
/// without an id.
const CSV_OBJECTS_REFUSED: &[&str] = &["no-oid-column"];

/// The `read-csv-objects-texts` golden: one log read with each objects
/// table. pm4py does not check the table against the relations, and an
/// object from a table without `ocel:type` has no type, which ichnos gives
/// as the empty type.
#[test]
fn csv_objects_texts_match_pm4py() {
    let g = golden("ocel", "read-csv-objects-texts");
    let log = g.expected["log"].as_str().unwrap();
    let mut checked = 0;
    for (name, case) in g.expected["cases"].as_object().unwrap() {
        let what = format!("read-csv-objects-texts {name}");
        let mut objects = case["objects_text"].as_str().unwrap().as_bytes();
        let result = read_ocel_csv_from_reader(log.as_bytes(), Some(&mut objects));
        if CSV_OBJECTS_REFUSED.contains(&name.as_str()) {
            assert!(result.is_err(), "{what}: refused");
        } else if case.get("error").is_some() {
            assert!(result.is_err(), "{what}: pm4py raises {}", case["error"]);
        } else {
            let ocel = result.unwrap_or_else(|e| panic!("{what}: {e}"));
            let mut want = case["ocel"].clone();
            for o in want["objects"].as_array_mut().unwrap() {
                if o["type"].is_null() {
                    o["type"] = Value::from("");
                }
            }
            check(&ocel, &want, &what, true, false);
        }
        checked += 1;
    }
    assert_eq!(checked, 10);
}

/// pm4py's OCEL 1.0 CSV reader with an objects table takes the objects, in
/// file order, from it.
#[test]
fn reads_csv_objects_table() {
    let g = golden("ocel", "read-typed-csv-objects");
    let objects = g.fixture("objects");
    let ocel = read_ocel_csv(g.fixture("log"), Some(Path::new(&objects))).unwrap();
    check(
        &ocel,
        &g.expected["ocel"],
        "read-typed-csv-objects",
        true,
        false,
    );
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
    // OCEL 1.0: `jsonocel`, `xmlocel` and `csv`.
    assert_eq!(read_ocel(copy("a.JSONOCEL"), &options).unwrap(), want);
    assert!(read_ocel(copy("a.json"), &options).is_err());
    assert!(read_ocel(copy("a.csv"), &options).is_err(), "JSON as CSV");
    let csv = golden("ocel", "model-typed-csv").fixture("log");
    let path = dir.join("a.CSV");
    std::fs::copy(&csv, &path).unwrap();
    assert_eq!(
        read_ocel(&path, &options).unwrap(),
        read_ocel_csv(&csv, None).unwrap()
    );
    // OCEL 2.0: `json` and `jsonocel`, also before `.gz`.
    let doc = golden("ocel", "model-typed20-jsonocel").fixture("log");
    let want2 = read_ocel2_json(&doc).unwrap();
    let path = dir.join("b.json");
    std::fs::copy(&doc, &path).unwrap();
    assert_eq!(read_ocel2(&path, &options).unwrap(), want2);
    let csv2 = golden("ocel", "model-typed20-ocel-csv").fixture("log");
    let path = dir.join("b.OCEL.CSV");
    std::fs::copy(&csv2, &path).unwrap();
    assert_eq!(
        read_ocel2(&path, &options).unwrap(),
        read_ocel2_csv(&csv2).unwrap()
    );
    // SQLite: `.sqlite` for OCEL 1.0, any name ending in `sqlite` for 2.0.
    let db = golden("ocel", "model-ocel20-example-sqlite").fixture("log");
    let path = dir.join("c.SQLITE");
    std::fs::copy(&db, &path).unwrap();
    assert_eq!(
        read_ocel2(&path, &options).unwrap(),
        read_ocel2_sqlite(&db).unwrap()
    );
    let db = golden("ocel", "model-example-log-sqlite").fixture("log");
    let path = dir.join("d.sqlite");
    std::fs::copy(&db, &path).unwrap();
    assert_eq!(
        read_ocel(&path, &options).unwrap(),
        read_ocel_sqlite(&db).unwrap()
    );
    assert!(read_ocel(dir.join("d.db"), &options).is_err());
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

/// The bytes of a golden file: text, or `{"base64": ...}`.
fn file_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::String(text) => text.as_bytes().to_vec(),
        _ => base64(v["base64"].as_str().unwrap()),
    }
}

fn base64(text: &str) -> Vec<u8> {
    let digit = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => panic!("base64 {c}"),
    };
    let bits: Vec<u8> = text.bytes().filter(|&c| c != b'=').map(digit).collect();
    bits.chunks(4)
        .flat_map(|chunk| {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, &d)| n | (u32::from(d) << (18 - 6 * i)));
            let bytes = n.to_be_bytes();
            bytes[1..chunk.len()].to_vec()
        })
        .collect()
}

/// A new, empty directory in the temporary directory.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let path =
        std::env::temp_dir().join(format!("ichnos-ocel-bundle-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Writes each golden file under `root`.
fn write_files(root: &Path, files: &Value) {
    for (name, content) in files.as_object().unwrap() {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, file_bytes(content)).unwrap();
    }
}

/// Checks a bundle read against a golden result: pm4py's tables, or its
/// error, with the message when pm4py raised a `ValueError`.
fn check_bundle(result: ichnos_io::Result<Ocel>, case: &Value, what: &str) {
    match case.get("error") {
        Some(error) => {
            let e = result
                .err()
                .unwrap_or_else(|| panic!("{what}: pm4py raises {error}"));
            if let Some(message) = case["message"].as_str() {
                assert_eq!(e.to_string(), format!("invalid OCEL: {message}"), "{what}");
            }
        }
        None => {
            let ocel = result.unwrap_or_else(|e| panic!("{what}: {e}"));
            check(&ocel, &case["ocel"], what, true, false);
        }
    }
}

/// The `read-bundle-directories` and `read-bundle-parquet` goldens: each
/// bundle directory gives pm4py's tables or its error.
#[test]
fn bundle_directories_match_pm4py() {
    let mut checked = 0;
    for id in ["read-bundle-directories", "read-bundle-parquet"] {
        let g = golden("ocel", id);
        for (name, case) in g.expected["cases"].as_object().unwrap() {
            let what = format!("{id} {name}");
            let root = temp_dir(&format!("{id}-{name}"));
            write_files(&root, &case["files"]);
            check_bundle(read_ocel2_bundle(&root), case, &what);
            if case.get("error").is_none() {
                let options = OcelReadOptions::default();
                assert_eq!(
                    read_ocel2(&root, &options).unwrap(),
                    read_ocel2_bundle(&root).unwrap(),
                    "{what}: read_ocel2"
                );
            }
            std::fs::remove_dir_all(&root).unwrap();
            checked += 1;
        }
    }
    assert_eq!(checked, 96);
}

/// The `read-bundle-archives` golden: each archive, under its name, gives
/// pm4py's tables or its error.
#[test]
fn bundle_archives_match_pm4py() {
    let g = golden("ocel", "read-bundle-archives");
    let cases = g.expected["cases"].as_object().unwrap();
    for (name, case) in cases {
        let what = format!("read-bundle-archives {name}");
        let root = temp_dir(&format!("archive-{name}"));
        let path = root.join(case["name"].as_str().unwrap());
        std::fs::write(&path, file_bytes(&case["archive"])).unwrap();
        check_bundle(read_ocel2_bundle(&path), case, &what);
        if case.get("error").is_none() {
            let options = OcelReadOptions::default();
            assert_eq!(
                read_ocel2(&path, &options).unwrap(),
                read_ocel2_bundle(&path).unwrap(),
                "{what}: read_ocel2"
            );
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
    assert_eq!(cases.len(), 11);
}

/// pm4py's bundles, rebuilt from the `write-*` and `bundle-write-*`
/// goldens, read as pm4py reads them back.
#[test]
fn bundle_files_of_pm4py_read_back() {
    let mut checked = 0;
    for id in cases("ocel")
        .into_iter()
        .filter(|c| c.starts_with("write-") || c.starts_with("bundle-write-"))
    {
        let g = golden("ocel", &id);
        for name in ["bundle-csv", "bundle-parquet"] {
            let want = &g.expected["writers"][name];
            if want.get("error").is_some() {
                continue;
            }
            let what = format!("{id} {name}");
            let root = temp_dir(&format!("{id}-{name}"));
            write_files(&root, &want["files"]);
            if name == "bundle-parquet" {
                std::fs::write(root.join("ocel-meta.json"), want["meta"].as_str().unwrap())
                    .unwrap();
            }
            check_bundle(read_ocel2_bundle(&root), &want["reread"], &what);
            std::fs::remove_dir_all(&root).unwrap();
            checked += 1;
        }
    }
    assert_eq!(checked, 34);
}
