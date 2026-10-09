//! OCEL XML: the OCEL 1.0 layout (pm4py's `xmlocel` `classic` variant) and
//! the OCEL 2.0 layout (`ocel20`).

use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::{EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject};

use super::{finish, open, time};
use crate::error::{Error, Result};
use crate::model_xml::{self, Element};

const FORMAT: &str = "OCEL XML";
const MAX_DEPTH: usize = 64;

/// Reads an OCEL 1.0 XML file (pm4py's `read_ocel_xml`). A name ending in
/// `.gz` is decompressed.
pub fn read_ocel_xml(path: impl AsRef<Path>) -> Result<Ocel> {
    read_ocel_xml_from_reader(open(path.as_ref())?)
}

/// Reads OCEL 1.0 XML from a reader.
///
/// Each value's element name gives its type: `float`, `int`, `boolean`
/// and `date` convert, anything else is a string. The first value of an
/// object attribute is the object's attribute, and each later value is an
/// [`ObjectChange`] at its `timestamp`. An event relates to each object of
/// its `omap` once. Relations to objects that are not in the log are left
/// out.
pub fn read_ocel_xml_from_reader(input: impl BufRead) -> Result<Ocel> {
    classic(&model_xml::read(input, FORMAT, MAX_DEPTH, usize::MAX)?)
}

/// Reads an OCEL 2.0 XML file (pm4py's `read_ocel2_xml`). A name ending in
/// `.gz` is decompressed.
pub fn read_ocel2_xml(path: impl AsRef<Path>) -> Result<Ocel> {
    read_ocel2_xml_from_reader(open(path.as_ref())?)
}

/// Reads OCEL 2.0 XML from a reader.
///
/// Attribute values are converted to the types that `object-types` and
/// `event-types` declare: `float`, `integer`, `boolean` and `time`; anything
/// else is a string. The first value of an object attribute is the
/// object's attribute, and each later value is an [`ObjectChange`] at its
/// `time`. Relations to objects that are not in the log are left out.
pub fn read_ocel2_xml_from_reader(input: impl BufRead) -> Result<Ocel> {
    standard(&model_xml::read(input, FORMAT, MAX_DEPTH, usize::MAX)?)
}

fn timestamp(text: &str) -> Result<DateTime<FixedOffset>> {
    time::parse(text).ok_or_else(|| Error::Ocel(format!("invalid timestamp {text:?}")))
}

/// A value converted to the type a name gives (pm4py's `parse_xml`). A value
/// that does not convert stays a string.
fn typed(text: &str, kind: &str) -> AttributeValue {
    let kind = kind.to_lowercase();
    let t = text.trim();
    let converted = if kind.contains("float") || kind.contains("double") {
        t.parse().ok().map(AttributeValue::Float)
    } else if kind.contains("date") || kind.contains("time") {
        time::parse(t).map(AttributeValue::Date)
    } else if kind.contains("int") || kind == "long" {
        t.parse().ok().map(AttributeValue::Int)
    } else if kind.contains("bool") {
        match t.to_lowercase().as_str() {
            "true" => Some(AttributeValue::Bool(true)),
            "false" => Some(AttributeValue::Bool(false)),
            _ => None,
        }
    } else {
        None
    };
    converted.unwrap_or_else(|| AttributeValue::String(text.into()))
}

fn ends_with(e: &Element, suffix: &str) -> bool {
    e.name.to_lowercase().ends_with(suffix)
}

fn key(e: &Element) -> &str {
    e.attr("key").unwrap_or_default()
}

fn value(e: &Element) -> &str {
    e.attr("value").unwrap_or_default()
}

