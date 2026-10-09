//! The OCEL JSON, XML and CSV writers against pm4py's.
//!
//! Each `ocel/write-*` golden holds the tables pm4py's writers start from
//! (`input`), the log's globals, and the file each writer gives. This test
//! builds an [`Ocel`] from the tables, writes it with each ichnos writer and
//! compares the files.
//!
//! pm4py writes attributes in the column order of its data frames, which
//! depends on the reader, and lists columns that hold no value in
//! `attribute-names`. ichnos writes attributes in first-appearance order and
//! lists only names with a value. So the comparison sorts attributes by name
//! and drops unused names from `attribute-names`; everything else, including
//! the order of events, objects and relations, must agree. Where the orders
//! agree (`BYTE_EXACT`), the files must agree byte for byte.
//!
//! A CSV file is compared row by row, each row as its non-empty cells keyed
//! by column name, with the JSON values in OCEL 2.0 references sorted by
//! name. When both headers are the same, the files must agree byte for
//! byte.
//!
//! A second test reads each fixture with the ichnos reader, writes it and
//! compares the files with pm4py's read-then-write output in the same way.

use std::fmt;
use std::io::Write;
use std::sync::Arc;

use chrono::DateTime;
use ichnos_core::{AttributeValue, Attributes};
use ichnos_golden::{cases, golden};
use ichnos_io::{
    OcelReadOptions, read_ocel, read_ocel_csv, read_ocel_json, read_ocel_xml, read_ocel2,
    read_ocel2_csv, read_ocel2_json, read_ocel2_xml, write_ocel, write_ocel_csv_to_writer,
    write_ocel_json, write_ocel_json_to_writer, write_ocel_xml, write_ocel_xml_to_writer,
    write_ocel2, write_ocel2_csv, write_ocel2_csv_to_writer, write_ocel2_json,
    write_ocel2_json_to_writer, write_ocel2_xml, write_ocel2_xml_to_writer,
};
use ichnos_ocel::{
    EventEvent, EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject,
};
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

/// The writers whose output must match pm4py's byte for byte, per case:
/// those where pm4py's column order is first-appearance order and no column
/// is empty.
const BYTE_EXACT: &[(&str, &[&str])] = &[
    ("write-empty", &["json", "xml", "json2", "xml2"]),
    ("write-synthetic", &["json", "xml", "json2", "xml2"]),
    ("write-synthetic20", &["json", "xml", "json2", "xml2"]),
    ("write-example-log-jsonocel", &["json2", "xml2"]),
    (
        "write-example-log-xmlocel",
        &["json", "xml", "json2", "xml2"],
    ),
    ("write-newocel-jsonocel", &["json2", "xml2"]),
    ("write-ocel20-example-jsonocel", &["json2", "xml2"]),
    ("write-ocel20-example-xmlocel", &["xml", "json2", "xml2"]),
];

/// Where pm4py's writer fails and ichnos writes the file. pm4py's OCEL 2.0
/// XML reader leaves the time of an object change as text when the times in
/// the file mix forms, and its OCEL 2.0 XML writer then fails calling
/// `isoformat` on that text. ichnos reads every time as a time.
const PM4PY_FAILS: &[(&str, &str)] = &[("write-typed20-xmlocel", "xml2")];

/// Where pm4py writes the text of such a time as it stands in the file,
/// and ichnos writes the time as pandas formats it. These cases compare
/// times as instants.
const TEXT_TIMES: &[&str] = &["write-typed20-xmlocel"];

/// Where pm4py's reader leaves the object attribute columns as pandas
/// `object` columns, so its OCEL 1.0 XML writer tags every object value as
/// `string`. ichnos tags a value by its type. These cases compare the
/// `float` and `string` tags as one.
const OBJECT_DTYPE: &[(&str, &str)] = &[("write-typed20-ocel-csv", "xml")];

/// The time pm4py gives an object's attribute values in OCEL 2.0.
const OBJECT_TIME: &str = "1970-01-01T00:00:00Z";

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

