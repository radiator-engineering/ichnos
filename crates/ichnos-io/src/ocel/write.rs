//! Object-centric event log writers: OCEL 1.0 and OCEL 2.0, as JSON and XML
//! (pm4py's `write_ocel_json`, `write_ocel_xml`, `write_ocel2_json`,
//! `write_ocel2_xml`, `write_ocel` and `write_ocel2`).
//!
//! Each writer first runs [`Ocel::make_consistent`] and
//! [`Ocel::retain_related`] on a copy, as pm4py's do, so an event or object
//! without a relation is not written.
//!
//! pm4py keeps each table as a pandas data frame, and a column's type decides
//! how its values are written. Here a column is an attribute name, and its
//! type follows from the values, as pandas infers it when it builds the
//! frame: a column whose values are all numbers, with at least one float or
//! a row without a value, is a float column; a column whose values are all
//! dates is a date column; anything else is an object column. Attributes are
//! written in the order their names first appear in the table.
//!
//! The output is pm4py's byte for byte when the attribute order agrees:
//! JSON as Python's `json.dump(indent=2)` writes it, with non-ASCII
//! characters escaped, and XML as lxml's pretty printer writes it.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, Timelike, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::Ocel;
use ichnos_ocel::constants::{
    CHANGED_FIELD, CURRENT_VERSION, DEFAULT_ORDERING, EVENT_ACTIVITY, EVENT_TIMESTAMP,
    EVENT_TYPES_KEY, EVENTS_KEY, GLOBAL_EVENT, GLOBAL_LOG, GLOBAL_LOG_ATTRIBUTE_NAMES,
    GLOBAL_LOG_OBJECT_TYPES, GLOBAL_LOG_ORDERING, GLOBAL_LOG_VERSION, GLOBAL_OBJECT, O2O_KEY,
    OBJECT_CHANGES_KEY, OBJECT_ID, OBJECT_TYPE, OBJECT_TYPES_KEY, OBJECTS_KEY, OMAP_KEY, OVMAP_KEY,
    QUALIFIER, TYPED_OMAP_KEY, VMAP_KEY,
};

use super::{lower_name, unsupported};
use crate::error::{Error, Result};

/// The time pm4py gives an object's attribute values in OCEL 2.0.
const OBJECT_ATTRIBUTE_TIME: &str = "1970-01-01T00:00:00Z";

/// Writes an OCEL log as JSON (pm4py's `write_ocel_json`). A log with OCEL
/// 2.0 features ([`Ocel::is_ocel20`]) gets pm4py's `ocel20` layout, which
/// adds event and object types, object changes, qualified relations and
/// object-to-object relations to OCEL 1.0's; any other log gets the
/// `classic` OCEL 1.0 layout. A path ending in `.gz` is compressed.
pub fn write_ocel_json(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    to_path(path.as_ref(), |out| write_ocel_json_to_writer(ocel, out))
}

/// Writes an OCEL log as JSON to `output`, as [`write_ocel_json`] does.
pub fn write_ocel_json_to_writer(ocel: &Ocel, mut output: impl Write) -> Result<()> {
    let ocel20 = ocel.is_ocel20();
    let log = Prepared::new(ocel);
    let json = if ocel20 {
        log.enriched_json()
    } else {
        log.classic_json()
    };
    output.write_all(json.dump().as_bytes())?;
    Ok(())
}

/// Writes an OCEL 1.0 log as XML (pm4py's `write_ocel_xml`, the `classic`
/// variant). A path ending in `.gz` is compressed.
pub fn write_ocel_xml(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    to_path(path.as_ref(), |out| write_ocel_xml_to_writer(ocel, out))
}

/// Writes an OCEL 1.0 log as XML to `output`, as [`write_ocel_xml`] does.
pub fn write_ocel_xml_to_writer(ocel: &Ocel, mut output: impl Write) -> Result<()> {
    let xml = Prepared::new(ocel).classic_xml()?;
    output.write_all(xml.as_bytes())?;
    Ok(())
}

/// Writes an OCEL 2.0 log as JSON (pm4py's `write_ocel2_json`, the
/// `ocel20_standard` variant). A path ending in `.gz` is compressed.
pub fn write_ocel2_json(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    to_path(path.as_ref(), |out| write_ocel2_json_to_writer(ocel, out))
}

/// Writes an OCEL 2.0 log as JSON to `output`, as [`write_ocel2_json`] does.
pub fn write_ocel2_json_to_writer(ocel: &Ocel, mut output: impl Write) -> Result<()> {
    let json = Prepared::new(ocel).standard_json();
    output.write_all(json.dump().as_bytes())?;
    Ok(())
}

/// Writes an OCEL 2.0 log as XML (pm4py's `write_ocel2_xml`, the `ocel20`
/// variant). A path ending in `.gz` is compressed.
pub fn write_ocel2_xml(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    to_path(path.as_ref(), |out| write_ocel2_xml_to_writer(ocel, out))
}

/// Writes an OCEL 2.0 log as XML to `output`, as [`write_ocel2_xml`] does.
pub fn write_ocel2_xml_to_writer(ocel: &Ocel, mut output: impl Write) -> Result<()> {
    let xml = Prepared::new(ocel).ocel20_xml()?;
    output.write_all(xml.as_bytes())?;
    Ok(())
}

