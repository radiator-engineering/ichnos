//! OCEL 2.0 as CSV (pm4py's `read_ocel2_csv` and `write_ocel2_csv`): one
//! table whose rows are events, object declarations, object-to-object
//! relations and object attribute changes.
//!
//! The columns are `id`, `activity`, `timestamp`, one `ot:<type>` column per
//! object type and one column per event attribute. A cell of an `ot:`
//! column holds references separated by `/`, each an object id, then
//! optionally `#` and a qualifier, then optionally a JSON object of
//! attribute values. `\` escapes `/`, `#`, `{` and `\` in ids and
//! qualifiers.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, TimeZone};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::{EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject};
use serde_json::Value;

use super::csv::{parse_timestamp, write_record};
use super::json::{Ordered, value as json_value};
use super::write::{Columns, Kind, as_float, isoformat, missing, normalized_text, to_path};
use super::{lower_name, open};
use crate::error::{Error, Result};

const ID: &str = "id";
const ACTIVITY: &str = "activity";
const TIMESTAMP: &str = "timestamp";
const TYPE_PREFIX: &str = "ot:";
const O2O: &str = "o2o";
/// Python's `csv.field_size_limit()` default, in characters.
const FIELD_LIMIT: usize = 131_072;

/// An object id and an attribute name.
type ObjectAttribute = (Arc<str>, Arc<str>);
/// An attribute change: object, time, attribute and value.
type Change = (Arc<str>, DateTime<FixedOffset>, Arc<str>, Raw);
/// A parsed reference: object id, qualifier and attribute values.
type Reference = (Arc<str>, Option<Arc<str>>, Vec<(Arc<str>, Raw)>);

fn error(detail: impl Into<String>) -> Error {
    Error::Ocel(detail.into())
}

/// Reads an OCEL 2.0 CSV file (pm4py's `read_ocel2_csv`). A path ending in
/// `.gz` is decompressed.
///
/// See [`read_ocel2_csv_from_reader`] for how the file is read.
pub fn read_ocel2_csv(path: impl AsRef<Path>) -> Result<Ocel> {
    read_ocel2_csv_from_reader(open(path.as_ref())?)
}

/// Reads an OCEL 2.0 CSV table (pm4py's `read_ocel2_csv`).
///
/// The text must be UTF-8 CSV as Python's `csv` module reads it in strict
/// mode: every record has as many fields as the header, a quote may open a
/// field only at its start and must close it before a comma or a line
/// break, and no column name repeats. The columns `id`, `activity` and
/// `timestamp` must be there; their values lose surrounding white space.
/// Each row is one of these:
///
/// - an event: `id`, `activity` and `timestamp` are set. Its references
///   relate the event to objects, and their JSON values change the objects'
///   attributes at the event's time.
/// - an object-to-object row: `activity` is `o2o` in any case and `id`
///   names an object declared in an earlier row. Its references are the
///   targets. JSON values need a `timestamp`.
/// - an attribute change: only `timestamp` is set, and each reference
///   carries JSON values.
/// - a declaration: none of the three is set. Its references declare
///   objects, and their JSON values are the objects' initial attributes.
///
/// Only event rows may hold event attribute values. A timestamp must be an
/// ISO 8601 date and time with an offset, and is read as UTC. An object
/// keeps the first type it appears with; another type is an error. Two
/// different values of one attribute of one object at one time are an
/// error. Values at no time, or at the Unix epoch, are the object's
/// attributes; others are [`Ocel::object_changes`].
///
/// Event attribute values, and the values of each object attribute per
/// object type, are then typed together: as integers when all are
/// integers in canonical form, else as floats when all are floats in
/// Python's shortest form, else as booleans (`true` or `false` in any
/// case), else as times with an offset, else as text.
///
/// The events, relations and changes are then sorted by timestamp, keeping
/// file order for ties, and [`Ocel::make_consistent`] runs. Events and
/// objects without relations stay.
pub fn read_ocel2_csv_from_reader(mut input: impl Read) -> Result<Ocel> {
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes)?;
    let text = String::from_utf8(bytes).map_err(|_| error("the CSV file is not UTF-8"))?;
    let mut records = records(&text)?.into_iter();
    let header = records
        .next()
        .ok_or_else(|| error("an OCEL 2.0 CSV file must have a header row"))?;
    let mut names = HashSet::new();
    if !header.iter().all(|name| names.insert(name.as_str())) {
        return Err(error("OCEL 2.0 CSV column names must be unique"));
    }
    let rows: Vec<Vec<String>> = records.collect();
    for (i, row) in rows.iter().enumerate() {
        if row.len() != header.len() {
            return Err(error(format!(
                "CSV row {} has {} fields; expected {}",
                i + 2,
                row.len(),
                header.len()
            )));
        }
    }
    let column = |name: &str| {
        header
            .iter()
            .position(|c| c == name)
            .ok_or_else(|| error(format!("the OCEL 2.0 CSV file has no {name} column")))
    };
    let (id_c, activity_c, ts_c) = (column(ID)?, column(ACTIVITY)?, column(TIMESTAMP)?);
    let mut types: Vec<(usize, Arc<str>)> = Vec::new();
    let mut attribute_columns: Vec<(usize, Arc<str>)> = Vec::new();
    for (i, name) in header.iter().enumerate() {
        if let Some(object_type) = name.strip_prefix(TYPE_PREFIX) {
            if object_type.is_empty() {
                return Err(error("an OCEL 2.0 CSV object type column must name a type"));
            }
            types.push((i, object_type.into()));
        } else if ![id_c, activity_c, ts_c].contains(&i) {
            attribute_columns.push((i, name.as_str().into()));
        }
    }
    let times = row_times(&rows, ts_c)?;

    let mut reader = Reader::default();
    for (r, row) in rows.iter().enumerate() {
        reader.row(
            r,
            row,
            times[r],
            [id_c, activity_c, ts_c],
            &types,
            &attribute_columns,
        )?;
    }
    reader.finish(&attribute_columns)
}