/// A global value as the OCEL 1.0 JSON reader keeps it.
fn global(v: &Value) -> Option<AttributeValue> {
    Some(match v {
        Value::Null => return None,
        Value::Bool(b) => AttributeValue::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => AttributeValue::Int(i),
            None => AttributeValue::Float(n.as_f64().expect("number")),
        },
        Value::String(s) => AttributeValue::String(s.as_str().into()),
        Value::Array(items) => AttributeValue::List(
            items
                .iter()
                .filter_map(|v| global(v).map(|v| (Arc::from(""), v)))
                .collect(),
        ),
        Value::Object(map) => AttributeValue::Container(
            map.iter()
                .filter_map(|(k, v)| global(v).map(|v| (k.as_str(), v)))
                .collect(),
        ),
    })
}

fn rows<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v[key].as_array().expect(key)
}

fn timestamp(v: &Value) -> chrono::DateTime<chrono::FixedOffset> {
    DateTime::parse_from_rfc3339(v.as_str().expect("timestamp")).expect("timestamp")
}

fn build(v: &Value, globals: &Value) -> Ocel {
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
        globals: globals
            .as_object()
            .expect("globals")
            .iter()
            .filter_map(|(k, v)| global(v).map(|v| (k.as_str(), v)))
            .collect(),
    }
}

/// A JSON value that keeps key order and tells integers from floats.
#[derive(Debug, Clone, PartialEq)]
enum J {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl<'de> Deserialize<'de> for J {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = J;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON")
            }
            fn visit_unit<E>(self) -> Result<J, E> {
                Ok(J::Null)
            }
            fn visit_bool<E>(self, b: bool) -> Result<J, E> {
                Ok(J::Bool(b))
            }
            fn visit_i64<E>(self, i: i64) -> Result<J, E> {
                Ok(J::Int(i.into()))
            }
            fn visit_u64<E>(self, i: u64) -> Result<J, E> {
                Ok(J::Int(i.into()))
            }
            fn visit_f64<E>(self, f: f64) -> Result<J, E> {
                Ok(J::Float(f))
            }
            fn visit_str<E>(self, s: &str) -> Result<J, E> {
                Ok(J::Str(s.to_owned()))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<J, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(J::Arr(items))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<J, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(J::Obj(entries))
            }
        }
        d.deserialize_any(V)
    }
}

impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> &str {
        match self {
            J::Str(s) => s,
            _ => "",
        }
    }

    /// Indented text, one value per line, for readable diffs.
    fn render(&self, level: usize, out: &mut String) {
        let pad = "  ".repeat(level);
        match self {
            J::Arr(items) => {
                for item in items {
                    out.push_str(&format!("{pad}-\n"));
                    item.render(level + 1, out);
                }
            }
            J::Obj(entries) => {
                for (k, v) in entries {
                    out.push_str(&format!("{pad}{k:?}:\n"));
                    v.render(level + 1, out);
                }
            }
            scalar => out.push_str(&format!("{pad}{scalar:?}\n")),
        }
    }
}

/// The attribute names used by any event or object of a JSON log.
fn used_json_names(log: &J) -> Vec<String> {
    let mut names = Vec::new();
    for (rows, map) in [("ocel:events", "ocel:vmap"), ("ocel:objects", "ocel:ovmap")] {
        if let Some(J::Obj(rows)) = log.get(rows) {
            for (_, row) in rows {
                if let Some(J::Obj(attrs)) = row.get(map) {
                    names.extend(attrs.iter().map(|(k, _)| k.clone()));
                }
            }
        }
    }
    names
}

