//! OCEL JSON: the OCEL 1.0 layout (pm4py's `jsonocel` `classic` variant)
//! and the OCEL 2.0 standard layout (`ocel20_standard`).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::constants::{
    CHANGED_FIELD, EVENT_TIMESTAMP, GLOBAL_EVENT, GLOBAL_LOG, GLOBAL_OBJECT, OBJECT_ID, QUALIFIER,
};
use ichnos_ocel::{EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject};
use serde::de::{Deserialize, DeserializeOwned, Deserializer, MapAccess, Visitor};
use serde_json::Value;

use super::{finish, open, time};
use crate::error::{Error, Result};

/// Reads an OCEL 1.0 JSON file (pm4py's `read_ocel_json`). A name ending in
/// `.gz` is decompressed.
pub fn read_ocel_json(path: impl AsRef<Path>) -> Result<Ocel> {
    read_ocel_json_from_reader(open(path.as_ref())?)
}

/// Reads OCEL 1.0 JSON from a reader.
///
/// Attribute values keep their JSON types. A `null` value is left out.
/// Relations to objects that are not in `ocel:objects` are left out. An
/// event's `ocel:typedOmap`, when present, gives the qualifiers.
pub fn read_ocel_json_from_reader(input: impl Read) -> Result<Ocel> {
    let doc: ClassicDoc = serde_json::from_reader(input)?;
    classic(doc)
}

/// Reads an OCEL 2.0 JSON file (pm4py's `read_ocel2_json`). A name ending
/// in `.gz` is decompressed.
pub fn read_ocel2_json(path: impl AsRef<Path>) -> Result<Ocel> {
    read_ocel2_json_from_reader(open(path.as_ref())?)
}

/// Reads OCEL 2.0 JSON from a reader.
///
/// Attribute values are converted to the types that `eventTypes` and
/// `objectTypes` declare. The first value of an object attribute is the
/// object's attribute, and each later value is an [`ObjectChange`] at its
/// `time`. An event relates to each object once, with the last qualifier
/// given for it, in the order the relationships first name the objects.
pub fn read_ocel2_json_from_reader(input: impl Read) -> Result<Ocel> {
    let doc: StandardDoc = serde_json::from_reader(input)?;
    standard(doc)
}

/// A JSON object read as its entries in file order. A repeated key keeps
/// its first position and its last value, as in a Python `dict`.
struct Ordered<T>(Vec<(String, T)>);

impl<T> Default for Ordered<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Ordered<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: DeserializeOwned> Visitor<'de> for V<T> {
            type Value = Ordered<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut entries: Vec<(String, T)> = Vec::new();
                let mut index: HashMap<String, usize> = HashMap::new();
                while let Some((k, v)) = map.next_entry::<String, T>()? {
                    match index.get(&k) {
                        Some(&i) => entries[i].1 = v,
                        None => {
                            index.insert(k.clone(), entries.len());
                            entries.push((k, v));
                        }
                    }
                }
                Ok(Ordered(entries))
            }
        }
        deserializer.deserialize_map(V(PhantomData))
    }
}

#[derive(serde::Deserialize)]
struct ClassicDoc {
    #[serde(rename = "ocel:global-log", default)]
    global_log: Option<Value>,
    #[serde(rename = "ocel:global-event", default)]
    global_event: Option<Value>,
    #[serde(rename = "ocel:global-object", default)]
    global_object: Option<Value>,
    #[serde(rename = "ocel:events", default)]
    events: Ordered<ClassicEvent>,
    #[serde(rename = "ocel:objects", default)]
    objects: Ordered<ClassicObject>,
    #[serde(rename = "ocel:objectChanges", default)]
    object_changes: Vec<Ordered<Value>>,
}