/// Each row's stripped timestamp, parsed when present.
fn row_times(rows: &[Vec<String>], ts: usize) -> Result<Vec<Option<DateTime<FixedOffset>>>> {
    rows.iter()
        .map(|row| {
            let text = py_strip(&row[ts]);
            if text.is_empty() {
                return Ok(None);
            }
            parse_time(text)
                .map(Some)
                .ok_or_else(|| error(format!("cannot read the timestamp {text:?}")))
        })
        .collect()
}

/// A value as read, before typing.
#[derive(Debug, Clone, PartialEq)]
enum Raw {
    Missing,
    Str(Arc<str>),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl Raw {
    fn from_json(v: &Value) -> Result<Self> {
        Ok(match json_value(v) {
            None => Raw::Missing,
            Some(AttributeValue::String(s)) => Raw::Str(s),
            Some(AttributeValue::Int(i)) => Raw::Int(i),
            Some(AttributeValue::Float(f)) if f.is_finite() => Raw::Float(f),
            Some(AttributeValue::Bool(b)) => Raw::Bool(b),
            _ => {
                return Err(error(
                    "JSON arrays, objects and non-finite numbers are not valid OCEL 2.0 CSV attribute values",
                ));
            }
        })
    }

    /// pm4py's `_is_empty`: no value or the empty string.
    fn is_empty(&self) -> bool {
        match self {
            Raw::Missing => true,
            Raw::Str(s) => s.is_empty(),
            _ => false,
        }
    }
}

/// One assignment of an object attribute: its row, its time (none for a
/// declaration) and its value.
type Assignment = (usize, Option<DateTime<FixedOffset>>, Raw);

#[derive(Default)]
struct Reader {
    events: Vec<OcelEvent>,
    /// Per event, its raw attribute values by attribute column.
    event_values: Vec<Vec<Raw>>,
    relations: Vec<(DateTime<FixedOffset>, EventObject)>,
    o2o: Vec<ObjectObject>,
    /// The objects in order of first reference, with their types.
    objects: Vec<(Arc<str>, Arc<str>)>,
    object_types: HashMap<Arc<str>, Arc<str>>,
    /// Objects referenced in a row before the current one.
    declared: HashSet<Arc<str>>,
    /// Assignments per object and attribute, in order of first assignment.
    assignments: Vec<(ObjectAttribute, Vec<Assignment>)>,
    assignment_index: HashMap<ObjectAttribute, usize>,
    event_ids: HashSet<Arc<str>>,
    relation_keys: HashSet<(Arc<str>, Arc<str>, Arc<str>)>,
    o2o_keys: HashSet<(Arc<str>, Arc<str>, Arc<str>)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Event,
    O2o,
    Change,
    Declaration,
}

impl Reader {
    fn row(
        &mut self,
        r: usize,
        row: &[String],
        time: Option<DateTime<FixedOffset>>,
        [id_c, activity_c, ts_c]: [usize; 3],
        types: &[(usize, Arc<str>)],
        attribute_columns: &[(usize, Arc<str>)],
    ) -> Result<()> {
        let line = r + 2;
        let id = py_strip(&row[id_c]);
        let activity = py_strip(&row[activity_c]);
        let has_ts = !py_strip(&row[ts_c]).is_empty();
        let kind = if !activity.is_empty() && fold(activity) == O2O {
            if id.is_empty() {
                return Err(error("object-to-object rows must have a source object id"));
            }
            RowKind::O2o
        } else {
            match (!id.is_empty(), !activity.is_empty(), has_ts) {
                (true, true, true) => RowKind::Event,
                (false, false, true) => RowKind::Change,
                (false, false, false) => RowKind::Declaration,
                _ => {
                    return Err(error(format!(
                        "OCEL 2.0 CSV row {line} has an unsupported mix of id, activity and timestamp"
                    )));
                }
            }
        };
        if kind != RowKind::Event && attribute_columns.iter().any(|(i, _)| !row[*i].is_empty()) {
            return Err(error(format!(
                "event attribute values are only valid in event rows (CSV row {line})"
            )));
        }
        let id: Arc<str> = id.into();
        if kind == RowKind::Event {
            if !self.event_ids.insert(id.clone()) {
                return Err(error(format!("duplicate event id {id:?}")));
            }
            self.events.push(OcelEvent {
                id: id.clone(),
                activity: activity.into(),
                timestamp: time.expect("an event row has a timestamp"),
                attributes: Attributes::default(),
            });
            self.event_values.push(
                attribute_columns
                    .iter()
                    .map(|(i, _)| match row[*i].as_str() {
                        "" => Raw::Missing,
                        value => Raw::Str(value.into()),
                    })
                    .collect(),
            );
        }
        if kind == RowKind::O2o && !self.declared.contains(&id) {
            return Err(error(format!(
                "the source object {id:?} of an object-to-object row has no earlier declaration"
            )));
        }
        let mut references = 0;
        let mut declared = Vec::new();
        for (i, object_type) in types {
            for entry in split_entries(&row[*i])? {
                references += 1;
                let (object, qualifier, attributes) = split_reference(&entry)?;
                self.register(&object, object_type)?;
                declared.push(object.clone());
                let qualifier_key: Arc<str> = qualifier.clone().unwrap_or_else(|| "".into());
                let at = match kind {
                    RowKind::Event => {
                        let key = (id.clone(), object.clone(), qualifier_key.clone());
                        if !self.relation_keys.insert(key) {
                            return Err(error("duplicate event-to-object relation"));
                        }
                        self.relations.push((
                            time.expect("an event row has a timestamp"),
                            EventObject {
                                event: id.clone(),
                                object: object.clone(),
                                qualifier: Some(qualifier_key),
                            },
                        ));
                        time
                    }
                    RowKind::O2o => {
                        let key = (id.clone(), object.clone(), qualifier_key.clone());
                        if !self.o2o_keys.insert(key) {
                            return Err(error("duplicate object-to-object relation"));
                        }
                        self.o2o.push(ObjectObject {
                            source: id.clone(),
                            target: object.clone(),
                            qualifier: Some(qualifier_key),
                        });
                        if !attributes.is_empty() && time.is_none() {
                            return Err(error(
                                "object-to-object rows with JSON attributes must have a timestamp",
                            ));
                        }
                        time
                    }
                    RowKind::Change => {
                        if attributes.is_empty() {
                            return Err(error("object attribute rows must have JSON attributes"));
                        }
                        time
                    }
                    RowKind::Declaration => {
                        if qualifier.is_some() {
                            return Err(error("object declaration rows cannot have qualifiers"));
                        }
                        None
                    }
                };
                for (name, value) in attributes {
                    self.assign(&object, name, (r, at, value));
                }
            }
        }
        if references == 0 && kind == RowKind::Change {
            return Err(error("object attribute rows must have an object reference"));
        }
        if references == 0 && kind == RowKind::O2o {
            return Err(error(
                "object-to-object rows must have a target object reference",
            ));
        }
        self.declared.extend(declared);
        Ok(())
    }