/// Sorts what pm4py orders by data frame column: every map except the
/// event and object maps, and the attribute lists of the OCEL 2.0 layout.
fn canon_json(j: &mut J, key: &str, used: &[String]) {
    match j {
        J::Obj(entries) => {
            for (k, v) in entries.iter_mut() {
                canon_json(v, k, used);
            }
            if key != "ocel:events" && key != "ocel:objects" {
                entries.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }
        J::Arr(items) => {
            for item in items.iter_mut() {
                canon_json(item, "", used);
            }
            if key == "ocel:attribute-names" {
                items.retain(|n| used.iter().any(|u| u == n.str()));
            }
            if key == "attributes" {
                // An object's attributes start with its values at 1970;
                // the object changes follow in order.
                let start = items
                    .iter()
                    .take_while(|a| a.get("time").is_none_or(|t| t.str() == OBJECT_TIME))
                    .count();
                items[..start].sort_by(|a, b| {
                    a.get("name")
                        .unwrap()
                        .str()
                        .cmp(b.get("name").unwrap().str())
                });
            }
        }
        _ => {}
    }
}

/// Writes every time under `ocel:timestamp` or `time` in one form, so two
/// texts for the same instant compare equal.
fn same_times(j: &mut J, key: &str) {
    match j {
        J::Obj(entries) => entries.iter_mut().for_each(|(k, v)| same_times(v, k)),
        J::Arr(items) => items.iter_mut().for_each(|v| same_times(v, key)),
        J::Str(s) if key == "ocel:timestamp" || key == "time" => {
            if let Ok(d) = DateTime::parse_from_rfc3339(s) {
                *s = d.with_timezone(&chrono::Utc).to_rfc3339();
            }
        }
        _ => {}
    }
}

fn canon_json_text(text: &str, times: bool) -> String {
    let mut j: J = serde_json::from_str(text).expect("JSON");
    let used = used_json_names(&j);
    canon_json(&mut j, "", &used);
    if times {
        same_times(&mut j, "");
    }
    let mut out = String::new();
    j.render(0, &mut out);
    out
}

/// An XML element with ordered attributes. Text between child elements is
/// layout and is dropped.
#[derive(Debug, Clone)]
struct X {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<X>,
}

impl X {
    fn attr(&self, key: &str) -> &str {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map_or("", |(_, v)| v.as_str())
    }

    fn render(&self, level: usize, out: &mut String) {
        let pad = "  ".repeat(level);
        out.push_str(&format!("{pad}<{} {:?}", self.name, self.attrs));
        if self.children.is_empty() {
            out.push_str(&format!(" {:?}\n", self.text));
        } else {
            out.push('\n');
            for c in &self.children {
                c.render(level + 1, out);
            }
        }
    }
}

fn parse_xml(text: &str) -> X {
    let mut reader = Reader::from_str(text);
    reader.config_mut().expand_empty_elements = true;
    let mut stack: Vec<X> = Vec::new();
    loop {
        match reader.read_event().expect("XML") {
            Event::Start(tag) => {
                let attrs = tag
                    .attributes()
                    .map(|a| {
                        let a = a.expect("attribute");
                        (
                            a.key.as_ref().to_owned(),
                            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                .expect("value")
                                .into_owned(),
                        )
                    })
                    .collect();
                stack.push(X {
                    name: tag.name().as_ref().to_owned(),
                    attrs,
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            Event::End(_) => {
                let mut el = stack.pop().expect("open element");
                if !el.children.is_empty() {
                    el.text.clear();
                }
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => return el,
                }
            }
            Event::Text(t) => {
                if let Some(el) = stack.last_mut() {
                    el.text.push_str(t.as_ref());
                }
            }
            Event::GeneralRef(r) => {
                let encoded = format!("&{};", r.as_ref().to_owned());
                let decoded = quick_xml::escape::unescape(&encoded).expect("reference");
                stack
                    .last_mut()
                    .expect("open element")
                    .text
                    .push_str(&decoded);
            }
            Event::Eof => panic!("incomplete XML"),
            _ => {}
        }
    }
}

/// The attribute names used by any event or object of a classic XML log.
fn used_xml_names(log: &X, out: &mut Vec<String>) {
    if log.name == "list" && (log.attr("key") == "vmap" || log.attr("key") == "ovmap") {
        out.extend(log.children.iter().map(|c| c.attr("key").to_owned()));
    }
    for c in &log.children {
        used_xml_names(c, out);
    }
}

/// The XML counterpart of [`canon_json`].
fn canon_xml(x: &mut X, parent: &str, used: &[String]) {
    let name = x.name.clone();
    for c in x.children.iter_mut() {
        canon_xml(c, &name, used);
    }
    let by = |key: &'static str| move |a: &X, b: &X| a.attr(key).cmp(b.attr(key));
    match (name.as_str(), x.attr("key")) {
        ("event", _) if parent == "events" && x.attrs.is_empty() => {
            // Classic: the id, activity and timestamp come in column order.
            let fields = x.children.iter().take_while(|c| c.name != "list").count();
            x.children[..fields].sort_by(by("key"));
        }
        ("list", "vmap" | "ovmap") => x.children.sort_by(by("key")),
        ("list", "attribute-names") => x
            .children
            .retain(|c| used.iter().any(|u| u == c.attr("value"))),
        ("attributes", _) => {
            let start = x
                .children
                .iter()
                .take_while(|a| a.attrs.iter().all(|(k, v)| k != "time" || v == OBJECT_TIME))
                .count();
            x.children[..start].sort_by(by("name"));
        }
        _ => {}
    }
}

fn canon_xml_text(text: &str, _times: bool) -> String {
    let mut x = parse_xml(text);
    let mut used = Vec::new();
    used_xml_names(&x, &mut used);
    canon_xml(&mut x, "", &used);
    let mut out = String::new();
    x.render(0, &mut out);
    out
}

/// The records of a CSV text as Python's `csv` module writes them.
fn csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => record.push(std::mem::take(&mut field)),
            '\r' if !quoted && chars.peek() == Some(&'\n') => {}
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            c => field.push(c),
        }
    }
    records
}