/// Writes an OCEL 1.0 log, choosing the format by extension as pm4py's
/// `write_ocel` does: a name ending in `csv` is CSV, without an objects
/// table, one ending in `jsonocel` is JSON and one ending in `xmlocel` is
/// XML. pm4py's SQLite (`sqlite`) writer is not ported yet.
pub fn write_ocel(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let name = lower_name(path);
    if name.ends_with("csv") {
        super::write_ocel_csv(ocel, path, None)
    } else if name.ends_with("sqlite") {
        Err(not_ported(path))
    } else if name.ends_with("jsonocel") {
        write_ocel_json(ocel, path)
    } else if name.ends_with("xmlocel") {
        write_ocel_xml(ocel, path)
    } else {
        Err(unsupported(path))
    }
}

/// Writes an OCEL 2.0 log, choosing the format by extension as pm4py's
/// `write_ocel2` does: a name ending in `xml` or `xmlocel` is XML and one
/// ending in `json` or `jsonocel` is JSON, each optionally followed by
/// `.gz`, and one ending in `.ocel.csv` is CSV. pm4py's bundle
/// (`.ocel.zip`) and SQLite (`sqlite`) writers are not ported yet.
pub fn write_ocel2(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let name = lower_name(path);
    let matches = |extensions: [&str; 2]| {
        extensions
            .iter()
            .any(|e| name.ends_with(e) || name.ends_with(&format!("{e}.gz")))
    };
    if name.ends_with(".ocel.zip") || name.ends_with("sqlite") {
        Err(not_ported(path))
    } else if name.ends_with(".ocel.csv") {
        super::write_ocel2_csv(ocel, path)
    } else if matches(["xml", "xmlocel"]) {
        write_ocel2_xml(ocel, path)
    } else if matches(["json", "jsonocel"]) {
        write_ocel2_json(ocel, path)
    } else {
        Err(unsupported(path))
    }
}

fn not_ported(path: &Path) -> Error {
    Error::Ocel(format!(
        "writing this OCEL format is not ported yet: {}",
        path.display()
    ))
}

/// Creates `path` and writes to it, through gzip when the name ends in
/// `.gz`.
pub(super) fn to_path(path: &Path, write: impl FnOnce(&mut dyn Write) -> Result<()>) -> Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    if lower_name(path).ends_with(".gz") {
        let mut encoder = GzEncoder::new(&mut output, Compression::default());
        write(&mut encoder)?;
        encoder.finish()?;
    } else {
        write(&mut output)?;
    }
    output.flush()?;
    Ok(())
}

/// How pandas types a column, as far as the writers care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// `float64`: numbers, with a float or a missing value among them.
    Float,
    /// `datetime64`: dates, possibly missing.
    Date,
    /// Every row holds an integer (`int64`).
    Int,
    /// Every row holds a boolean (`bool`).
    Bool,
    /// Anything else (`object`).
    Object,
}

/// The attribute columns of one table: their names in first-appearance
/// order and their kinds.
pub(super) struct Columns {
    pub(super) names: Vec<Arc<str>>,
    kinds: HashMap<Arc<str>, Kind>,
}

impl Columns {
    pub(super) fn new<'a>(rows: impl Iterator<Item = &'a Attributes>) -> Self {
        // Per column: rows with a value, and which value types occur.
        #[derive(Default)]
        struct Seen {
            rows: usize,
            int: bool,
            float: bool,
            bool: bool,
            date: bool,
            other: bool,
        }
        let mut names: Vec<Arc<str>> = Vec::new();
        let mut seen: HashMap<Arc<str>, Seen> = HashMap::new();
        let mut total = 0;
        for row in rows {
            total += 1;
            for (key, value) in row.iter() {
                let s = seen.entry(key.clone()).or_insert_with(|| {
                    names.push(key.clone());
                    Seen::default()
                });
                s.rows += 1;
                match value.plain() {
                    AttributeValue::Int(_) => s.int = true,
                    AttributeValue::Float(f) if f.is_nan() => {
                        s.float = true;
                        s.rows -= 1;
                    }
                    AttributeValue::Float(_) => s.float = true,
                    AttributeValue::Bool(_) => s.bool = true,
                    AttributeValue::Date(_) => s.date = true,
                    _ => s.other = true,
                }
            }
        }
        let kinds = seen
            .into_iter()
            .map(|(name, s)| {
                let missing = s.rows < total;
                let kind = if s.other || (s.bool && (s.int || s.float || s.date)) {
                    Kind::Object
                } else if s.date {
                    if s.int || s.float {
                        Kind::Object
                    } else {
                        Kind::Date
                    }
                } else if s.float || (s.int && missing) {
                    Kind::Float
                } else if s.int {
                    Kind::Int
                } else if s.bool && !missing {
                    Kind::Bool
                } else {
                    Kind::Object
                };
                (name, kind)
            })
            .collect();
        Self { names, kinds }
    }

    pub(super) fn kind(&self, name: &str) -> Kind {
        self.kinds.get(name).copied().unwrap_or(Kind::Object)
    }

    /// The row's present values in column order.
    fn ordered<'a>(
        &'a self,
        row: &'a Attributes,
    ) -> impl Iterator<Item = (&'a Arc<str>, &'a AttributeValue)> {
        self.names
            .iter()
            .filter_map(move |n| row.get(n).filter(|v| !missing(v)).map(|v| (n, v)))
    }
}

/// pandas' missing value: here an absent attribute or a float NaN.
pub(super) fn missing(value: &AttributeValue) -> bool {
    matches!(value.plain(), AttributeValue::Float(f) if f.is_nan())
}