    fn register(&mut self, object: &Arc<str>, object_type: &Arc<str>) -> Result<()> {
        match self.object_types.get(object) {
            Some(known) if known != object_type => Err(error(format!(
                "the object {object:?} appears with the types {known:?} and {object_type:?}"
            ))),
            Some(_) => Ok(()),
            None => {
                self.object_types
                    .insert(object.clone(), object_type.clone());
                self.objects.push((object.clone(), object_type.clone()));
                Ok(())
            }
        }
    }

    fn assign(&mut self, object: &Arc<str>, name: Arc<str>, assignment: Assignment) {
        let key = (object.clone(), name);
        let i = *self.assignment_index.entry(key.clone()).or_insert_with(|| {
            self.assignments.push((key, Vec::new()));
            self.assignments.len() - 1
        });
        self.assignments[i].1.push(assignment);
    }

    fn finish(self, attribute_columns: &[(usize, Arc<str>)]) -> Result<Ocel> {
        let epoch = DateTime::UNIX_EPOCH.fixed_offset();
        // The objects' initial values and the changes, as pm4py's two
        // tables hold them before typing.
        let mut initial: HashMap<Arc<str>, Vec<(Arc<str>, Raw)>> = HashMap::new();
        let mut changes: Vec<Change> = Vec::new();
        for ((object, name), mut values) in self.assignments {
            values.sort_by_key(|(_, at, _)| (at.is_some(), *at));
            let mut seen: Vec<(DateTime<FixedOffset>, Raw)> = Vec::new();
            for (_, at, value) in values {
                let at = at.unwrap_or(epoch);
                if let Some((_, first)) = seen.iter().find(|(t, _)| *t == at) {
                    if !same_value(first, &value) {
                        return Err(error(format!(
                            "the attribute {name:?} of the object {object:?} has different values at {}",
                            isoformat(&at)
                        )));
                    }
                    continue;
                }
                seen.push((at, value.clone()));
                if at == epoch {
                    initial
                        .entry(object.clone())
                        .or_default()
                        .push((name.clone(), value));
                } else {
                    changes.push((object.clone(), at, name.clone(), value));
                }
            }
        }

        // pandas gives a column of integers with a gap float64, and the
        // typing below then sees floats.
        let mut initial_columns: HashMap<&str, Vec<&Raw>> = HashMap::new();
        for values in initial.values() {
            for (name, value) in values {
                initial_columns.entry(name).or_default().push(value);
            }
        }
        let initial_float: HashSet<&str> = initial_columns
            .iter()
            .filter(|(_, values)| float64(values, values.len() < self.objects.len()))
            .map(|(name, _)| *name)
            .collect();
        let mut change_columns: HashMap<&str, Vec<&Raw>> = HashMap::new();
        for (_, _, name, value) in &changes {
            change_columns.entry(name).or_default().push(value);
        }
        let change_float: HashSet<&str> = change_columns
            .iter()
            .filter(|(_, values)| float64(values, values.len() < changes.len()))
            .map(|(name, _)| *name)
            .collect();
        let as_table = |value: &Raw, float: bool| match value {
            Raw::Int(i) if float => Raw::Float(*i as f64),
            Raw::Missing => Raw::Missing,
            other => other.clone(),
        };

        // Type each attribute per object type, over initial values and
        // changes together.
        let mut by_type: HashMap<(&str, &str), Vec<Raw>> = HashMap::new();
        for (object, object_type) in &self.objects {
            for (name, value) in initial.get(object).into_iter().flatten() {
                let value = as_table(value, initial_float.contains(&**name));
                by_type.entry((object_type, name)).or_default().push(value);
            }
        }
        for (object, _, name, value) in &changes {
            let value = as_table(value, change_float.contains(&**name));
            by_type
                .entry((&self.object_types[object], name))
                .or_default()
                .push(value);
        }
        let typing: HashMap<(&str, &str), Typing> = by_type
            .iter()
            .map(|(key, values)| (*key, Typing::of(values.iter())))
            .collect();

        let mut ocel = Ocel::new();
        ocel.objects = self
            .objects
            .iter()
            .map(|(object, object_type)| OcelObject {
                id: object.clone(),
                object_type: object_type.clone(),
                attributes: initial
                    .get(object)
                    .into_iter()
                    .flatten()
                    .filter_map(|(name, value)| {
                        let value = as_table(value, initial_float.contains(&**name));
                        let typing = typing[&(&**object_type, &**name)];
                        Some((name.clone(), typing.apply(&value)?))
                    })
                    .collect(),
            })
            .collect();
        let mut object_changes: Vec<ObjectChange> = changes
            .iter()
            .map(|(object, at, name, value)| {
                let object_type = self.object_types[object].clone();
                let value = as_table(value, change_float.contains(&**name));
                let value = typing[&(&*object_type, &**name)].apply(&value);
                ObjectChange {
                    object: object.clone(),
                    object_type,
                    timestamp: *at,
                    field: name.clone(),
                    value,
                }
            })
            .collect();
        object_changes.sort_by_key(|c| c.timestamp);
        ocel.object_changes = object_changes;

        let mut events = self.events;
        for (c, (_, name)) in attribute_columns.iter().enumerate() {
            let typing = Typing::of(self.event_values.iter().map(|values| &values[c]));
            for (event, values) in events.iter_mut().zip(&self.event_values) {
                if let Some(value) = typing.apply(&values[c]) {
                    event.attributes.insert(name.clone(), value);
                }
            }
        }
        events.sort_by_key(|e| e.timestamp);
        ocel.events = events;
        let mut relations = self.relations;
        relations.sort_by_key(|(t, _)| *t);
        ocel.relations = relations.into_iter().map(|(_, r)| r).collect();
        ocel.o2o = self.o2o;
        ocel.make_consistent();
        Ok(ocel)
    }
}

/// Whether pandas makes a column of these values `float64`: all numbers,
/// with a float or a gap among them.
fn float64(values: &[&Raw], gap: bool) -> bool {
    let present: Vec<&&Raw> = values.iter().filter(|v| **v != &Raw::Missing).collect();
    !present.is_empty()
        && present
            .iter()
            .all(|v| matches!(v, Raw::Int(_) | Raw::Float(_)))
        && (gap
            || present.len() < values.len()
            || present.iter().any(|v| matches!(v, Raw::Float(_))))
}

/// How pm4py's `_infer_values` types one column.
#[derive(Debug, Clone, Copy)]
enum Typing {
    Int,
    Float,
    Bool,
    Time,
    Text,
}

impl Typing {
    fn of<'a>(values: impl Iterator<Item = &'a Raw> + Clone) -> Self {
        let mut present = values.filter(|v| !v.is_empty());
        if present.clone().all(|v| as_int(v).is_some()) {
            Typing::Int
        } else if present.clone().all(|v| as_float_raw(v).is_some()) {
            Typing::Float
        } else if present.clone().all(|v| as_bool(v).is_some()) {
            Typing::Bool
        } else if present.all(|v| matches!(v, Raw::Str(s) if parse_time(s).is_some())) {
            Typing::Time
        } else {
            Typing::Text
        }
    }