/// A reference cell with the keys of each JSON object sorted.
fn sorted_json(cell: &str) -> String {
    let mut out = String::new();
    let mut rest = cell;
    loop {
        let mut escape = false;
        let start = rest.char_indices().find_map(|(i, c)| {
            let found = !escape && c == '{';
            escape = !escape && c == '\\';
            found.then_some(i)
        });
        let Some(start) = start else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..start]);
        let mut stream = serde_json::Deserializer::from_str(&rest[start..]).into_iter::<J>();
        let mut j = stream.next().expect("JSON").expect("JSON");
        if let J::Obj(entries) = &mut j {
            entries.sort_by(|a, b| a.0.cmp(&b.0));
        }
        out.push_str(&format!("{j:?}"));
        rest = &rest[start + stream.byte_offset()..];
    }
}

/// One line per row: its non-empty cells by column name.
fn canon_csv(text: &str) -> String {
    let mut records = csv_records(text).into_iter();
    let header = records.next().unwrap_or_default();
    records
        .map(|record| {
            let mut cells: Vec<(&String, String)> = header
                .iter()
                .zip(record)
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| {
                    let v = if k.starts_with("ot:") {
                        sorted_json(&v)
                    } else {
                        v
                    };
                    (k, v)
                })
                .collect();
            cells.sort();
            format!("{cells:?}\n")
        })
        .collect()
}

/// Compares a CSV text with pm4py's.
fn compare_csv(what: &str, ours: &[u8], theirs: &Value, failures: &mut Vec<String>) {
    let ours = std::str::from_utf8(ours).expect("UTF-8");
    let theirs = theirs.as_str().expect("text");
    assert_eq!(
        canon_csv(theirs).lines().count() + 1,
        csv_records(theirs).len(),
        "{what}: one canonical line per record"
    );
    if ours.lines().next() == theirs.lines().next()
        && let Some(diff) = first_difference(ours, theirs)
    {
        failures.push(format!("{what} (bytes): {diff}"));
    }
    if let Some(diff) = first_difference(&canon_csv(ours), &canon_csv(theirs)) {
        failures.push(format!("{what}: {diff}"));
    }
}