/// The type pm4py's `attributes_per_type` gives a column within one group of
/// rows, reduced to what the writers write: `date`, `float` or `string`.
fn attribute_type(kind: Kind, values: &[&AttributeValue]) -> &'static str {
    match kind {
        Kind::Float => "float",
        Kind::Date => "date",
        Kind::Int | Kind::Bool => "string",
        Kind::Object => {
            let all = |f: fn(&AttributeValue) -> bool| values.iter().all(|v| f(v.plain()));
            if all(|v| matches!(v, AttributeValue::Bool(_) | AttributeValue::Int(_))) {
                "string"
            } else if all(|v| {
                matches!(
                    v,
                    AttributeValue::Bool(_) | AttributeValue::Int(_) | AttributeValue::Float(_)
                )
            }) {
                "float"
            } else if all(|v| matches!(v, AttributeValue::Date(_))) {
                "date"
            } else {
                "string"
            }
        }
    }
}

/// Each group with its columns and their type names, in pm4py's order.
type GroupTypes = Vec<(Arc<str>, Vec<(Arc<str>, &'static str)>)>;

/// pm4py's `attributes_per_type.get` for one table: per group, in sorted
/// group order, each column with a value in the group and its type.
fn types_per_group<'a>(
    rows: impl Iterator<Item = (&'a Arc<str>, &'a Attributes)>,
    columns: &Columns,
) -> GroupTypes {
    let mut groups: Vec<(&Arc<str>, Vec<&Attributes>)> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (group, row) in rows {
        let i = *index.entry(&**group).or_insert_with(|| {
            groups.push((group, Vec::new()));
            groups.len() - 1
        });
        groups[i].1.push(row);
    }
    groups.sort_by(|a, b| a.0.cmp(b.0));
    groups
        .into_iter()
        .map(|(group, rows)| {
            let types = columns
                .names
                .iter()
                .filter_map(|name| {
                    let values: Vec<&AttributeValue> = rows
                        .iter()
                        .filter_map(|r| r.get(name))
                        .filter(|v| !missing(v))
                        .collect();
                    (!values.is_empty())
                        .then(|| (name.clone(), attribute_type(columns.kind(name), &values)))
                })
                .collect();
            (group.clone(), types)
        })
        .collect()
}

/// A copy of the log after pm4py's consistency step and relation filter,
/// with what every writer needs.
pub(super) struct Prepared {
    pub(super) ocel: Ocel,
    pub(super) events: Columns,
    pub(super) objects: Columns,
    /// The object changes as rows of pm4py's `object_changes` table: each
    /// holds its changed field's value, if it has one.
    change_rows: Vec<Attributes>,
    changes: Columns,
}

impl Prepared {
    pub(super) fn new(ocel: &Ocel) -> Self {
        let mut ocel = ocel.clone();
        ocel.make_consistent();
        ocel.retain_related();
        let events = Columns::new(ocel.events.iter().map(|e| &e.attributes));
        let objects = Columns::new(ocel.objects.iter().map(|o| &o.attributes));
        let change_rows: Vec<Attributes> = ocel
            .object_changes
            .iter()
            .map(|c| {
                let mut row = Attributes::default();
                if let Some(v) = &c.value {
                    row.insert(c.field.clone(), v.clone());
                }
                row
            })
            .collect();
        let changes = Columns::new(change_rows.iter());
        Self {
            ocel,
            events,
            objects,
            change_rows,
            changes,
        }
    }

    /// The related object ids of each event, in relation order (pm4py's
    /// `related_objects_dct_overall`).
    fn related(&self) -> HashMap<&str, Vec<&Arc<str>>> {
        let mut related: HashMap<&str, Vec<&Arc<str>>> = HashMap::new();
        for r in &self.ocel.relations {
            related.entry(&r.event).or_default().push(&r.object);
        }
        related
    }

    /// The object types in order of first appearance.
    fn object_types(&self) -> Vec<&Arc<str>> {
        let mut seen = HashSet::new();
        self.ocel
            .objects
            .iter()
            .map(|o| &o.object_type)
            .filter(|t| seen.insert(&***t))
            .collect()
    }

    /// The sorted attribute names of the events and objects (pm4py's
    /// `attributes_names.get_attribute_names`).
    fn attribute_names(&self) -> Vec<&Arc<str>> {
        let mut names: Vec<&Arc<str>> = self
            .events
            .names
            .iter()
            .chain(&self.objects.names)
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// The global event and object maps, or pm4py's defaults.
    fn globals(&self) -> [(&'static str, Json); 2] {
        [(GLOBAL_EVENT, EVENT_ACTIVITY), (GLOBAL_OBJECT, OBJECT_TYPE)].map(|(key, item)| {
            let json = match self.ocel.globals.get(key) {
                Some(v) => Json::from_value(v),
                None => Json::Obj(vec![(item.into(), Json::Str("__INVALID__".into()))]),
            };
            (key, json)
        })
    }

    /// pm4py's `get_types`: per activity and per object type, the attribute
    /// types. Object types that only the object changes have come last.
    fn types(&self) -> (GroupTypes, GroupTypes) {
        let ets = types_per_group(
            self.ocel
                .events
                .iter()
                .map(|e| (&e.activity, &e.attributes)),
            &self.events,
        );
        let mut ots = types_per_group(
            self.ocel
                .objects
                .iter()
                .map(|o| (&o.object_type, &o.attributes)),
            &self.objects,
        );
        let changed = types_per_group(
            self.ocel
                .object_changes
                .iter()
                .zip(&self.change_rows)
                .map(|(c, row)| (&c.object_type, row)),
            &self.changes,
        );
        for (group, types) in changed {
            match ots.iter_mut().find(|(g, _)| *g == group) {
                Some((_, known)) => {
                    for (name, t) in types {
                        if !known.iter().any(|(n, _)| *n == name) {
                            known.push((name, t));
                        }
                    }
                }
                None => ots.push((group, types)),
            }
        }
        (ets, ots)
    }

    /// pm4py's `classic.get_base_json_object`.
    fn classic_json(&self) -> Json {
        let related = self.related();
        let [global_event, global_object] = self.globals();
        let mut log = Json::Obj(Vec::new());
        log.set(global_event.0, global_event.1);
        log.set(global_object.0, global_object.1);
        log.set(
            GLOBAL_LOG,
            Json::Obj(vec![
                (
                    GLOBAL_LOG_OBJECT_TYPES.into(),
                    Json::Arr(
                        self.object_types()
                            .into_iter()
                            .map(|t| Json::Str(t.to_string()))
                            .collect(),
                    ),
                ),
                (
                    GLOBAL_LOG_ATTRIBUTE_NAMES.into(),
                    Json::Arr(
                        self.attribute_names()
                            .into_iter()
                            .map(|n| Json::Str(n.to_string()))
                            .collect(),
                    ),
                ),
                (GLOBAL_LOG_VERSION.into(), Json::Str(CURRENT_VERSION.into())),
                (
                    GLOBAL_LOG_ORDERING.into(),
                    Json::Str(DEFAULT_ORDERING.into()),
                ),
            ]),
        );
        let mut events = Json::Obj(Vec::new());
        for e in &self.ocel.events {
            let vmap = self
                .events
                .ordered(&e.attributes)
                .map(|(k, v)| (k.to_string(), classic_json_value(v, self.events.kind(k))))
                .collect();
            let omap = related
                .get(&*e.id)
                .map(|os| os.iter().map(|o| Json::Str(o.to_string())).collect())
                .unwrap_or_default();
            events.set(
                &e.id,
                Json::Obj(vec![
                    (EVENT_ACTIVITY.into(), Json::Str(e.activity.to_string())),
                    (EVENT_TIMESTAMP.into(), Json::Str(strftime_z(&e.timestamp))),
                    (VMAP_KEY.into(), Json::Obj(vmap)),
                    (OMAP_KEY.into(), Json::Arr(omap)),
                ]),
            );
        }
        log.set(EVENTS_KEY, events);
        let mut objects = Json::Obj(Vec::new());
        for o in &self.ocel.objects {
            let ovmap = self
                .objects
                .ordered(&o.attributes)
                .map(|(k, v)| (k.to_string(), classic_json_value(v, self.objects.kind(k))))
                .collect();
            objects.set(
                &o.id,
                Json::Obj(vec![
                    (OBJECT_TYPE.into(), Json::Str(o.object_type.to_string())),
                    (OVMAP_KEY.into(), Json::Obj(ovmap)),
                ]),
            );
        }
        log.set(OBJECTS_KEY, objects);
        log
    }

    /// pm4py's `ocel20.get_enriched_object`.
    fn enriched_json(&self) -> Json {
        let mut log = self.classic_json();
        let (ets, ots) = self.types();
        log.set(EVENT_TYPES_KEY, types_json(ets));
        log.set(OBJECT_TYPES_KEY, types_json(ots));
        let changes = self
            .ocel
            .object_changes
            .iter()
            .map(|c| {
                let mut change = vec![
                    (OBJECT_ID.into(), Json::Str(c.object.to_string())),
                    (OBJECT_TYPE.into(), Json::Str(c.object_type.to_string())),
                    (EVENT_TIMESTAMP.into(), Json::Str(isoformat(&c.timestamp))),
                    (CHANGED_FIELD.into(), Json::Str(c.field.to_string())),
                ];
                if let Some(v) = c.value.as_ref().filter(|v| !missing(v)) {
                    change.push((
                        c.field.to_string(),
                        normalized_json(v, self.changes.kind(&c.field)),
                    ));
                }
                Json::Obj(change)
            })
            .collect();
        log.set(OBJECT_CHANGES_KEY, Json::Arr(changes));
        let Json::Obj(top) = &mut log else {
            unreachable!()
        };
        for (key, value) in top.iter_mut() {
            let Json::Obj(rows) = value else { continue };
            if key == EVENTS_KEY {
                for r in &self.ocel.relations {
                    let row = Json::get_mut(rows, &r.event);
                    row.push_to(
                        TYPED_OMAP_KEY,
                        Json::Obj(vec![
                            (OBJECT_ID.into(), Json::Str(r.object.to_string())),
                            (QUALIFIER.into(), qualifier_json(&r.qualifier)),
                        ]),
                    );
                }
            } else if key == OBJECTS_KEY {
                for r in &self.ocel.o2o {
                    let row = Json::get_mut(rows, &r.source);
                    row.push_to(
                        O2O_KEY,
                        Json::Obj(vec![
                            (OBJECT_ID.into(), Json::Str(r.target.to_string())),
                            (QUALIFIER.into(), qualifier_json(&r.qualifier)),
                        ]),
                    );
                }
            }
        }
        log
    }

    /// pm4py's `ocel20_standard.apply`, which rewrites the enriched object.
    fn standard_json(&self) -> Json {
        let legacy = self.enriched_json();
        let Json::Obj(legacy) = legacy else {
            unreachable!()
        };
        let field = |key: &str| {
            legacy
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .expect("the enriched object has every key")
        };
        let types = |key: &str| {
            let Json::Obj(types) = field(key) else {
                unreachable!()
            };
            Json::Arr(
                types
                    .iter()
                    .map(|(name, attrs)| {
                        let Json::Obj(attrs) = attrs else {
                            unreachable!()
                        };
                        Json::Obj(vec![
                            ("name".into(), Json::Str(name.clone())),
                            (
                                "attributes".into(),
                                Json::Arr(
                                    attrs
                                        .iter()
                                        .map(|(n, t)| {
                                            Json::Obj(vec![
                                                ("name".into(), Json::Str(n.clone())),
                                                ("type".into(), t.clone()),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            )
        };
        let relationships = |list: Option<&Json>| -> Option<Json> {
            let Some(Json::Arr(list)) = list else {
                return None;
            };
            (!list.is_empty()).then(|| {
                Json::Arr(
                    list.iter()
                        .map(|r| {
                            Json::Obj(vec![
                                ("objectId".into(), r.field(OBJECT_ID).clone()),
                                ("qualifier".into(), r.field(QUALIFIER).clone()),
                            ])
                        })
                        .collect(),
                )
            })
        };
        let mut objects: Vec<Json> = Vec::new();
        let mut object_index: HashMap<&str, usize> = HashMap::new();
        let Json::Obj(legacy_objects) = field(OBJECTS_KEY) else {
            unreachable!()
        };
        for (id, object) in legacy_objects {
            let mut descr = vec![
                ("id".into(), Json::Str(id.clone())),
                ("type".into(), object.field(OBJECT_TYPE).clone()),
            ];
            if let Some(Json::Obj(ovmap)) = object.get(OVMAP_KEY)
                && !ovmap.is_empty()
            {
                let attributes = ovmap
                    .iter()
                    .map(|(k, v)| {
                        Json::Obj(vec![
                            ("name".into(), Json::Str(k.clone())),
                            ("time".into(), Json::Str(OBJECT_ATTRIBUTE_TIME.into())),
                            ("value".into(), v.clone()),
                        ])
                    })
                    .collect();
                descr.push(("attributes".into(), Json::Arr(attributes)));
            }
            if let Some(r) = relationships(object.get(O2O_KEY)) {
                descr.push(("relationships".into(), r));
            }
            object_index.insert(id.as_str(), objects.len());
            objects.push(Json::Obj(descr));
        }
        let mut events: Vec<Json> = Vec::new();
        let Json::Obj(legacy_events) = field(EVENTS_KEY) else {
            unreachable!()
        };
        for (id, event) in legacy_events {
            let mut descr = vec![
                ("id".into(), Json::Str(id.clone())),
                ("type".into(), event.field(EVENT_ACTIVITY).clone()),
                ("time".into(), event.field(EVENT_TIMESTAMP).clone()),
            ];
            if let Some(Json::Obj(vmap)) = event.get(VMAP_KEY)
                && !vmap.is_empty()
            {
                let attributes = vmap
                    .iter()
                    .map(|(k, v)| {
                        Json::Obj(vec![
                            ("name".into(), Json::Str(k.clone())),
                            ("value".into(), v.clone()),
                        ])
                    })
                    .collect();
                descr.push(("attributes".into(), Json::Arr(attributes)));
            }
            if let Some(r) = relationships(event.get(TYPED_OMAP_KEY)) {
                descr.push(("relationships".into(), r));
            }
            events.push(Json::Obj(descr));
        }
        let Json::Arr(changes) = field(OBJECT_CHANGES_KEY) else {
            unreachable!()
        };
        for change in changes {
            let Some(Json::Str(name)) = change.get(CHANGED_FIELD) else {
                continue;
            };
            let Some(value) = change.get(name) else {
                continue;
            };
            let Some(Json::Str(oid)) = change.get(OBJECT_ID) else {
                continue;
            };
            let Some(&i) = object_index.get(oid.as_str()) else {
                continue;
            };
            let entry = Json::Obj(vec![
                ("name".into(), Json::Str(name.clone())),
                ("time".into(), change.field(EVENT_TIMESTAMP).clone()),
                ("value".into(), value.clone()),
            ]);
            objects[i].push_to("attributes", entry);
        }
        Json::Obj(vec![
            ("objectTypes".into(), types(OBJECT_TYPES_KEY)),
            ("eventTypes".into(), types(EVENT_TYPES_KEY)),
            ("objects".into(), Json::Arr(objects)),
            ("events".into(), Json::Arr(events)),
        ])
    }

    /// pm4py's `xmlocel` `classic` exporter.
    fn classic_xml(&self) -> Result<String> {
        let related = self.related();
        let mut root = El::new("log");
        for (scope, (_, items)) in ["event", "object"].into_iter().zip(self.globals()) {
            let mut global = El::new("global").attr("scope", scope);
            let Json::Obj(items) = items else {
                return Err(Error::Ocel(format!("the global {scope} map is not a map")));
            };
            for (k, v) in items {
                let Json::Str(v) = v else {
                    return Err(Error::Ocel(format!(
                        "the global {scope} value of {k} is not a string"
                    )));
                };
                global.push(El::new("string").attr("key", k).attr("value", v));
            }
            root.push(global);
        }
        let mut global = El::new("global").attr("scope", "log");
        let mut names = El::new("list").attr("key", "attribute-names");
        for n in self.attribute_names() {
            names.push(
                El::new("string")
                    .attr("key", "attribute-name")
                    .attr("value", &**n),
            );
        }
        let mut types = El::new("list").attr("key", "object-types");
        for t in self.object_types() {
            types.push(
                El::new("string")
                    .attr("key", "object-type")
                    .attr("value", &**t),
            );
        }
        global.push(names);
        global.push(types);
        global.push(
            El::new("string")
                .attr("key", "version")
                .attr("value", CURRENT_VERSION),
        );
        global.push(
            El::new("string")
                .attr("key", "ordering")
                .attr("value", DEFAULT_ORDERING),
        );
        root.push(global);
        let mut events = El::new("events");
        for e in &self.ocel.events {
            let mut event = El::new("event");
            event.push(El::new("string").attr("key", "id").attr("value", &*e.id));
            event.push(
                El::new("string")
                    .attr("key", "activity")
                    .attr("value", &*e.activity),
            );
            event.push(
                El::new("date")
                    .attr("key", "timestamp")
                    .attr("value", strftime_z(&e.timestamp)),
            );
            let mut omap = El::new("list").attr("key", "omap");
            for o in related.get(&*e.id).into_iter().flatten() {
                omap.push(
                    El::new("string")
                        .attr("key", "object-id")
                        .attr("value", &***o),
                );
            }
            event.push(omap);
            let mut vmap = El::new("list").attr("key", "vmap");
            for (k, v) in self.events.ordered(&e.attributes) {
                let kind = self.events.kind(k);
                vmap.push(
                    El::new(classic_xml_tag(kind))
                        .attr("key", &**k)
                        .attr("value", classic_text(v, kind)),
                );
            }
            event.push(vmap);
            events.push(event);
        }
        root.push(events);
        let mut objects = El::new("objects");
        for o in &self.ocel.objects {
            let mut object = El::new("object");
            object.push(El::new("string").attr("key", "id").attr("value", &*o.id));
            object.push(
                El::new("string")
                    .attr("key", "type")
                    .attr("value", &*o.object_type),
            );
            let mut ovmap = El::new("list").attr("key", "ovmap");
            for (k, v) in self.objects.ordered(&o.attributes) {
                let kind = self.objects.kind(k);
                ovmap.push(
                    El::new(classic_xml_tag(kind))
                        .attr("key", &**k)
                        .attr("value", classic_text(v, kind)),
                );
            }
            object.push(ovmap);
            objects.push(object);
        }
        root.push(objects);
        root.document()
    }

    /// pm4py's `xmlocel` `ocel20` exporter.
    fn ocel20_xml(&self) -> Result<String> {
        let (ets, ots) = self.types();
        let mut root = El::new("log");
        for (list, item, types) in [
            ("object-types", "object-type", ots),
            ("event-types", "event-type", ets),
        ] {
            let mut list = El::new(list);
            for (name, attrs) in types {
                let mut attributes = El::new("attributes");
                for (n, t) in attrs {
                    attributes.push(El::new("attribute").attr("name", &*n).attr("type", t));
                }
                list.push(El::new(item).attr("name", &*name).child(attributes));
            }
            root.push(list);
        }
        let mut changes: HashMap<&str, Vec<El>> = HashMap::new();
        for c in &self.ocel.object_changes {
            let text = match c.value.as_ref().filter(|v| !missing(v)) {
                Some(v) => normalized_text(v, self.changes.kind(&c.field)),
                None => "None".to_string(),
            };
            changes.entry(&c.object).or_default().push(
                El::new("attribute")
                    .attr("name", &*c.field)
                    .attr("time", isoformat(&c.timestamp))
                    .text(text),
            );
        }
        let mut o2o: HashMap<&str, Vec<El>> = HashMap::new();
        for r in &self.ocel.o2o {
            o2o.entry(&r.source).or_default().push(
                El::new("relationship")
                    .attr("object-id", &*r.target)
                    .attr("qualifier", r.qualifier.as_deref().unwrap_or_default()),
            );
        }
        let mut objects = El::new("objects");
        for o in dict_rows(&self.ocel.objects, |o| &o.id) {
            let mut attributes = El::new("attributes");
            for (k, v) in self.objects.ordered(&o.attributes) {
                attributes.push(
                    El::new("attribute")
                        .attr("name", &**k)
                        .attr("time", OBJECT_ATTRIBUTE_TIME)
                        .text(normalized_text(v, self.objects.kind(k))),
                );
            }
            for c in changes.remove(&*o.id).into_iter().flatten() {
                attributes.push(c);
            }
            let mut object = El::new("object")
                .attr("id", &*o.id)
                .attr("type", &*o.object_type)
                .child(attributes);
            if let Some(rs) = o2o.remove(&*o.id) {
                let mut list = El::new("objects");
                for r in rs {
                    list.push(r);
                }
                object.push(list);
            }
            objects.push(object);
        }
        root.push(objects);
        let mut relations: HashMap<&str, Vec<El>> = HashMap::new();
        for r in &self.ocel.relations {
            relations.entry(&r.event).or_default().push(
                El::new("relationship")
                    .attr("object-id", &*r.object)
                    .attr("qualifier", r.qualifier.as_deref().unwrap_or_default()),
            );
        }
        let mut events = El::new("events");
        for e in dict_rows(&self.ocel.events, |e| &e.id) {
            let mut attributes = El::new("attributes");
            for (k, v) in self.events.ordered(&e.attributes) {
                attributes.push(
                    El::new("attribute")
                        .attr("name", &**k)
                        .text(normalized_text(v, self.events.kind(k))),
                );
            }
            let mut objects = El::new("objects");
            for r in relations.remove(&*e.id).into_iter().flatten() {
                objects.push(r);
            }
            events.push(
                El::new("event")
                    .attr("id", &*e.id)
                    .attr("type", &*e.activity)
                    .attr("time", isoformat(&e.timestamp))
                    .child(attributes)
                    .child(objects),
            );
        }
        root.push(events);
        root.document()
    }
}

/// The rows of a table read into a Python dict by id: one per id, at the
/// first position, holding the last row with that id.
fn dict_rows<T>(rows: &[T], id: impl Fn(&T) -> &Arc<str>) -> Vec<&T> {
    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut out: Vec<&T> = Vec::new();
    for row in rows {
        match index.get(&**id(row)) {
            Some(&i) => out[i] = row,
            None => {
                index.insert(id(row), out.len());
                out.push(row);
            }
        }
    }
    out
}

fn types_json(types: GroupTypes) -> Json {
    Json::Obj(
        types
            .into_iter()
            .map(|(group, attrs)| {
                (
                    group.to_string(),
                    Json::Obj(
                        attrs
                            .into_iter()
                            .map(|(n, t)| (n.to_string(), Json::Str(t.into())))
                            .collect(),
                    ),
                )
            })
            .collect(),
    )
}

fn qualifier_json(qualifier: &Option<Arc<str>>) -> Json {
    match qualifier {
        Some(q) => Json::Str(q.to_string()),
        None => Json::Null,
    }
}

/// The classic XML tag of a column (pm4py's `get_type` on its dtype).
fn classic_xml_tag(kind: Kind) -> &'static str {
    match kind {
        Kind::Float => "float",
        Kind::Date => "date",
        _ => "string",
    }
}

pub(super) fn as_float(value: &AttributeValue) -> Option<f64> {
    match value.plain() {
        AttributeValue::Int(i) => Some(*i as f64),
        AttributeValue::Float(f) => Some(*f),
        _ => None,
    }
}

/// A value of the OCEL 1.0 tables as JSON: pm4py's
/// `clean_dataframes.get_dataframes_from_ocel` writes a date column with
/// `strftime`, to the second, and other dates with `isoformat`.
fn classic_json_value(value: &AttributeValue, kind: Kind) -> Json {
    match (kind, value.plain()) {
        (Kind::Date, AttributeValue::Date(d)) => Json::Str(strftime_z(d)),
        _ => normalized_json(value, kind),
    }
}

/// A value as JSON after pm4py's `clean_dataframes.normalize_value`.
fn normalized_json(value: &AttributeValue, kind: Kind) -> Json {
    if kind == Kind::Float
        && let Some(f) = as_float(value)
    {
        return Json::Float(f);
    }
    match value.plain() {
        AttributeValue::Date(d) => Json::Str(isoformat(d)),
        other => Json::from_value(other),
    }
}

/// A value as the classic XML writes it: Python's `str` of the cleaned
/// table's value.
fn classic_text(value: &AttributeValue, kind: Kind) -> String {
    match (kind, value.plain()) {
        (Kind::Date, AttributeValue::Date(d)) => strftime_z(d),
        _ => normalized_text(value, kind),
    }
}

/// A value as Python's `str` of pm4py's `normalize_value` writes it.
pub(super) fn normalized_text(value: &AttributeValue, kind: Kind) -> String {
    if kind == Kind::Float
        && let Some(f) = as_float(value)
    {
        return AttributeValue::Float(f).to_string();
    }
    match value.plain() {
        AttributeValue::Date(d) => isoformat(d),
        other => other.to_string(),
    }
}

/// A UTC time as pandas' `strftime("%Y-%m-%dT%H:%M:%SZ")` writes it.
fn strftime_z(d: &DateTime<FixedOffset>) -> String {
    d.with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// A UTC time as pandas' `Timestamp.isoformat` writes it: fractional
/// seconds only when there are any, with nine digits when the time has
/// nanoseconds.
pub(super) fn isoformat(d: &DateTime<FixedOffset>) -> String {
    let d = d.with_timezone(&Utc);
    let nanos = d.nanosecond() % 1_000_000_000;
    let fraction = if !nanos.is_multiple_of(1_000) {
        format!(".{nanos:09}")
    } else if nanos != 0 {
        format!(".{:06}", nanos / 1_000)
    } else {
        String::new()
    };
    format!("{}{fraction}+00:00", d.format("%Y-%m-%dT%H:%M:%S"))
}

/// A JSON value with ordered keys, written as Python's `json.dump` does.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// An attribute value as Python holds it: a container is a dict and a
    /// list a list of its values.
    fn from_value(value: &AttributeValue) -> Json {
        match value.plain() {
            AttributeValue::String(s) | AttributeValue::Id(s) => Json::Str(s.to_string()),
            AttributeValue::Int(i) => Json::Int(*i),
            AttributeValue::Float(f) => Json::Float(*f),
            AttributeValue::Bool(b) => Json::Bool(*b),
            AttributeValue::Date(d) => Json::Str(isoformat(d)),
            AttributeValue::List(items) => {
                Json::Arr(items.iter().map(|(_, v)| Json::from_value(v)).collect())
            }
            AttributeValue::Container(c) => Json::Obj(
                c.iter()
                    .map(|(k, v)| (k.to_string(), Json::from_value(v)))
                    .collect(),
            ),
            AttributeValue::Meta(_) => unreachable!("plain() strips Meta"),
        }
    }

    /// Sets a key as a Python dict does: a known key keeps its position.
    fn set(&mut self, key: &str, value: Json) {
        let Json::Obj(entries) = self else {
            unreachable!("set on a JSON object")
        };
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => entries.push((key.to_string(), value)),
        }
    }

    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn field(&self, key: &str) -> &Json {
        self.get(key).expect("the key is set")
    }

    fn get_mut<'a>(entries: &'a mut [(String, Json)], key: &str) -> &'a mut Json {
        &mut entries
            .iter_mut()
            .find(|(k, _)| k == key)
            .expect("relations refer to kept rows")
            .1
    }

    /// Appends to the list at `key`, creating it at the end when absent.
    fn push_to(&mut self, key: &str, value: Json) {
        let Json::Obj(entries) = self else {
            unreachable!("push_to on a JSON object")
        };
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some((_, Json::Arr(list))) => list.push(value),
            _ => entries.push((key.to_string(), Json::Arr(vec![value]))),
        }
    }

    fn dump(&self) -> String {
        let mut out = String::new();
        self.write(0, &mut out);
        out
    }

    fn write(&self, level: usize, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Int(i) => out.push_str(&i.to_string()),
            Json::Float(f) if f.is_nan() => out.push_str("NaN"),
            Json::Float(f) if f.is_infinite() => {
                out.push_str(if *f > 0.0 { "Infinity" } else { "-Infinity" })
            }
            Json::Float(f) => out.push_str(&AttributeValue::Float(*f).to_string()),
            Json::Str(s) => json_string(s, out),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Obj(entries) if entries.is_empty() => out.push_str("{}"),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    out.push_str(if i == 0 { "\n" } else { ",\n" });
                    indent(level + 1, out);
                    item.write(level + 1, out);
                }
                out.push('\n');
                indent(level, out);
                out.push(']');
            }
            Json::Obj(entries) => {
                out.push('{');
                for (i, (key, value)) in entries.iter().enumerate() {
                    out.push_str(if i == 0 { "\n" } else { ",\n" });
                    indent(level + 1, out);
                    json_string(key, out);
                    out.push_str(": ");
                    value.write(level + 1, out);
                }
                out.push('\n');
                indent(level, out);
                out.push('}');
            }
        }
    }
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

/// A JSON string with Python's `ensure_ascii` escapes.
fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// An XML element, written as lxml's pretty printer writes it.
struct El {
    name: &'static str,
    attrs: Vec<(&'static str, String)>,
    text: Option<String>,
    children: Vec<El>,
}

impl El {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            attrs: Vec::new(),
            text: None,
            children: Vec::new(),
        }
    }

    fn attr(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.attrs.push((key, value.into()));
        self
    }

    fn text(mut self, text: String) -> Self {
        self.text = Some(text);
        self
    }

    fn child(mut self, child: El) -> Self {
        self.children.push(child);
        self
    }

    fn push(&mut self, child: El) {
        self.children.push(child);
    }

    /// The document with lxml's declaration.
    fn document(&self) -> Result<String> {
        let mut out = String::from("<?xml version='1.0' encoding='UTF-8'?>\n");
        self.write(0, &mut out)?;
        Ok(out)
    }

    fn write(&self, level: usize, out: &mut String) -> Result<()> {
        indent(level, out);
        out.push('<');
        out.push_str(self.name);
        for (key, value) in &self.attrs {
            out.push(' ');
            out.push_str(key);
            out.push_str("=\"");
            xml_escape(value, true, out)?;
            out.push('"');
        }
        match (&self.text, self.children.is_empty()) {
            (None, true) => out.push_str("/>\n"),
            (Some(text), true) => {
                out.push('>');
                xml_escape(text, false, out)?;
                out.push_str("</");
                out.push_str(self.name);
                out.push_str(">\n");
            }
            _ => {
                out.push_str(">\n");
                for child in &self.children {
                    child.write(level + 1, out)?;
                }
                indent(level, out);
                out.push_str("</");
                out.push_str(self.name);
                out.push_str(">\n");
            }
        }
        Ok(())
    }
}

/// Escapes text as libxml2 does, for an attribute value or element text.
/// Like lxml, it refuses the control characters, U+FFFE and U+FFFF, which
/// XML 1.0 cannot hold.
fn xml_escape(s: &str, attribute: bool, out: &mut String) -> Result<()> {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\n' if attribute => out.push_str("&#10;"),
            '\t' if attribute => out.push_str("&#9;"),
            '\r' => out.push_str("&#13;"),
            '\n' | '\t' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {
                return Err(Error::Ocel(format!(
                    "XML cannot hold the character U+{:04X}",
                    c as u32
                )));
            }
            c => out.push(c),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_strings_escape_like_python() {
        let mut out = String::new();
        json_string("a\"\\\n\u{1}\u{7f}é😀", &mut out);
        assert_eq!(out, r#""a\"\\\n\u0001\u007f\u00e9\ud83d\ude00""#);
    }

    #[test]
    fn json_layout_follows_python() {
        let json = Json::Obj(vec![
            ("a".into(), Json::Arr(vec![Json::Int(1), Json::Float(2.0)])),
            ("b".into(), Json::Obj(Vec::new())),
            ("c".into(), Json::Float(1e16)),
        ]);
        assert_eq!(
            json.dump(),
            "{\n  \"a\": [\n    1,\n    2.0\n  ],\n  \"b\": {},\n  \"c\": 1e+16\n}"
        );
    }

    #[test]
    fn xml_refuses_control_characters() {
        for c in ['\u{1}', '\u{FFFE}', '\u{FFFF}'] {
            assert!(El::new("a").attr("k", c).document().is_err());
            assert!(El::new("a").text(c.to_string()).document().is_err());
        }
        assert!(El::new("a").text("\u{FFFD}".into()).document().is_ok());
    }

    #[test]
    fn times_follow_pandas() {
        let d = DateTime::parse_from_rfc3339("2022-01-09T15:00:00.5+01:00").unwrap();
        assert_eq!(isoformat(&d), "2022-01-09T14:00:00.500000+00:00");
        assert_eq!(strftime_z(&d), "2022-01-09T14:00:00Z");
        let d = DateTime::parse_from_rfc3339("2022-01-09T14:00:00.000000001Z").unwrap();
        assert_eq!(isoformat(&d), "2022-01-09T14:00:00.000000001+00:00");
    }
}