    /// The typed value. The empty string stays as it is.
    fn apply(self, value: &Raw) -> Option<AttributeValue> {
        if let Raw::Str(s) = value
            && s.is_empty()
        {
            return Some(AttributeValue::String(s.clone()));
        }
        Some(match (self, value) {
            (_, Raw::Missing) => return None,
            (Typing::Int, v) => AttributeValue::Int(as_int(v)?),
            (Typing::Float, v) => AttributeValue::Float(as_float_raw(v)?),
            (Typing::Bool, v) => AttributeValue::Bool(as_bool(v)?),
            (Typing::Time, Raw::Str(s)) => AttributeValue::Date(parse_time(s)?),
            (_, Raw::Str(s)) => AttributeValue::String(s.clone()),
            (_, Raw::Int(i)) => AttributeValue::String(i.to_string().into()),
            (_, Raw::Float(f)) => {
                AttributeValue::String(AttributeValue::Float(*f).to_string().into())
            }
            (_, Raw::Bool(b)) => {
                AttributeValue::String(AttributeValue::Bool(*b).to_string().into())
            }
        })
    }
}

fn as_int(value: &Raw) -> Option<i64> {
    match value {
        Raw::Int(i) => Some(*i),
        Raw::Str(s) => canonical_int(s),
        _ => None,
    }
}

fn as_float_raw(value: &Raw) -> Option<f64> {
    match value {
        Raw::Int(i) => Some(*i as f64),
        Raw::Float(f) => Some(*f).filter(|f| f.is_finite()),
        Raw::Str(s) => canonical_float(s),
        _ => None,
    }
}

fn as_bool(value: &Raw) -> Option<bool> {
    match value {
        Raw::Bool(b) => Some(*b),
        Raw::Str(s) => match fold(s).as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// pm4py's `_parse_canonical_integer`: `-?(0|[1-9][0-9]*)` within 64 bits.
/// Python's `$` also lets a final line break follow.
fn canonical_int(text: &str) -> Option<i64> {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let digits = text.strip_prefix('-').unwrap_or(text);
    let canonical = digits == "0"
        || (!digits.is_empty()
            && !digits.starts_with('0')
            && digits.bytes().all(|b| b.is_ascii_digit()));
    if canonical { text.parse().ok() } else { None }
}

/// pm4py's `_parse_canonical_float`: a finite float written as Python's
/// `repr` writes it, or a canonical integer a float holds exactly.
fn canonical_float(text: &str) -> Option<f64> {
    if let Ok(f) = text.parse::<f64>()
        && f.is_finite()
        && AttributeValue::Float(f).to_string() == text
    {
        return Some(f);
    }
    let i = canonical_int(text)?;
    let f = i as f64;
    (f as i128 == i128::from(i)).then_some(f)
}

/// Python's `casefold` for the words it is compared with here: lower case,
/// with the long s folded to `s`.
fn fold(text: &str) -> String {
    text.to_lowercase().replace('ſ', "s")
}

/// Python's `str.strip()`: white space and the separators U+001C to U+001F.
fn py_strip(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c))
}

/// An ISO 8601 date and time with an offset, as UTC.
pub(super) fn parse_time(text: &str) -> Option<DateTime<FixedOffset>> {
    let (local, form) = parse_timestamp(text)?;
    form.separator?;
    let offset = FixedOffset::east_opt(form.offset?)?;
    let utc = FixedOffset::east_opt(0)?;
    Some(
        offset
            .from_local_datetime(&local)
            .single()?
            .with_timezone(&utc),
    )
}

/// Whether two values are equal as pm4py's `_attribute_value_key` compares
/// them: numbers by decimal value, others by type and value.
fn same_value(a: &Raw, b: &Raw) -> bool {
    match (a, b) {
        (Raw::Missing, Raw::Missing) => true,
        (Raw::Bool(x), Raw::Bool(y)) => x == y,
        (Raw::Str(x), Raw::Str(y)) => x == y,
        (Raw::Int(_) | Raw::Float(_), Raw::Int(_) | Raw::Float(_)) => {
            decimal(&raw_number(a)) == decimal(&raw_number(b))
        }
        _ => false,
    }
}