#[derive(serde::Deserialize)]
struct ClassicEvent {
    #[serde(rename = "ocel:activity")]
    activity: Value,
    #[serde(rename = "ocel:timestamp")]
    timestamp: String,
    #[serde(rename = "ocel:omap", default)]
    omap: Vec<Value>,
    #[serde(rename = "ocel:vmap", default)]
    vmap: Ordered<Value>,
    #[serde(rename = "ocel:typedOmap", default)]
    typed_omap: Vec<Ordered<Value>>,
}

#[derive(serde::Deserialize)]
struct ClassicObject {
    #[serde(rename = "ocel:type")]
    object_type: Value,
    #[serde(rename = "ocel:ovmap", default)]
    ovmap: Ordered<Value>,
    #[serde(rename = "ocel:o2o", default)]
    o2o: Vec<Ordered<Value>>,
}

/// The text of a JSON scalar, as Python's `str` gives it for ids.
fn text(v: &Value) -> Arc<str> {
    match v {
        Value::String(s) => s.as_str().into(),
        Value::Null => "".into(),
        other => other.to_string().into(),
    }
}

fn get<'a>(entries: &'a Ordered<Value>, key: &str) -> Option<&'a Value> {
    entries
        .0
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

/// A JSON value as an attribute value. `null` has none.
pub(super) fn value(v: &Value) -> Option<AttributeValue> {
    Some(match v {
        Value::Null => return None,
        Value::Bool(b) => AttributeValue::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => AttributeValue::Int(i),
            None => AttributeValue::Float(n.as_f64()?),
        },
        Value::String(s) => AttributeValue::String(s.as_str().into()),
        Value::Array(items) => AttributeValue::List(
            items
                .iter()
                .filter_map(|x| Some((Arc::from(""), value(x)?)))
                .collect(),
        ),
        Value::Object(map) => AttributeValue::Container(
            map.iter()
                .filter_map(|(k, x)| Some((k.as_str(), value(x)?)))
                .collect(),
        ),
    })
}

fn attributes(entries: &Ordered<Value>) -> Attributes {
    entries
        .0
        .iter()
        .filter_map(|(k, v)| Some((k.as_str(), value(v)?)))
        .collect()
}

fn timestamp(text: &str) -> Result<chrono::DateTime<chrono::FixedOffset>> {
    time::parse(text).ok_or_else(|| Error::Ocel(format!("invalid timestamp {text:?}")))
}

fn classic(doc: ClassicDoc) -> Result<Ocel> {
    let mut ocel = Ocel::new();
    let mut types: HashMap<&str, Arc<str>> = HashMap::new();
    for (id, o) in &doc.objects.0 {
        let object_type = text(&o.object_type);
        types.insert(id, object_type.clone());
        for rel in &o.o2o {
            ocel.o2o.push(ObjectObject {
                source: id.as_str().into(),
                target: get(rel, OBJECT_ID).map(text).unwrap_or_default(),
                qualifier: get(rel, QUALIFIER).and_then(qualifier),
            });
        }
        ocel.objects.push(OcelObject {
            id: id.as_str().into(),
            object_type,
            attributes: attributes(&o.ovmap),
        });
    }
    for (id, e) in &doc.events.0 {
        let event: Arc<str> = id.as_str().into();
        // One relation per known object, in first-mention order.
        let mut related: Vec<EventObject> = Vec::new();
        let mut index: HashMap<Arc<str>, usize> = HashMap::new();
        for o in &e.omap {
            let o = text(o);
            if types.contains_key(&*o) && !index.contains_key(&o) {
                index.insert(o.clone(), related.len());
                related.push(EventObject {
                    event: event.clone(),
                    object: o,
                    qualifier: None,
                });
            }
        }
        for typed in &e.typed_omap {
            if let Some(o) = get(typed, OBJECT_ID)
                && let Some(&i) = index.get(&text(o))
            {
                related[i].qualifier = get(typed, QUALIFIER).and_then(qualifier);
            }
        }
        ocel.relations.extend(related);
        ocel.events.push(OcelEvent {
            id: event,
            activity: text(&e.activity),
            timestamp: timestamp(&e.timestamp)?,
            attributes: attributes(&e.vmap),
        });
    }
    for change in &doc.object_changes {
        let object = get(change, OBJECT_ID).map(text).unwrap_or_default();
        let field = get(change, CHANGED_FIELD).map(text).unwrap_or_default();
        let when = get(change, EVENT_TIMESTAMP).map(text).unwrap_or_default();
        ocel.object_changes.push(ObjectChange {
            object_type: types.get(&*object).cloned().unwrap_or_default(),
            object,
            timestamp: timestamp(&when)?,
            value: get(change, &field).and_then(value),
            field,
        });
    }
    let mut globals = Attributes::default();
    for (key, v) in [
        (GLOBAL_LOG, &doc.global_log),
        (GLOBAL_EVENT, &doc.global_event),
        (GLOBAL_OBJECT, &doc.global_object),
    ] {
        if let Some(v) = v.as_ref().and_then(value) {
            globals.insert(key, v);
        }
    }
    ocel.globals = globals;
    Ok(finish(ocel))
}