type Writer = fn(&Ocel, &mut Vec<u8>) -> ichnos_io::Result<()>;

const WRITERS: [(&str, Writer); 4] = [
    ("json", |o, out| write_ocel_json_to_writer(o, out)),
    ("xml", |o, out| write_ocel_xml_to_writer(o, out)),
    ("json2", |o, out| write_ocel2_json_to_writer(o, out)),
    ("xml2", |o, out| write_ocel2_xml_to_writer(o, out)),
];

fn write_cases() -> Vec<String> {
    cases("ocel")
        .into_iter()
        .filter(|c| c.starts_with("write-"))
        .collect()
}

#[test]
fn writers_match_pm4py() {
    let ids = write_cases();
    assert_eq!(ids.len(), 14, "cases: {ids:?}");
    let mut failures = Vec::new();
    for id in &ids {
        let g = golden("ocel", id);
        let ocel = build(&g.expected["input"], &g.expected["globals"]);
        compare_writers(id, &g.expected, &ocel, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Where the ichnos reader types values that pm4py's OCEL XML readers keep
/// as text (the "Typed XML values" Behaviour change), so the attribute
/// types in the written files differ.
const READER_TYPES_DIFFER: &[&str] = &["write-typed-xmlocel", "write-typed20-xmlocel"];

/// Where pm4py's OCEL 2.0 JSON reader orders each event's relations by
/// Python set iteration (the "Relation order" Behaviour change). The test
/// puts the relations in pm4py's order before writing.
const READER_RELATION_ORDER: &[&str] = &["write-ocel20-example-jsonocel", "write-typed20-jsonocel"];

/// pm4py reads each fixture with its own reader, then writes it. Reading
/// the fixture with the ichnos reader instead must give the same files, so
/// "read with ichnos, write with ichnos" agrees with pm4py end to end.
#[test]
fn writers_match_pm4py_from_files() {
    let options = OcelReadOptions::default();
    let mut checked = 0;
    let mut failures = Vec::new();
    for id in &write_cases() {
        let g = golden("ocel", id);
        if g.meta["fixtures"].get("log").is_none() || READER_TYPES_DIFFER.contains(&id.as_str()) {
            continue;
        }
        let path = g.fixture("log");
        let mut ocel = match g.meta["functions"][0].as_str().expect("reader") {
            "pm4py.read_ocel_json" => read_ocel_json(&path),
            "pm4py.read_ocel_xml" => read_ocel_xml(&path, &options),
            "pm4py.read_ocel2_json" => read_ocel2_json(&path),
            "pm4py.read_ocel2_xml" => read_ocel2_xml(&path, &options),
            "pm4py.read_ocel2_csv" => read_ocel2_csv(&path),
            other => panic!("{id}: unknown reader {other}"),
        }
        .unwrap_or_else(|e| panic!("{id}: {e}"));
        if READER_RELATION_ORDER.contains(&id.as_str()) {
            let order = build(&g.expected["input"], &g.expected["globals"]).relations;
            let position = |r: &EventObject| order.iter().position(|o| o == r);
            assert!(ocel.relations.iter().all(|r| position(r).is_some()), "{id}");
            ocel.relations.sort_by_key(position);
        }
        compare_writers(id, &g.expected, &ocel, &mut failures);
        checked += 1;
    }
    assert_eq!(checked, 8);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Writes `ocel` with each writer and compares the files with the golden's.
fn compare_writers(id: &str, expected: &Value, ocel: &Ocel, failures: &mut Vec<String>) {
    for (name, write) in WRITERS {
        let want = &expected["writers"][name];
        let mut out = Vec::new();
        let result = write(ocel, &mut out);
        if let Some(error) = want.get("error") {
            if PM4PY_FAILS.contains(&(id, name)) {
                result.unwrap_or_else(|e| panic!("{id} {name}: {e}"));
                parse_xml(std::str::from_utf8(&out).expect("UTF-8"));
            } else {
                assert!(result.is_err(), "{id} {name}: pm4py raises {error}");
            }
            continue;
        }
        result.unwrap_or_else(|e| panic!("{id} {name}: {e}"));
        let ours = String::from_utf8(out).expect("UTF-8");
        let theirs = want["text"].as_str().expect("text");
        if BYTE_EXACT
            .iter()
            .any(|(case, writers)| *case == id && writers.contains(&name))
            && let Some(diff) = first_difference(&ours, theirs)
        {
            failures.push(format!("{id} {name} (bytes): {diff}"));
        }
        let canon = if name.starts_with("json") {
            canon_json_text
        } else {
            canon_xml_text
        };
        let times = TEXT_TIMES.contains(&id);
        let (mut ours, mut theirs) = (canon(&ours, times), canon(theirs, times));
        if OBJECT_DTYPE.contains(&(id, name)) {
            ours = ours.replace("<float [", "<string [");
            theirs = theirs.replace("<float [", "<string [");
        }
        if let Some(diff) = first_difference(&ours, &theirs) {
            failures.push(format!("{id} {name}: {diff}"));
        }
    }
    let want = &expected["writers"]["csv"];
    let (mut table, mut objects) = (Vec::new(), Vec::new());
    write_ocel_csv_to_writer(ocel, &mut table, Some(&mut objects as &mut dyn Write))
        .unwrap_or_else(|e| panic!("{id} csv: {e}"));
    compare_csv(&format!("{id} csv"), &table, &want["text"], failures);
    compare_csv(
        &format!("{id} csv objects"),
        &objects,
        &want["objects"],
        failures,
    );
    let want = &expected["writers"]["csv2"];
    let mut out = Vec::new();
    let result = write_ocel2_csv_to_writer(ocel, &mut out);
    if let Some(error) = want.get("error") {
        assert!(result.is_err(), "{id} csv2: pm4py raises {error}");
    } else {
        result.unwrap_or_else(|e| panic!("{id} csv2: {e}"));
        compare_csv(&format!("{id} csv2"), &out, &want["text"], failures);
    }
}

/// The first line where two texts differ, with the lines before it.
fn first_difference(ours: &str, theirs: &str) -> Option<String> {
    let a: Vec<&str> = ours.lines().collect();
    let b: Vec<&str> = theirs.lines().collect();
    let i = (0..a.len().max(b.len())).find(|&i| a.get(i) != b.get(i))?;
    let from = i.saturating_sub(6);
    Some(format!(
        "line {i}\n  ours:   {:?}\n  pm4py:  {:?}",
        &a[from.min(a.len())..(i + 2).min(a.len())],
        &b[from.min(b.len())..(i + 2).min(b.len())]
    ))
}

/// pm4py's `write_ocel_json` picks the OCEL 2.0 layout only for a log with
/// OCEL 2.0 features, before its consistency step fills in empty
/// qualifiers.
#[test]
fn json_layout_follows_ocel20_features() {
    let g = golden("ocel", "write-synthetic");
    let ocel = build(&g.expected["input"], &g.expected["globals"]);
    assert!(!ocel.is_ocel20());
    let mut out = Vec::new();
    write_ocel_json_to_writer(&ocel, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(!text.contains("ocel:typedOmap"), "{text}");
    let g = golden("ocel", "write-synthetic20");
    let ocel = build(&g.expected["input"], &g.expected["globals"]);
    let mut out = Vec::new();
    write_ocel_json_to_writer(&ocel, &mut out).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("ocel:typedOmap"));
}

/// XML cannot hold most control characters; lxml refuses them, and so does
/// ichnos. JSON escapes them.
#[test]
fn xml_refuses_control_characters() {
    let g = golden("ocel", "write-synthetic20");
    let mut ocel = build(&g.expected["input"], &g.expected["globals"]);
    ocel.events[0]
        .attributes
        .insert("c_text", AttributeValue::String("a\u{1}b".into()));
    for (name, write) in WRITERS {
        let result = write(&ocel, &mut Vec::new());
        assert_eq!(result.is_err(), name.starts_with("xml"), "{name}");
    }
}

/// Each written file reads back, through gzip too, and the dispatchers pick
/// the writer by the name's ending.
#[test]
fn files_read_back() {
    let dir = std::env::temp_dir().join(format!("ichnos-ocel-write-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let options = OcelReadOptions::default();
    let g = golden("ocel", "model-ocel20-example-jsonocel");
    let ocel = read_ocel2_json(g.fixture("log")).unwrap();
    for gz in ["", ".gz"] {
        let path = dir.join(format!("a.jsonocel{gz}"));
        write_ocel2_json(&ocel, &path).unwrap();
        assert_eq!(read_ocel2_json(&path).unwrap(), ocel, "json2{gz}");
        let path = dir.join(format!("a.xmlocel{gz}"));
        write_ocel2_xml(&ocel, &path).unwrap();
        let back = read_ocel2_xml(&path, &options).unwrap();
        assert_eq!(back.events.len(), ocel.events.len(), "xml2{gz}");
        assert_eq!(back.relations, ocel.relations, "xml2{gz}");
        assert_eq!(back.o2o, ocel.o2o, "xml2{gz}");
    }
    let path = dir.join("b.jsonocel");
    write_ocel_json(&ocel, &path).unwrap();
    assert_eq!(read_ocel_json(&path).unwrap().relations, ocel.relations);
    let path = dir.join("b.xmlocel");
    write_ocel_xml(&ocel, &path).unwrap();
    assert_eq!(
        read_ocel_xml(&path, &options).unwrap().events.len(),
        ocel.events.len()
    );

    // `write_ocel`: `jsonocel`, `xmlocel` and `csv`.
    write_ocel(&ocel, dir.join("c.JSONOCEL")).unwrap();
    assert!(write_ocel(&ocel, dir.join("c.json")).is_err());
    let path = dir.join("c.csv");
    write_ocel(&ocel, &path).unwrap();
    let back = read_ocel_csv(&path, None).unwrap();
    let pairs = |o: &Ocel| {
        let mut pairs: Vec<(String, String)> = o
            .relations
            .iter()
            .map(|r| (r.event.to_string(), r.object.to_string()))
            .collect();
        pairs.sort();
        pairs
    };
    assert_eq!(back.events.len(), ocel.events.len(), "csv");
    assert_eq!(pairs(&back), pairs(&ocel), "csv");
    assert_eq!(read_ocel(&path, &options).unwrap(), back);
    // `write_ocel2`: `json`, `jsonocel`, `xml` and `xmlocel`, also before `.gz`.
    let path = dir.join("d.json.gz");
    write_ocel2(&ocel, &path).unwrap();
    assert_eq!(read_ocel2(&path, &options).unwrap(), ocel);
    write_ocel2(&ocel, dir.join("d.xml")).unwrap();
    let path = dir.join("d.ocel.csv");
    write_ocel2(&ocel, &path).unwrap();
    let back = read_ocel2(&path, &options).unwrap();
    assert_eq!(back.events.len(), ocel.events.len(), "csv2");
    assert_eq!(pairs(&back), pairs(&ocel), "csv2");
    // The writer orders the object-to-object rows by source.
    let mut o2o = ocel.o2o.clone();
    o2o.sort_by(|a, b| (&a.source, &a.target).cmp(&(&b.source, &b.target)));
    assert_eq!(back.o2o, o2o, "csv2");
    // pm4py's OCEL 2.0 CSV writer requires the `.ocel.csv` ending.
    assert!(write_ocel2_csv(&ocel, dir.join("d.csv")).is_err());
    assert!(write_ocel2(&ocel, dir.join("d.txt")).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}