fn raw_number(value: &Raw) -> String {
    match value {
        Raw::Int(i) => i.to_string(),
        Raw::Float(f) => AttributeValue::Float(*f).to_string(),
        _ => String::new(),
    }
}

/// Python's `Decimal(text)` as a comparable key: sign, significant digits
/// and exponent.
pub(super) fn decimal(text: &str) -> (bool, String, i64) {
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i64>().unwrap_or(0)),
        None => (text, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    let digits = digits.trim_start_matches('0');
    let significant = digits.trim_end_matches('0');
    if significant.is_empty() {
        return (false, String::new(), 0);
    }
    let exponent = exponent - fraction.len() as i64 + (digits.len() - significant.len()) as i64;
    (negative, significant.to_string(), exponent)
}

/// pm4py's `_split_entries`: the references of a cell, split at each `/`
/// outside an escape and outside JSON.
fn split_entries(cell: &str) -> Result<Vec<String>> {
    if cell.is_empty() {
        return Ok(Vec::new());
    }
    if !cell.contains(['/', '{', '\\']) {
        return Ok(vec![cell.to_string()]);
    }
    let empty = || error("object reference cells cannot have empty references");
    let mut entries = Vec::new();
    let mut current = String::new();
    let (mut in_string, mut escape, mut depth) = (false, false, 0usize);
    for c in cell.chars() {
        if escape {
            current.push(c);
            escape = false;
            continue;
        }
        match c {
            '\\' => {
                current.push(c);
                escape = depth == 0 || in_string;
                continue;
            }
            '"' => {
                current.push(c);
                if depth > 0 {
                    in_string = !in_string;
                }
                continue;
            }
            '{' if !in_string => depth += 1,
            '}' if !in_string && depth > 0 => depth -= 1,
            '/' if !in_string && depth == 0 => {
                if current.is_empty() {
                    return Err(empty());
                }
                entries.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if escape {
        return Err(error(
            "object reference cells cannot end with a dangling escape character",
        ));
    }
    if current.is_empty() {
        return Err(empty());
    }
    entries.push(current);
    Ok(entries)
}

/// The byte position of the first `target` outside an escape.
fn find_unescaped(text: &str, target: char) -> Option<usize> {
    let mut escape = false;
    for (i, c) in text.char_indices() {
        if escape {
            escape = false;
        } else if c == '\\' {
            escape = true;
        } else if c == target {
            return Some(i);
        }
    }
    None
}

fn unescape(text: &str, what: &str) -> Result<String> {
    let mut out = String::new();
    let mut escape = false;
    for c in text.chars() {
        if escape {
            if !matches!(c, '/' | '#' | '{' | '\\') {
                return Err(error(format!(
                    "invalid escape sequence \\{c} in an OCEL 2.0 CSV {what}"
                )));
            }
            out.push(c);
            escape = false;
        } else if c == '\\' {
            escape = true;
        } else if matches!(c, '/' | '#' | '{') {
            return Err(error(format!(
                "unescaped {c} in an OCEL 2.0 CSV {what}; escape it as \\{c}"
            )));
        } else {
            out.push(c);
        }
    }
    if escape {
        return Err(error(format!(
            "dangling escape character in an OCEL 2.0 CSV {what}"
        )));
    }
    Ok(out)
}

/// pm4py's `_split_reference`: object id, qualifier and JSON values.
fn split_reference(entry: &str) -> Result<Reference> {
    let mut text = py_strip(entry);
    let mut attributes = Vec::new();
    if let Some(start) = find_unescaped(text, '{') {
        let map: Ordered<Value> = serde_json::from_str(&text[start..])
            .map_err(|e| error(format!("invalid JSON in an object reference: {e}")))?;
        for (name, value) in map.0 {
            attributes.push((Arc::from(name), Raw::from_json(&value)?));
        }
        text = &text[..start];
    }
    let (object, qualifier) = match find_unescaped(text, '#') {
        Some(i) => (&text[..i], Some(&text[i + 1..])),
        None => (text, None),
    };
    let object = unescape(object, "object id")?;
    let object = py_strip(&object);
    if object.is_empty() {
        return Err(error("object references must have a non-empty object id"));
    }
    let qualifier = match qualifier {
        Some(q) => Some(Arc::from(py_strip(&unescape(q, "qualifier")?))),
        None => None,
    };
    Ok((object.into(), qualifier, attributes))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    StartRecord,
    StartField,
    InField,
    InQuoted,
    QuoteInQuoted,
    EatCrnl,
}

/// The records of a CSV text as Python's `csv.reader` gives them in strict
/// mode, with the file opened with `newline=''`. A blank line is a record
/// without fields.
pub(super) fn records(text: &str) -> Result<Vec<Vec<String>>> {
    let mut records = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut length = 0;
    let mut state = State::StartRecord;
    for line in lines(text) {
        // `None` is the end of the line.
        for c in line.chars().map(Some).chain([None]) {
            let mut save = |field: &mut String, length: &mut usize| {
                fields.push(std::mem::take(field));
                *length = 0;
            };
            let end = |eol: bool| {
                if eol {
                    State::StartRecord
                } else {
                    State::EatCrnl
                }
            };
            let add = |c: char, field: &mut String, length: &mut usize| {
                if *length >= FIELD_LIMIT {
                    return Err(error(format!(
                        "field larger than field limit ({FIELD_LIMIT})"
                    )));
                }
                field.push(c);
                *length += 1;
                Ok(())
            };
            if state == State::StartRecord {
                state = match c {
                    None => State::StartRecord,
                    Some('\n' | '\r') => State::EatCrnl,
                    Some(_) => State::StartField,
                };
                if state != State::StartField {
                    continue;
                }
            }
            state = match (state, c) {
                (
                    State::StartField | State::InField | State::QuoteInQuoted,
                    None | Some('\n' | '\r'),
                ) => {
                    save(&mut field, &mut length);
                    end(c.is_none())
                }
                (State::StartField, Some('"')) => State::InQuoted,
                (State::StartField | State::InField | State::QuoteInQuoted, Some(',')) => {
                    save(&mut field, &mut length);
                    State::StartField
                }
                (State::StartField | State::InField, Some(c)) => {
                    add(c, &mut field, &mut length)?;
                    State::InField
                }
                (State::InQuoted, None) => State::InQuoted,
                (State::InQuoted, Some('"')) => State::QuoteInQuoted,
                (State::InQuoted, Some(c)) => {
                    add(c, &mut field, &mut length)?;
                    State::InQuoted
                }
                (State::QuoteInQuoted, Some('"')) => {
                    add('"', &mut field, &mut length)?;
                    State::InQuoted
                }
                (State::QuoteInQuoted, Some(c)) => {
                    return Err(error(format!("',' expected after '\"', found {c:?}")));
                }
                (State::EatCrnl, Some('\n' | '\r')) => State::EatCrnl,
                (State::EatCrnl, None) => State::StartRecord,
                (State::EatCrnl, Some(_)) => {
                    return Err(error("new-line character seen in unquoted field"));
                }
                (State::StartRecord, _) => unreachable!("handled above"),
            };
        }
        if state == State::StartRecord {
            records.push(std::mem::take(&mut fields));
        }
    }
    if state == State::InQuoted {
        return Err(error("unexpected end of data inside a quoted field"));
    }
    Ok(records)
}

/// The lines of a text, each with its `\r\n`, `\r` or `\n`.
fn lines(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    let mut lines = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < b.len() {
        let end = match b[i] {
            b'\n' => i + 1,
            b'\r' if b.get(i + 1) == Some(&b'\n') => i + 2,
            b'\r' => i + 1,
            _ => {
                i += 1;
                continue;
            }
        };
        lines.push(&text[start..end]);
        start = end;
        i = end;
    }
    if start < b.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Writes an OCEL 2.0 CSV file (pm4py's `write_ocel2_csv`). The name must
/// end in `.ocel.csv`, in any case, as pm4py requires.
///
/// See [`write_ocel2_csv_to_writer`] for what is written. Nothing is
/// written when the log cannot be written.
pub fn write_ocel2_csv(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !lower_name(path).ends_with(".ocel.csv") {
        return Err(error(format!(
            "an OCEL 2.0 CSV file name must end in .ocel.csv: {}",
            path.display()
        )));
    }
    let text = render(ocel)?;
    to_path(path, |out| Ok(out.write_all(text.as_bytes())?))
}

/// Writes an OCEL 2.0 CSV table (pm4py's `write_ocel2_csv`).
///
/// The log is first made consistent. Events and objects without relations
/// stay. The rows are, in order:
///
/// - the events, by time, each with its references grouped by object type;
/// - a declaration of each object, by type and id, with its initial
///   attribute values as JSON, unless an event refers to it and it has no
///   initial values;
/// - one object-to-object row per source object, by id, with sorted
///   references;
/// - the changes at each time to each object, by time.
///
/// Times are written in UTC as pandas' `isoformat` writes them, and lines
/// end in `\r\n`. A change at the Unix epoch is written as an initial
/// value.
///
/// These are errors, as in pm4py: a repeated event or object id, a
/// relation to an unknown event or object, a repeated relation, an object
/// id or qualifier with surrounding white space, an event id or activity
/// that is empty or has surrounding white space, the activity `o2o`, a
/// change without a value or of an unknown object, two different values of
/// one attribute at one time, a list value, and an event attribute named
/// like a column of the format.
pub fn write_ocel2_csv_to_writer(ocel: &Ocel, mut output: impl Write) -> Result<()> {
    output.write_all(render(ocel)?.as_bytes())?;
    Ok(())
}

/// A present value as pm4py's `normalize_value` leaves it, with the
/// integers of a float column as floats.
fn normalized(value: &AttributeValue, kind: Kind) -> Option<AttributeValue> {
    if missing(value) {
        return None;
    }
    if kind == Kind::Float
        && let Some(f) = as_float(value)
    {
        return Some(AttributeValue::Float(f));
    }
    Some(value.plain().clone())
}

/// pm4py's `_values_equal` on normalized values.
fn values_equal(a: &AttributeValue, b: &AttributeValue) -> bool {
    let number = |v: &AttributeValue| match v {
        AttributeValue::Int(i) => Some(i.to_string()),
        AttributeValue::Float(f) => Some(AttributeValue::Float(*f).to_string()),
        _ => None,
    };
    match (number(a), number(b)) {
        (Some(x), Some(y)) => decimal(&x) == decimal(&y),
        _ => match (a, b) {
            (AttributeValue::Date(x), AttributeValue::Date(y)) => isoformat(x) == isoformat(y),
            _ => std::mem::discriminant(a) == std::mem::discriminant(b) && a == b,
        },
    }
}

/// pm4py's `_validate_reference_part`.
fn reference_part<'a>(text: &'a str, what: &str) -> Result<&'a str> {
    if text.is_empty() {
        return Err(error(format!("OCEL 2.0 CSV {what} values cannot be empty")));
    }
    if py_strip(text) != text {
        return Err(error(format!(
            "OCEL 2.0 CSV {what} values cannot have leading or trailing white space"
        )));
    }
    Ok(text)
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('/', "\\/")
        .replace('#', "\\#")
        .replace('{', "\\{")
}

/// pm4py's `_format_reference`.
fn reference(
    object: &str,
    qualifier: Option<&str>,
    attributes: &[(Arc<str>, AttributeValue)],
) -> Result<String> {
    let mut out = escape(reference_part(object, "object id")?);
    if let Some(q) = qualifier.filter(|q| !q.is_empty()) {
        out.push('#');
        out.push_str(&escape(reference_part(q, "qualifier")?));
    }
    if !attributes.is_empty() {
        out.push('{');
        for (i, (name, value)) in attributes.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_string(name, &mut out);
            out.push(':');
            match value {
                AttributeValue::String(s) | AttributeValue::Id(s) => json_string(s, &mut out),
                AttributeValue::Int(i) => out.push_str(&i.to_string()),
                AttributeValue::Float(f) if f.is_finite() => {
                    out.push_str(&value.to_string());
                }
                AttributeValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                AttributeValue::Date(d) => json_string(&isoformat(d), &mut out),
                _ => {
                    return Err(error(
                        "lists, maps and non-finite numbers are not valid OCEL 2.0 CSV attribute values",
                    ));
                }
            }
        }
        out.push('}');
    }
    Ok(out)
}

/// A JSON string as Python's `json.dumps(ensure_ascii=False)` writes it.
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
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The change values of one object at one time.
struct Group {
    timestamp: DateTime<FixedOffset>,
    object: Arc<str>,
    object_type: Arc<str>,
    values: Vec<(Arc<str>, AttributeValue)>,
}

/// Sets `name` to `value`, failing when it holds another value.
fn set_value(
    values: &mut Vec<(Arc<str>, AttributeValue)>,
    name: &Arc<str>,
    value: AttributeValue,
    conflict: impl FnOnce() -> Error,
) -> Result<()> {
    match values.iter_mut().find(|(n, _)| n == name) {
        Some((_, existing)) if !values_equal(existing, &value) => Err(conflict()),
        Some((_, existing)) => {
            *existing = value;
            Ok(())
        }
        None => {
            values.push((name.clone(), value));
            Ok(())
        }
    }
}

fn render(ocel: &Ocel) -> Result<String> {
    let mut ocel = ocel.clone();
    ocel.make_consistent();
    let mut ids = HashSet::new();
    if let Some(e) = ocel.events.iter().find(|e| !ids.insert(&*e.id)) {
        return Err(error(format!(
            "duplicate event id {:?} cannot be written",
            e.id
        )));
    }
    let mut object_type: HashMap<&str, &Arc<str>> = HashMap::new();
    for o in &ocel.objects {
        if object_type.insert(&o.id, &o.object_type).is_some() {
            return Err(error(format!(
                "duplicate object id {:?} cannot be written",
                o.id
            )));
        }
        reference_part(&o.id, "object id")?;
    }
    let mut types: Vec<&Arc<str>> = object_type.values().copied().collect();
    types.sort();
    types.dedup();
    let event_columns = Columns::new(ocel.events.iter().map(|e| &e.attributes));
    let object_columns = Columns::new(ocel.objects.iter().map(|o| &o.attributes));
    let event_names: Vec<&Arc<str>> = event_columns
        .names
        .iter()
        .filter(|n| !n.starts_with("ocel:"))
        .collect();
    let type_column = |t: &str| format!("{TYPE_PREFIX}{t}");
    let mut collisions: Vec<&str> = event_names
        .iter()
        .map(|n| &***n)
        .filter(|n| {
            [ID, ACTIVITY, TIMESTAMP].contains(n) || types.iter().any(|t| type_column(t) == *n)
        })
        .collect();
    if !collisions.is_empty() {
        collisions.sort_unstable();
        return Err(error(format!(
            "event attribute columns collide with OCEL 2.0 CSV columns: {}",
            collisions.join(", ")
        )));
    }

    let mut initial: HashMap<&str, Vec<(Arc<str>, AttributeValue)>> = HashMap::new();
    for o in &ocel.objects {
        let values: Vec<(Arc<str>, AttributeValue)> = object_columns
            .names
            .iter()
            .filter(|n| !n.starts_with("ocel:"))
            .filter_map(|n| {
                let value = normalized(o.attributes.get(n)?, object_columns.kind(n))?;
                Some((n.clone(), value))
            })
            .collect();
        if !values.is_empty() {
            initial.insert(&o.id, values);
        }
    }

    let event_ids: HashSet<&str> = ocel.events.iter().map(|e| &*e.id).collect();
    let mut keys = HashSet::new();
    let mut related: HashMap<&str, Vec<&EventObject>> = HashMap::new();
    for r in &ocel.relations {
        if !event_ids.contains(&*r.event) {
            return Err(error(format!(
                "a relation refers to the unknown event {:?}",
                r.event
            )));
        }
        if !object_type.contains_key(&*r.object) {
            return Err(error(format!(
                "a relation refers to the unknown object {:?}",
                r.object
            )));
        }
        if !keys.insert((&*r.event, &*r.object, r.qualifier.as_deref().unwrap_or(""))) {
            return Err(error(
                "a repeated event-to-object relation cannot be written",
            ));
        }
        related.entry(&r.event).or_default().push(r);
    }

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
    let change_columns = Columns::new(change_rows.iter());
    let epoch = DateTime::UNIX_EPOCH.fixed_offset();
    let mut groups: Vec<Group> = Vec::new();
    let mut group_index: HashMap<(DateTime<FixedOffset>, &str, &str), usize> = HashMap::new();
    for (c, row) in ocel.object_changes.iter().zip(&change_rows) {
        let known = object_type
            .get(&*c.object)
            .ok_or_else(|| error("an object change refers to an unknown object"))?;
        if c.object_type != **known {
            return Err(error(format!(
                "the type of an object change does not match the object {:?}",
                c.object
            )));
        }
        let value = row
            .get(&c.field)
            .and_then(|v| normalized(v, change_columns.kind(&c.field)))
            .ok_or_else(|| {
                error(format!(
                    "the object change for {:?} has no value for {:?}",
                    c.object, c.field
                ))
            })?;
        if c.timestamp == epoch {
            let values = initial.entry(&c.object).or_default();
            set_value(values, &c.field, value, || {
                error(format!(
                    "the object {:?} has conflicting time-0 values for {:?}",
                    c.object, c.field
                ))
            })?;
            continue;
        }
        let key = (c.timestamp, &*c.object_type, &*c.object);
        let i = *group_index.entry(key).or_insert_with(|| {
            groups.push(Group {
                timestamp: c.timestamp,
                object: c.object.clone(),
                object_type: c.object_type.clone(),
                values: Vec::new(),
            });
            groups.len() - 1
        });
        set_value(&mut groups[i].values, &c.field, value, || {
            error(format!(
                "the object {:?} has conflicting values for {:?} at {}",
                c.object,
                c.field,
                isoformat(&c.timestamp)
            ))
        })?;
    }

    let mut header: Vec<String> = vec![ID.into(), ACTIVITY.into(), TIMESTAMP.into()];
    header.extend(types.iter().map(|t| type_column(t)));
    header.extend(event_names.iter().map(|n| n.to_string()));
    let type_index: HashMap<&str, usize> = types
        .iter()
        .enumerate()
        .map(|(i, t)| (&***t, 3 + i))
        .collect();
    let blank = || vec![String::new(); header.len()];
    let mut out = Vec::new();
    write_record(&mut out, &header, "\r\n")?;

    let mut order: Vec<&OcelEvent> = ocel.events.iter().collect();
    order.sort_by_key(|e| e.timestamp);
    let mut established: HashSet<&str> = HashSet::new();
    for e in order {
        let mut row = blank();
        let trimmed = |text: &str, what: &str| {
            if text.is_empty() || py_strip(text) != text {
                Err(error(format!(
                    "OCEL 2.0 CSV {what} values must be non-empty and have no leading or trailing white space"
                )))
            } else {
                Ok(text.to_string())
            }
        };
        row[0] = trimmed(&e.id, "event id")?;
        row[1] = trimmed(&e.activity, "event activity")?;
        if fold(&e.activity) == O2O {
            return Err(error(
                "an event type named o2o cannot be written as OCEL 2.0 CSV",
            ));
        }
        row[2] = isoformat(&e.timestamp);
        for (i, name) in event_names.iter().enumerate() {
            if let Some(v) = e.attributes.get(name).filter(|v| !missing(v)) {
                row[3 + types.len() + i] = normalized_text(v, event_columns.kind(name));
            }
        }
        let mut cells: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for r in related.get(&*e.id).into_iter().flatten() {
            established.insert(&r.object);
            cells
                .entry(object_type[&*r.object])
                .or_default()
                .push(reference(&r.object, r.qualifier.as_deref(), &[])?);
        }
        for (t, refs) in cells {
            row[type_index[t]] = refs.join("/");
        }
        write_record(&mut out, &row, "\r\n")?;
    }

    let mut declarations: Vec<&OcelObject> = ocel.objects.iter().collect();
    declarations.sort_by(|a, b| (&a.object_type, &a.id).cmp(&(&b.object_type, &b.id)));
    for o in declarations {
        let values = initial.get(&*o.id).map(Vec::as_slice).unwrap_or(&[]);
        if established.contains(&*o.id) && values.is_empty() {
            continue;
        }
        let mut row = blank();
        row[type_index[&*o.object_type]] = reference(&o.id, None, values)?;
        write_record(&mut out, &row, "\r\n")?;
    }

    let mut o2o: BTreeMap<&str, BTreeMap<&str, Vec<String>>> = BTreeMap::new();
    let mut o2o_keys = HashSet::new();
    for r in &ocel.o2o {
        if !object_type.contains_key(&*r.source) {
            return Err(error(format!(
                "the object-to-object source {:?} has no object type",
                r.source
            )));
        }
        let target_type = object_type.get(&*r.target).ok_or_else(|| {
            error(format!(
                "the object-to-object target {:?} has no object type",
                r.target
            ))
        })?;
        if !o2o_keys.insert((&*r.source, &*r.target, r.qualifier.as_deref().unwrap_or(""))) {
            return Err(error(
                "a repeated object-to-object relation cannot be written",
            ));
        }
        o2o.entry(&r.source)
            .or_default()
            .entry(target_type)
            .or_default()
            .push(reference(&r.target, r.qualifier.as_deref(), &[])?);
    }
    for (source, cells) in o2o {
        let mut row = blank();
        row[0] = source.to_string();
        row[1] = O2O.to_string();
        for (t, mut refs) in cells {
            refs.sort();
            row[type_index[t]] = refs.join("/");
        }
        write_record(&mut out, &row, "\r\n")?;
    }

    groups.sort_by_key(|g| g.timestamp);
    for g in &groups {
        let mut row = blank();
        row[2] = isoformat(&g.timestamp);
        row[type_index[&*g.object_type]] = reference(&g.object, None, &g.values)?;
        write_record(&mut out, &row, "\r\n")?;
    }
    String::from_utf8(out).map_err(|_| error("the CSV text is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::{canonical_float, canonical_int, decimal, records, split_entries};

    #[test]
    fn reads_like_python_csv() {
        let r = |text: &str| records(text).unwrap();
        assert_eq!(r("a,b\r\n1,2"), [vec!["a", "b"], vec!["1", "2"]]);
        assert_eq!(
            r("a\r\n\r\nb\rc\n"),
            [vec!["a"], vec![], vec!["b"], vec!["c"]]
        );
        assert_eq!(r("\"x\r\ny\",\"q\"\"\"\n"), [vec!["x\r\ny", "q\""]]);
        assert_eq!(r("a\"b,\n"), [vec!["a\"b", ""]]);
        assert!(records("\"a\"x\n").is_err());
        assert!(records("\"a\n").is_err());
    }

    #[test]
    fn numbers_follow_pm4py() {
        assert_eq!(canonical_int("-0"), Some(0));
        assert_eq!(canonical_int("007"), None);
        assert_eq!(canonical_int("9223372036854775808"), None);
        assert_eq!(canonical_float("1e5"), None);
        assert_eq!(canonical_float("1e+16"), Some(1e16));
        assert_eq!(canonical_float("2"), Some(2.0));
        assert_eq!(canonical_float("9223372036854775807"), None);
        assert_eq!(decimal("1.0"), decimal("1"));
        assert_eq!(decimal("1e+16"), decimal("10000000000000000"));
        assert_ne!(decimal("1.5"), decimal("15"));
    }

    #[test]
    fn splits_references() {
        let s = |text: &str| split_entries(text).unwrap();
        assert_eq!(s("a/b"), ["a", "b"]);
        assert_eq!(
            s("a\\/1#q/b{\"k\":\"x/y}\"}"),
            ["a\\/1#q", "b{\"k\":\"x/y}\"}"]
        );
        assert!(split_entries("a//b").is_err());
        assert!(split_entries("a\\").is_err());
    }
}