fn classic(root: &Element) -> Result<Ocel> {
    let mut ocel = Ocel::new();
    let mut types: HashMap<Arc<str>, Arc<str>> = HashMap::new();
    let mut relations: Vec<EventObject> = Vec::new();
    for child in &root.children {
        if ends_with(child, "events") {
            for event in &child.children {
                let (mut id, mut activity, mut when) = ("", "", None);
                let mut related: Vec<EventObject> = Vec::new();
                let mut attributes = Attributes::default();
                for field in &event.children {
                    match key(field) {
                        "id" => id = value(field),
                        "timestamp" => when = Some(value(field)),
                        "activity" => activity = value(field),
                        "omap" => {
                            for o in &field.children {
                                let object: Arc<str> = value(o).into();
                                let qualifier = o.attr("qualifier").map(Arc::from);
                                match related.iter_mut().find(|r| r.object == object) {
                                    Some(r) => r.qualifier = qualifier,
                                    None => related.push(EventObject {
                                        event: "".into(),
                                        object,
                                        qualifier,
                                    }),
                                }
                            }
                        }
                        "vmap" => {
                            for a in &field.children {
                                attributes.insert(key(a), typed(value(a), &a.name));
                            }
                        }
                        _ => {}
                    }
                }
                let id: Arc<str> = id.into();
                for mut r in related {
                    r.event = id.clone();
                    relations.push(r);
                }
                let when =
                    when.ok_or_else(|| Error::Ocel(format!("event {id} has no timestamp")))?;
                ocel.events.push(OcelEvent {
                    id,
                    activity: activity.into(),
                    timestamp: timestamp(when)?,
                    attributes,
                });
            }
        } else if ends_with(child, "objects") {
            for object in &child.children {
                let (mut id, mut kind) = ("", "");
                let mut values: Vec<(&str, AttributeValue, Option<&str>)> = Vec::new();
                for field in &object.children {
                    match key(field) {
                        "id" => id = value(field),
                        "type" => kind = value(field),
                        "ovmap" => {
                            for a in &field.children {
                                values.push((
                                    key(a),
                                    typed(value(a), &a.name),
                                    a.attr("timestamp"),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                let id: Arc<str> = id.into();
                let kind: Arc<str> = kind.into();
                let mut attributes = Attributes::default();
                for (name, v, when) in values {
                    if !attributes.contains_key(name) {
                        attributes.insert(name, v);
                        continue;
                    }
                    let when = when.ok_or_else(|| {
                        Error::Ocel(format!("object {id}: repeated {name} has no timestamp"))
                    })?;
                    ocel.object_changes.push(ObjectChange {
                        object: id.clone(),
                        object_type: kind.clone(),
                        timestamp: timestamp(when)?,
                        field: name.into(),
                        value: Some(v),
                    });
                }
                types.insert(id.clone(), kind.clone());
                ocel.objects.push(OcelObject {
                    id,
                    object_type: kind,
                    attributes,
                });
            }
        } else if ends_with(child, "o2o") {
            for r in &child.children {
                ocel.o2o.push(ObjectObject {
                    source: r.attr("source").unwrap_or_default().into(),
                    target: r.attr("target").unwrap_or_default().into(),
                    qualifier: r.attr("qualifier").map(Arc::from),
                });
            }
        }
    }
    ocel.relations = relations
        .into_iter()
        .filter(|r| types.contains_key(&r.object))
        .collect();
    Ok(finish(ocel))
}

/// The attribute types that each `object-type` or `event-type` declares.
fn declared(e: &Element) -> HashMap<String, HashMap<String, String>> {
    e.children
        .iter()
        .map(|t| {
            let attributes = t
                .children
                .iter()
                .flat_map(|a| &a.children)
                .map(|a| {
                    (
                        a.attr("name").unwrap_or_default().to_owned(),
                        a.attr("type").unwrap_or("string").to_owned(),
                    )
                })
                .collect();
            (t.attr("name").unwrap_or_default().to_owned(), attributes)
        })
        .collect()
}

fn kind_of<'a>(
    types: &'a HashMap<String, HashMap<String, String>>,
    owner: &str,
    name: &str,
) -> &'a str {
    types
        .get(owner)
        .and_then(|t| t.get(name))
        .map(String::as_str)
        .unwrap_or("string")
}

fn standard(root: &Element) -> Result<Ocel> {
    let mut object_types = HashMap::new();
    let mut event_types = HashMap::new();
    for child in &root.children {
        if ends_with(child, "object-types") {
            object_types = declared(child);
        } else if ends_with(child, "event-types") {
            event_types = declared(child);
        }
    }
    let mut ocel = Ocel::new();
    let mut known: HashSet<Arc<str>> = HashSet::new();
    for child in &root.children {
        if ends_with(child, "object-types") || ends_with(child, "event-types") {
            continue;
        }
        if ends_with(child, "objects") {
            for object in &child.children {
                let id: Arc<str> = object.attr("id").unwrap_or_default().into();
                let kind: Arc<str> = object.attr("type").unwrap_or_default().into();
                known.insert(id.clone());
                let mut attributes = Attributes::default();
                for part in &object.children {
                    if ends_with(part, "objects") {
                        for r in &part.children {
                            ocel.o2o.push(ObjectObject {
                                source: id.clone(),
                                target: r.attr("object-id").unwrap_or_default().into(),
                                qualifier: r.attr("qualifier").map(Arc::from),
                            });
                        }
                    } else if ends_with(part, "attributes") {
                        for a in &part.children {
                            let name = a.attr("name").unwrap_or_default();
                            let v = typed(&a.text, kind_of(&object_types, &kind, name));
                            if !attributes.contains_key(name) {
                                attributes.insert(name, v);
                                continue;
                            }
                            let when = a.attr("time").ok_or_else(|| {
                                Error::Ocel(format!("object {id}: repeated {name} has no time"))
                            })?;
                            ocel.object_changes.push(ObjectChange {
                                object: id.clone(),
                                object_type: kind.clone(),
                                timestamp: timestamp(when)?,
                                field: name.into(),
                                value: Some(v),
                            });
                        }
                    }
                }
                ocel.objects.push(OcelObject {
                    id,
                    object_type: kind,
                    attributes,
                });
            }
        } else if ends_with(child, "events") {
            for event in &child.children {
                let id: Arc<str> = event.attr("id").unwrap_or_default().into();
                let kind: Arc<str> = event.attr("type").unwrap_or_default().into();
                let when = event
                    .attr("time")
                    .ok_or_else(|| Error::Ocel(format!("event {id} has no time")))?;
                let mut attributes = Attributes::default();
                for part in &event.children {
                    if ends_with(part, "objects") {
                        for r in &part.children {
                            let object: Arc<str> = r.attr("object-id").unwrap_or_default().into();
                            if known.contains(&object) {
                                ocel.relations.push(EventObject {
                                    event: id.clone(),
                                    object,
                                    qualifier: r.attr("qualifier").map(Arc::from),
                                });
                            }
                        }
                    } else if ends_with(part, "attributes") {
                        for a in &part.children {
                            let name = a.attr("name").unwrap_or_default();
                            attributes
                                .insert(name, typed(&a.text, kind_of(&event_types, &kind, name)));
                        }
                    }
                }
                ocel.events.push(OcelEvent {
                    id,
                    activity: kind,
                    timestamp: timestamp(when)?,
                    attributes,
                });
            }
        }
    }
    Ok(finish(ocel))
}