fn qualifier(v: &Value) -> Option<Arc<str>> {
    (!v.is_null()).then(|| text(v))
}

#[derive(serde::Deserialize)]
struct StandardDoc {
    #[serde(rename = "eventTypes", default)]
    event_types: Vec<StandardType>,
    #[serde(rename = "objectTypes", default)]
    object_types: Vec<StandardType>,
    #[serde(default)]
    events: Vec<StandardEvent>,
    #[serde(default)]
    objects: Vec<StandardObject>,
}

#[derive(serde::Deserialize)]
struct StandardType {
    name: String,
    #[serde(default)]
    attributes: Vec<StandardAttributeType>,
}

#[derive(serde::Deserialize)]
struct StandardAttributeType {
    name: String,
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

#[derive(serde::Deserialize)]
struct StandardEvent {
    id: Value,
    #[serde(rename = "type")]
    kind: Value,
    time: String,
    #[serde(default)]
    attributes: Option<Vec<StandardAttribute>>,
    #[serde(default)]
    relationships: Option<Vec<StandardRelationship>>,
}

#[derive(serde::Deserialize)]
struct StandardObject {
    id: Value,
    #[serde(rename = "type")]
    kind: Value,
    #[serde(default)]
    attributes: Option<Vec<StandardAttribute>>,
    #[serde(default)]
    relationships: Option<Vec<StandardRelationship>>,
}

#[derive(serde::Deserialize)]
struct StandardAttribute {
    name: String,
    #[serde(default)]
    value: Value,
    #[serde(default)]
    time: Option<Value>,
}

#[derive(serde::Deserialize)]
struct StandardRelationship {
    #[serde(rename = "objectId")]
    object_id: Value,
    #[serde(default)]
    qualifier: Value,
}

/// The attribute types each event or object type declares.
fn declared(types: &[StandardType]) -> HashMap<&str, HashMap<&str, String>> {
    types
        .iter()
        .map(|t| {
            let attributes = t
                .attributes
                .iter()
                .map(|a| {
                    (
                        a.name.as_str(),
                        a.kind.clone().unwrap_or_default().to_lowercase(),
                    )
                })
                .collect();
            (t.name.as_str(), attributes)
        })
        .collect()
}

/// Converts a value to its declared type (pm4py's `_parse_attr_value`). A
/// value that does not convert keeps its JSON type.
fn typed(v: &Value, kind: Option<&String>) -> Option<AttributeValue> {
    if let Value::String(s) = v
        && s.trim().eq_ignore_ascii_case("null")
    {
        return None;
    }
    let kind = kind.map(String::as_str).unwrap_or_default();
    let raw = value(v)?;
    let s = match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    };
    let converted = if kind.contains("date") || kind.contains("time") {
        s.and_then(time::parse).map(AttributeValue::Date)
    } else if kind.contains("float") || kind.contains("double") {
        match v {
            Value::Number(n) => n.as_f64().map(AttributeValue::Float),
            _ => s
                .and_then(|s| s.trim().parse().ok())
                .map(AttributeValue::Float),
        }
    } else if kind.contains("int") {
        match v {
            Value::Number(n) => n
                .as_i64()
                .or_else(|| n.as_f64().map(|f| f.trunc() as i64))
                .map(AttributeValue::Int),
            _ => s
                .and_then(|s| s.trim().parse().ok())
                .map(AttributeValue::Int),
        }
    } else if kind.contains("bool") {
        Some(AttributeValue::Bool(match v {
            Value::String(s) if s.trim().eq_ignore_ascii_case("true") => true,
            Value::String(s) if s.trim().eq_ignore_ascii_case("false") => false,
            Value::String(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64() != Some(0.0),
            _ => true,
        }))
    } else {
        None
    };
    Some(converted.unwrap_or(raw))
}

fn standard(doc: StandardDoc) -> Result<Ocel> {
    let event_types = declared(&doc.event_types);
    let object_types = declared(&doc.object_types);
    let mut ocel = Ocel::new();
    let mut known: HashSet<Arc<str>> = HashSet::new();
    for o in &doc.objects {
        let id = text(&o.id);
        let kind_name = text(&o.kind);
        known.insert(id.clone());
        let types = object_types.get(&*kind_name);
        // The first value of each attribute is the object's; each later
        // value is a change at its time.
        let mut groups: Vec<(&str, Vec<&StandardAttribute>)> = Vec::new();
        for a in o.attributes.iter().flatten() {
            match groups.iter_mut().find(|(name, _)| *name == a.name) {
                Some((_, entries)) => entries.push(a),
                None => groups.push((&a.name, vec![a])),
            }
        }
        let mut attributes = Attributes::default();
        for (name, entries) in groups {
            let kind = types.and_then(|t| t.get(name));
            if let Some(v) = typed(&entries[0].value, kind) {
                attributes.insert(name, v);
            }
            for a in &entries[1..] {
                let Some(when) = a.time.as_ref().filter(|t| !t.is_null()) else {
                    continue;
                };
                ocel.object_changes.push(ObjectChange {
                    object: id.clone(),
                    object_type: kind_name.clone(),
                    timestamp: timestamp(&text(when))?,
                    field: name.into(),
                    value: typed(&a.value, kind),
                });
            }
        }
        for r in o.relationships.iter().flatten() {
            ocel.o2o.push(ObjectObject {
                source: id.clone(),
                target: text(&r.object_id),
                qualifier: qualifier(&r.qualifier),
            });
        }
        ocel.objects.push(OcelObject {
            id,
            object_type: kind_name,
            attributes,
        });
    }
    for e in &doc.events {
        let id = text(&e.id);
        let kind = text(&e.kind);
        let types = event_types.get(&*kind);
        let mut attributes = Attributes::default();
        for a in e.attributes.iter().flatten() {
            match typed(&a.value, types.and_then(|t| t.get(a.name.as_str()))) {
                Some(v) => {
                    attributes.insert(a.name.as_str(), v);
                }
                None => {
                    attributes.remove(&a.name);
                }
            }
        }
        let mut related: Vec<EventObject> = Vec::new();
        let mut index: HashMap<Arc<str>, usize> = HashMap::new();
        for r in e.relationships.iter().flatten() {
            let object = text(&r.object_id);
            if !known.contains(&object) {
                continue;
            }
            let q = qualifier(&r.qualifier);
            match index.get(&object) {
                Some(&i) => related[i].qualifier = q,
                None => {
                    index.insert(object.clone(), related.len());
                    related.push(EventObject {
                        event: id.clone(),
                        object,
                        qualifier: q,
                    });
                }
            }
        }
        ocel.relations.extend(related);
        ocel.events.push(OcelEvent {
            id,
            activity: kind,
            timestamp: timestamp(&e.time)?,
            attributes,
        });
    }
    Ok(finish(ocel))
}
