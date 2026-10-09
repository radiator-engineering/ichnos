//! Object-centric event logs in SQLite databases: pm4py's
//! `read_ocel_sqlite`, `read_ocel2_sqlite`, `write_ocel_sqlite` and
//! `write_ocel2_sqlite`.
//!
//! pm4py moves each table between pandas and SQLite with `read_sql` and
//! `to_sql`, so pandas' column types decide what is read and written.
//!
//! Reading, a column's cells give it a type: integers alone are integers,
//! integers with a float or a `NULL` are floats, text alone (with `NULL`s)
//! is text, and a column with text and numbers keeps each cell's own type.
//! When pm4py joins several tables into one, as the OCEL 2.0 reader does
//! for the per-type tables, a column that one table lacks counts as missing
//! there: an integer column becomes a float column, and a column that is
//! all `NULL` in one table keeps each cell's own type.
//!
//! Writing, a column's type follows from its values as in
//! [`write`](super::write): a float column is `REAL`, an integer or boolean
//! column is `INTEGER` and a date column is `TIMESTAMP`, holding text such
//! as `2022-01-09 14:00:00.500000+00:00`. A column of other values is
//! `TEXT`, unless all of them are integers, floats, booleans or dates, and
//! each value is stored with its own type. SQLite then converts it to the
//! column's affinity. An empty table's columns are `REAL`, as pm4py writes
//! them for an empty log.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, SubsecRound, TimeZone};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::constants::{
    EVENT_ACTIVITY, EVENT_ID, EVENT_TIMESTAMP, OBJECT_ID, OBJECT_TYPE, QUALIFIER,
};
use ichnos_ocel::{EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject};
use rusqlite::types::Value;
use rusqlite::{Connection, OpenFlags, params_from_iter};

use super::csv::parse_timestamp;
use super::lower_name;
use super::write::{Kind, Prepared, as_float, isoformat, missing};
use crate::error::{Error, Result};

fn error(detail: impl Into<String>) -> Error {
    Error::Ocel(detail.into())
}

/// A column's pandas type, as far as SQLite input and output care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dtype {
    /// `object` with every cell missing.
    Empty,
    /// `int64`.
    Int,
    /// `float64`.
    Float,
    /// `bool`.
    Bool,
    /// `str`.
    Str,
    /// `datetime64`.
    Date,
    /// `object`: each cell keeps its own type.
    Object,
}

impl From<Kind> for Dtype {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Float => Self::Float,
            Kind::Date => Self::Date,
            Kind::Int => Self::Int,
            Kind::Bool => Self::Bool,
            Kind::Object => Self::Object,
        }
    }
}

/// The type pandas' `concat` gives a column from two parts' types, where
/// `None` is a part without the column, which pandas fills with missing
/// values.
fn combine(a: Option<Dtype>, b: Option<Dtype>) -> Option<Dtype> {
    use Dtype::*;
    Some(match (a, b) {
        (None, None) => return None,
        (Some(k), None) | (None, Some(k)) => match k {
            Int => Float,
            Bool | Empty => Object,
            k => k,
        },
        (Some(Empty), _) | (_, Some(Empty)) => Object,
        (Some(x), Some(y)) if x == y => x,
        (Some(Int | Bool), Some(Int | Bool)) => Int,
        (Some(Int | Float | Bool), Some(Int | Float | Bool)) => Float,
        _ => Object,
    })
}

/// The type `read_sql` gives a column from its cells.
fn read_dtype<'a>(cells: impl Iterator<Item = &'a Value>) -> Dtype {
    let (mut int, mut real, mut text, mut blob, mut null) = (false, false, false, false, false);
    for cell in cells {
        match cell {
            Value::Null => null = true,
            Value::Integer(_) => int = true,
            Value::Real(_) => real = true,
            Value::Text(_) => text = true,
            Value::Blob(_) => blob = true,
        }
    }
    if text || blob {
        if text && !blob && !int && !real {
            Dtype::Str
        } else {
            Dtype::Object
        }
    } else if real || (int && null) {
        Dtype::Float
    } else if int {
        Dtype::Int
    } else {
        Dtype::Empty
    }
}

/// Python's `str` of a cell in a column of type `dtype`, or `None` for
/// `NULL`.
fn cell_text(cell: &Value, dtype: Dtype) -> Result<Option<String>> {
    Ok(Some(match cell {
        Value::Null => return Ok(None),
        Value::Integer(i) if dtype == Dtype::Float => AttributeValue::Float(*i as f64).to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => AttributeValue::Float(*f).to_string(),
        Value::Text(s) => s.clone(),
        Value::Blob(_) => return Err(error("a BLOB value cannot be an id, type or name")),
    }))
}

/// A cell as an attribute value in a column of type `dtype`, or `None` for
/// `NULL`.
fn cell_value(cell: &Value, dtype: Dtype) -> Result<Option<AttributeValue>> {
    Ok(Some(match cell {
        Value::Null => return Ok(None),
        Value::Integer(i) if dtype == Dtype::Float => AttributeValue::Float(*i as f64),
        Value::Integer(i) => AttributeValue::Int(*i),
        Value::Real(f) => AttributeValue::Float(*f),
        Value::Text(s) => AttributeValue::String(s.as_str().into()),
        Value::Blob(_) => return Err(error("BLOB attribute values are not supported")),
    }))
}

/// A cell of a timestamp column: text with an ISO 8601 date, optionally
/// with a time and an offset. A time without an offset is UTC.
fn cell_time(cell: &Value, table: &str) -> Result<DateTime<FixedOffset>> {
    let text = match cell {
        Value::Text(s) => s,
        Value::Null => return Err(error(format!("a row of {table} has no timestamp"))),
        _ => {
            return Err(error(format!(
                "a timestamp of {table} is not text: {cell:?}"
            )));
        }
    };
    let bad = || error(format!("cannot read the timestamp {text:?}"));
    let (local, form) = parse_timestamp(text).ok_or_else(bad)?;
    FixedOffset::east_opt(form.offset.unwrap_or(0))
        .and_then(|offset| offset.from_local_datetime(&local).single())
        .ok_or_else(bad)
}

/// A table as `SELECT *` gives it.
struct Table {
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
}

impl Table {
    fn read(conn: &Connection, name: &str) -> Result<Self> {
        let mut statement = conn.prepare(&format!("SELECT * FROM {}", quote(name)?))?;
        let columns: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(String::from)
            .collect();
        let n = columns.len();
        let rows = statement
            .query_map([], |row| (0..n).map(|i| row.get::<_, Value>(i)).collect())?
            .collect::<rusqlite::Result<Vec<Vec<Value>>>>()?;
        Ok(Self {
            name: name.into(),
            columns,
            rows,
        })
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    fn column(&self, name: &str) -> Result<usize> {
        self.find(name)
            .ok_or_else(|| error(format!("the table {} has no column {name}", self.name)))
    }

    fn dtype(&self, i: usize) -> Dtype {
        read_dtype(self.rows.iter().map(|r| &r[i]))
    }

    /// Each row's text in column `i`.
    fn texts(&self, i: usize) -> Result<Vec<Option<String>>> {
        let dtype = self.dtype(i);
        self.rows.iter().map(|r| cell_text(&r[i], dtype)).collect()
    }

    /// Replaces column `i` with pm4py's OCEL 2.0 normalized ids.
    fn normalize_ids(&mut self, i: usize) -> Result<()> {
        let ids = normalized_ids(self, i)?;
        for (row, id) in self.rows.iter_mut().zip(ids) {
            row[i] = id.map_or(Value::Null, Value::Text);
        }
        Ok(())
    }
}

/// `"name"` with inner quotes doubled, as pandas writes an identifier.
fn quote(name: &str) -> Result<String> {
    if name.is_empty() {
        return Err(error("SQLite names cannot be empty"));
    }
    if name.contains('\0') {
        return Err(error("SQLite names cannot contain NUL"));
    }
    Ok(format!("\"{}\"", name.replace('"', "\"\"")))
}

/// pm4py's `_normalize_id_series`: a float column of whole numbers gives
/// integer text, a number column gives Python's text, and a text column
/// loses a trailing backslash, character and `0` (the effect of its
/// `\\.0$` pattern).
fn normalized_ids(table: &Table, i: usize) -> Result<Vec<Option<String>>> {
    let dtype = table.dtype(i);
    let cells = table.rows.iter().map(|r| &r[i]);
    if dtype == Dtype::Float {
        let whole = cells.clone().all(|c| match c {
            Value::Real(f) => f.fract() == 0.0,
            _ => true,
        });
        if whole {
            return Ok(cells
                .map(|c| match c {
                    Value::Integer(n) => Some(n.to_string()),
                    Value::Real(f) => Some(format!("{f:.0}")),
                    _ => None,
                })
                .collect());
        }
    }
    let texts = table.texts(i)?;
    if matches!(dtype, Dtype::Float | Dtype::Int) {
        return Ok(texts);
    }
    Ok(texts
        .into_iter()
        .map(|t| t.map(strip_escape_suffix))
        .collect())
}

/// Removes `\`, one character other than a line break, and `0` at the end
/// of the text or before a final line break, as Python's
/// `re.sub(r"\\.0$", "", text)` does.
fn strip_escape_suffix(text: String) -> String {
    let (body, tail) = match text.strip_suffix('\n') {
        Some(body) => (body, "\n"),
        None => (text.as_str(), ""),
    };
    let mut chars = body.char_indices().rev();
    if let (Some((_, '0')), Some((_, c)), Some((start, '\\'))) =
        (chars.next(), chars.next(), chars.next())
        && c != '\n'
    {
        return format!("{}{tail}", &body[..start]);
    }
    text
}

/// Several tables joined into one, as pandas' `concat` joins them: every
/// column of any table, in order of first appearance, with each row's
/// index in its own table.
struct Joined {
    columns: Vec<String>,
    dtypes: Vec<Dtype>,
    rows: Vec<(usize, Vec<Value>)>,
}

impl Joined {
    fn new(tables: Vec<Table>) -> Self {
        let mut columns: Vec<String> = Vec::new();
        for t in &tables {
            for c in &t.columns {
                if !columns.contains(c) {
                    columns.push(c.clone());
                }
            }
        }
        let mut dtypes: Vec<Option<Dtype>> = vec![None; columns.len()];
        let mut first = true;
        for t in tables.iter().filter(|t| !t.rows.is_empty()) {
            for (j, c) in columns.iter().enumerate() {
                let part = t.find(c).map(|i| t.dtype(i));
                dtypes[j] = if first {
                    part
                } else {
                    combine(dtypes[j], part)
                };
            }
            first = false;
        }
        let dtypes = dtypes
            .into_iter()
            .map(|d| d.unwrap_or(Dtype::Empty))
            .collect();
        let mut rows = Vec::new();
        for t in tables {
            let at: Vec<Option<usize>> = columns.iter().map(|c| t.find(c)).collect();
            for (index, mut row) in t.rows.into_iter().enumerate() {
                let joined = at
                    .iter()
                    .map(|i| i.map_or(Value::Null, |i| std::mem::replace(&mut row[i], Value::Null)))
                    .collect();
                rows.push((index, joined));
            }
        }
        Self {
            columns,
            dtypes,
            rows,
        }
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// The row's attributes: every column but `skip`, with a value.
    fn attributes(&self, row: &[Value], skip: &[Option<usize>]) -> Result<Attributes> {
        attributes(&self.columns, &self.dtypes, row, skip)
    }
}

fn attributes(
    columns: &[String],
    dtypes: &[Dtype],
    row: &[Value],
    skip: &[Option<usize>],
) -> Result<Attributes> {
    let mut out = Attributes::default();
    for (i, name) in columns.iter().enumerate() {
        if skip.contains(&Some(i)) {
            continue;
        }
        if let Some(value) = cell_value(&row[i], dtypes[i])? {
            out.insert(name.as_str(), value);
        }
    }
    Ok(out)
}

fn open(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?)
}

/// Reads an OCEL 1.0 log from a SQLite database (pm4py's
/// `read_ocel_sqlite`, `pandas_importer` variant).
///
/// The database holds the tables `EVENTS` (`ocel:eid`, `ocel:timestamp`,
/// `ocel:activity` and attribute columns), `OBJECTS` (`ocel:oid`,
/// `ocel:type` and attribute columns) and `RELATIONS` (`ocel:eid`,
/// `ocel:activity`, `ocel:timestamp`, `ocel:oid`, `ocel:type` and,
/// optionally, `ocel:qualifier`). Each timestamp is ISO 8601 text, read as
/// UTC without an offset. Rows keep their table order. Then
/// [`Ocel::make_consistent`] drops rows with a missing or empty id, activity
/// or type, including relations whose own `ocel:activity` or `ocel:type` is
/// missing, and [`Ocel::retain_related`] runs.
pub fn read_ocel_sqlite(path: impl AsRef<Path>) -> Result<Ocel> {
    let conn = open(path.as_ref())?;
    let events = Table::read(&conn, "EVENTS")?;
    let objects = Table::read(&conn, "OBJECTS")?;
    let relations = Table::read(&conn, "RELATIONS")?;
    let mut ocel = Ocel::new();

    let (eid, ts, act) = (
        events.column(EVENT_ID)?,
        events.column(EVENT_TIMESTAMP)?,
        events.column(EVENT_ACTIVITY)?,
    );
    let dtypes: Vec<Dtype> = (0..events.columns.len()).map(|i| events.dtype(i)).collect();
    let (ids, activities) = (events.texts(eid)?, events.texts(act)?);
    for (r, row) in events.rows.iter().enumerate() {
        ocel.events.push(OcelEvent {
            id: ids[r].as_deref().unwrap_or("").into(),
            activity: activities[r].as_deref().unwrap_or("").into(),
            timestamp: cell_time(&row[ts], "EVENTS")?,
            attributes: attributes(
                &events.columns,
                &dtypes,
                row,
                &[Some(eid), Some(ts), Some(act)],
            )?,
        });
    }

    let (oid, otype) = (objects.column(OBJECT_ID)?, objects.column(OBJECT_TYPE)?);
    let dtypes: Vec<Dtype> = (0..objects.columns.len())
        .map(|i| objects.dtype(i))
        .collect();
    let (ids, types) = (objects.texts(oid)?, objects.texts(otype)?);
    for (r, row) in objects.rows.iter().enumerate() {
        ocel.objects.push(OcelObject {
            id: ids[r].as_deref().unwrap_or("").into(),
            object_type: types[r].as_deref().unwrap_or("").into(),
            attributes: attributes(&objects.columns, &dtypes, row, &[Some(oid), Some(otype)])?,
        });
    }

    let (reid, roid) = (relations.column(EVENT_ID)?, relations.column(OBJECT_ID)?);
    let texts = |name: &str| -> Result<Option<Vec<Option<String>>>> {
        relations.find(name).map(|i| relations.texts(i)).transpose()
    };
    let (events_of, objects_of) = (relations.texts(reid)?, relations.texts(roid)?);
    let qualifiers = texts(QUALIFIER)?;
    // pm4py's consistency step also checks the relation's own activity and
    // type.
    let checked = [texts(EVENT_ACTIVITY)?, texts(OBJECT_TYPE)?];
    for r in 0..relations.rows.len() {
        let valid = checked
            .iter()
            .flatten()
            .all(|column| column[r].as_deref().is_some_and(|t| !t.is_empty()));
        if !valid {
            continue;
        }
        ocel.relations.push(EventObject {
            event: events_of[r].as_deref().unwrap_or("").into(),
            object: objects_of[r].as_deref().unwrap_or("").into(),
            qualifier: qualifiers
                .as_ref()
                .and_then(|q| q[r].as_deref())
                .map(Arc::from),
        });
    }
    ocel.make_consistent();
    ocel.retain_related();
    Ok(ocel)
}

/// The `ocel_type` → `ocel_type_map` table.
fn type_map(conn: &Connection, name: &str) -> Result<HashMap<String, String>> {
    let table = Table::read(conn, name)?;
    let (from, to) = (table.column("ocel_type")?, table.column("ocel_type_map")?);
    let keys = table.texts(from)?;
    let mut map = HashMap::new();
    for (key, row) in keys.into_iter().zip(&table.rows) {
        let Value::Text(value) = &row[to] else {
            return Err(error(format!(
                "{name} maps a type to a value that is not text"
            )));
        };
        if let Some(key) = key {
            map.insert(key, value.clone());
        }
    }
    Ok(map)
}

/// Each id's type, as the `event` or `object` table gives it.
type IdTypes = HashMap<String, Arc<str>>;

/// The `event` or `object` table: each id's type, and the sorted types.
fn id_types(conn: &Connection, name: &str) -> Result<(IdTypes, Vec<String>)> {
    let mut table = Table::read(conn, name)?;
    let (id, kind) = (table.column("ocel_id")?, table.column("ocel_type")?);
    table.normalize_ids(id)?;
    let ids = table.texts(id)?;
    let mut types = Vec::new();
    let mut map = HashMap::new();
    for (i, t) in ids.into_iter().zip(table.texts(kind)?) {
        let t = t.ok_or_else(|| error(format!("the {name} table has a row without a type")))?;
        if let Some(i) = i {
            map.insert(i, Arc::from(t.as_str()));
        }
        types.push(t);
    }
    types.sort();
    types.dedup();
    Ok((map, types))
}

/// The per-type tables `{prefix}{map[type]}`, with `ocel_id` normalized,
/// joined.
fn type_tables(
    conn: &Connection,
    prefix: &str,
    types: &[String],
    map: &HashMap<String, String>,
) -> Result<Joined> {
    let mut tables = Vec::new();
    for t in types {
        let mapped = map
            .get(t)
            .ok_or_else(|| error(format!("{prefix}map_type has no entry for {t:?}")))?;
        let mut table = Table::read(conn, &format!("{prefix}{mapped}"))?;
        let id = table.column("ocel_id")?;
        table.normalize_ids(id)?;
        tables.push(table);
    }
    if tables.is_empty() {
        return Err(error(format!("the database has no {prefix}types")));
    }
    Ok(Joined::new(tables))
}

/// Reads an OCEL 2.0 log from a SQLite database (pm4py's
/// `read_ocel2_sqlite`, `ocel20` variant).
///
/// The database holds the tables `event` and `object` (`ocel_id`,
/// `ocel_type`), `event_map_type` and `object_map_type` (`ocel_type`,
/// `ocel_type_map`), `event_object` (`ocel_event_id`, `ocel_object_id`,
/// `ocel_qualifier`), `object_object` (`ocel_source_id`, `ocel_target_id`,
/// `ocel_qualifier`), and per type one table named `event_` or `object_`
/// followed by the type's map entry. An event table holds `ocel_id`,
/// `ocel_time` and attribute columns; an object table holds `ocel_id`,
/// attribute columns and, for object changes, `ocel_time` and
/// `ocel_changed_field`.
///
/// As in pm4py:
/// - Ids are text. A float column of whole numbers gives integer text.
/// - An object table row without a changed field is an object, and the
///   others are changes. When every row has one, each object's first row is
///   the object and the others are changes.
/// - Events, relations and changes are sorted by timestamp. Ties keep the
///   order of the row index within its own table, then the table order.
/// - [`Ocel::make_consistent`] and [`Ocel::retain_related`] run last.
///   Relations to an event or object missing from `event` or `object` are
///   dropped first.
pub fn read_ocel2_sqlite(path: impl AsRef<Path>) -> Result<Ocel> {
    let conn = open(path.as_ref())?;
    let (event_types, etypes) = id_types(&conn, "event")?;
    let (object_types, otypes) = id_types(&conn, "object")?;
    let event_map = type_map(&conn, "event_map_type")?;
    let object_map = type_map(&conn, "object_map_type")?;
    let mut ocel = Ocel::new();

    let events = type_tables(&conn, "event_", &etypes, &event_map)?;
    let id = events.find("ocel_id");
    let time = events
        .find("ocel_time")
        .ok_or_else(|| error("an event table has no ocel_time column"))?;
    // pm4py sets the activity from the `event` table.
    let skip = [id, Some(time), events.find(EVENT_ACTIVITY)];
    let mut keyed = Vec::with_capacity(events.rows.len());
    for (index, row) in &events.rows {
        let id = match &row[id.unwrap_or(0)] {
            Value::Text(s) => s.as_str(),
            _ => "",
        };
        let event = OcelEvent {
            id: id.into(),
            activity: event_types.get(id).cloned().unwrap_or_else(|| "".into()),
            timestamp: cell_time(&row[time], "an event table")?,
            attributes: events.attributes(row, &skip)?,
        };
        keyed.push(((event.timestamp, *index), event));
    }
    keyed.sort_by_key(|(key, _)| *key);
    let times: HashMap<Arc<str>, DateTime<FixedOffset>> = keyed
        .iter()
        .map(|(_, e)| (e.id.clone(), e.timestamp))
        .collect();
    ocel.events = keyed.into_iter().map(|(_, e)| e).collect();

    let objects = type_tables(&conn, "object_", &otypes, &object_map)?;
    let id = objects.find("ocel_id");
    let time = objects.find("ocel_time");
    let field = objects.find("ocel_changed_field");
    let skip = [id, time, field, objects.find(OBJECT_TYPE)];
    let text = |row: &[Value], i: Option<usize>| match i.map(|i| &row[i]) {
        Some(Value::Text(s)) => Some(s.clone()),
        _ => None,
    };
    let is_change: Vec<bool> = match field {
        None => vec![false; objects.rows.len()],
        Some(f) => {
            let changed: Vec<bool> = objects
                .rows
                .iter()
                .map(|(_, r)| r[f] != Value::Null)
                .collect();
            if changed.iter().all(|c| *c) {
                // pm4py's `cumcount` split: each object's first row.
                let mut seen = HashSet::new();
                objects
                    .rows
                    .iter()
                    .map(|(_, r)| match text(r, id) {
                        Some(i) => !seen.insert(i),
                        None => true,
                    })
                    .collect()
            } else {
                changed
            }
        }
    };
    let mut changes = Vec::new();
    for ((index, row), change) in objects.rows.iter().zip(is_change) {
        let oid = text(row, id).unwrap_or_default();
        let object_type = object_types.get(&oid).cloned().unwrap_or_else(|| "".into());
        if !change {
            ocel.objects.push(OcelObject {
                id: oid.into(),
                object_type,
                attributes: objects.attributes(row, &skip)?,
            });
            continue;
        }
        let Some(f) = field else { continue };
        let name = cell_text(&row[f], objects.dtypes[f])?.unwrap_or_default();
        let time = time.ok_or_else(|| error("an object table has no ocel_time column"))?;
        let value = match objects.find(&name).filter(|i| !skip.contains(&Some(*i))) {
            Some(i) => cell_value(&row[i], objects.dtypes[i])?,
            None => None,
        };
        let change = ObjectChange {
            object: oid.into(),
            object_type,
            timestamp: cell_time(&row[time], "an object table")?,
            field: name.into(),
            value,
        };
        changes.push(((change.timestamp, *index), change));
    }
    changes.sort_by_key(|(key, _)| *key);
    ocel.object_changes = changes.into_iter().map(|(_, c)| c).collect();

    let mut e2o = Table::read(&conn, "event_object")?;
    let (eid, oid) = (e2o.column("ocel_event_id")?, e2o.column("ocel_object_id")?);
    e2o.normalize_ids(eid)?;
    e2o.normalize_ids(oid)?;
    let qualifiers = match e2o.find("ocel_qualifier") {
        Some(q) => e2o.texts(q)?,
        None => vec![None; e2o.rows.len()],
    };
    let (eids, oids) = (e2o.texts(eid)?, e2o.texts(oid)?);
    let mut relations = Vec::new();
    for (index, ((e, o), q)) in eids.into_iter().zip(oids).zip(qualifiers).enumerate() {
        let (Some(e), Some(o)) = (e, o) else { continue };
        if !event_types.contains_key(&e) || !object_types.contains_key(&o) {
            continue;
        }
        // An event without a row in its type table sorts last.
        let key = (
            times.get(e.as_str()).copied().is_none(),
            times.get(e.as_str()).copied(),
            index,
        );
        relations.push((
            key,
            EventObject {
                event: e.into(),
                object: o.into(),
                qualifier: q.map(Arc::from),
            },
        ));
    }
    relations.sort_by_key(|(key, _)| *key);
    ocel.relations = relations.into_iter().map(|(_, r)| r).collect();

    let mut o2o = Table::read(&conn, "object_object")?;
    let (source, target) = (o2o.column("ocel_source_id")?, o2o.column("ocel_target_id")?);
    o2o.normalize_ids(source)?;
    o2o.normalize_ids(target)?;
    let qualifiers = match o2o.find("ocel_qualifier") {
        Some(q) => o2o.texts(q)?,
        None => vec![None; o2o.rows.len()],
    };
    for ((s, t), q) in o2o
        .texts(source)?
        .into_iter()
        .zip(o2o.texts(target)?)
        .zip(qualifiers)
    {
        ocel.o2o.push(ObjectObject {
            source: s.unwrap_or_default().into(),
            target: t.unwrap_or_default().into(),
            qualifier: q.map(Arc::from),
        });
    }
    ocel.make_consistent();
    ocel.retain_related();
    Ok(ocel)
}

/// A table to write: its name, its columns with their declared types, and
/// its rows.
struct Out {
    name: String,
    columns: Vec<(String, &'static str)>,
    rows: Vec<Vec<Value>>,
}

impl Out {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// Adds a column. `values` holds one value per row.
    fn push(&mut self, name: &str, (declared, values): (&'static str, Vec<Value>)) {
        if self.rows.is_empty() {
            self.rows = vec![Vec::new(); values.len()];
        }
        self.columns.push((name.into(), declared));
        for (row, v) in self.rows.iter_mut().zip(values) {
            row.push(v);
        }
    }

    /// A text column (pandas `str`).
    fn push_text<'a>(&mut self, name: &str, values: impl Iterator<Item = &'a str>) {
        let values: Vec<Value> = values.map(|v| Value::Text(v.into())).collect();
        self.push(name, ("TEXT", values));
    }

    /// Keeps the first of equal rows (pandas' `drop_duplicates`).
    fn dedup(&mut self) {
        let mut seen = HashSet::new();
        self.rows.retain(|r| seen.insert(format!("{r:?}")));
    }

    /// Creates the table as pandas' `to_sql` does and inserts the rows.
    fn write(&self, conn: &Connection) -> Result<()> {
        let empty = self.rows.is_empty();
        let mut columns = Vec::with_capacity(self.columns.len());
        for (name, declared) in &self.columns {
            columns.push(format!(
                "{} {}",
                quote(name)?,
                if empty { "REAL" } else { declared }
            ));
        }
        conn.execute(
            &format!(
                "CREATE TABLE {} (\n{}\n)",
                quote(&self.name)?,
                columns.join(",\n  ")
            ),
            [],
        )?;
        if empty {
            return Ok(());
        }
        let names: Vec<String> = self
            .columns
            .iter()
            .map(|(n, _)| quote(n))
            .collect::<Result<_>>()?;
        let mut insert = conn.prepare(&format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote(&self.name)?,
            names.join(","),
            vec!["?"; names.len()].join(",")
        ))?;
        for row in &self.rows {
            insert.execute(params_from_iter(row.iter()))?;
        }
        Ok(())
    }
}

/// A time as Python's `datetime.isoformat(" ")` writes it after pandas'
/// `to_pydatetime`: in UTC, to the microsecond.
fn py_datetime(d: &DateTime<FixedOffset>) -> String {
    isoformat(&d.trunc_subsecs(6)).replacen('T', " ", 1)
}

/// A value as Python's `sqlite3` binds it from an `object` column.
fn bind(value: &AttributeValue) -> Result<Value> {
    Ok(match value.plain() {
        AttributeValue::String(s) | AttributeValue::Id(s) => Value::Text(s.to_string()),
        AttributeValue::Int(i) => Value::Integer(*i),
        AttributeValue::Float(f) if f.is_nan() => Value::Null,
        AttributeValue::Float(f) => Value::Real(*f),
        AttributeValue::Bool(b) => Value::Integer(i64::from(*b)),
        // `Timestamp.isoformat(" ")`.
        AttributeValue::Date(d) => Value::Text(isoformat(d).replacen('T', " ", 1)),
        AttributeValue::List(_) | AttributeValue::Container(_) | AttributeValue::Meta(_) => {
            return Err(error("SQLite cannot hold a list or container attribute"));
        }
    })
}

/// pandas' `infer_dtype(skipna=True)` mapped to a SQLite type, for an
/// `object` column.
fn inferred(values: &[&AttributeValue]) -> &'static str {
    let all = |f: fn(&AttributeValue) -> bool| values.iter().all(|v| f(v.plain()));
    if values.is_empty() || all(|v| matches!(v, AttributeValue::String(_) | AttributeValue::Id(_)))
    {
        "TEXT"
    } else if all(|v| matches!(v, AttributeValue::Int(_))) {
        "INTEGER"
    } else if all(|v| matches!(v, AttributeValue::Float(_))) {
        "REAL"
    } else if all(|v| matches!(v, AttributeValue::Bool(_))) {
        "INTEGER"
    } else if all(|v| matches!(v, AttributeValue::Date(_))) {
        "TIMESTAMP"
    } else {
        "TEXT"
    }
}

/// The value, unless it is missing.
fn present<'a>(value: &Option<&'a AttributeValue>) -> Option<&'a AttributeValue> {
    value.filter(|v| !missing(v))
}

/// A column's declared type and values. With `normalize`, an `object`
/// column first goes through pm4py's `clean_dataframes.normalize_value`
/// (dates become ISO text) and pandas' `map`, which turns numbers with a
/// float or a gap into floats.
fn column(
    dtype: Dtype,
    values: &[Option<&AttributeValue>],
    normalize: bool,
) -> Result<(&'static str, Vec<Value>)> {
    let floats = |values: &[Option<&AttributeValue>]| {
        values
            .iter()
            .map(|v| {
                present(v)
                    .and_then(|v| match v.plain() {
                        AttributeValue::Bool(b) => Some(f64::from(u8::from(*b))),
                        v => as_float(v),
                    })
                    .map_or(Value::Null, Value::Real)
            })
            .collect()
    };
    match dtype {
        Dtype::Float => return Ok(("REAL", floats(values))),
        Dtype::Int | Dtype::Bool => {
            let ints = values
                .iter()
                .map(|v| match present(v).map(AttributeValue::plain) {
                    Some(AttributeValue::Int(i)) => Value::Integer(*i),
                    Some(AttributeValue::Bool(b)) => Value::Integer(i64::from(*b)),
                    _ => Value::Null,
                })
                .collect();
            return Ok(("INTEGER", ints));
        }
        Dtype::Date => {
            let dates = values
                .iter()
                .map(|v| match present(v).map(AttributeValue::plain) {
                    Some(AttributeValue::Date(d)) => Value::Text(py_datetime(d)),
                    _ => Value::Null,
                })
                .collect();
            return Ok(("TIMESTAMP", dates));
        }
        Dtype::Empty | Dtype::Str | Dtype::Object => {}
    }
    let normalized: Vec<Option<AttributeValue>> = values
        .iter()
        .map(|v| {
            present(v).map(|v| match v.plain() {
                AttributeValue::Date(d) if normalize => AttributeValue::String(isoformat(d).into()),
                v => v.clone(),
            })
        })
        .collect();
    let refs: Vec<Option<&AttributeValue>> = normalized.iter().map(Option::as_ref).collect();
    let set: Vec<&AttributeValue> = refs.iter().flatten().copied().collect();
    if normalize && !set.is_empty() {
        let numbers = set
            .iter()
            .all(|v| matches!(v.plain(), AttributeValue::Int(_) | AttributeValue::Float(_)));
        let gaps = set.len() < refs.len();
        let float = set
            .iter()
            .any(|v| matches!(v.plain(), AttributeValue::Float(_)));
        if numbers && (gaps || float) {
            return Ok(("REAL", floats(&refs)));
        }
    }
    let bound = refs
        .iter()
        .map(|v| v.map_or(Ok(Value::Null), bind))
        .collect::<Result<_>>()?;
    Ok((inferred(&set), bound))
}

/// Opens a new database at `path`, replacing any file there, and writes
/// the tables in one transaction.
fn create(path: &Path, tables: &[Out]) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    let mut conn = Connection::open(path)?;
    let transaction = conn.transaction()?;
    for t in tables {
        t.write(&transaction)?;
    }
    transaction.commit()?;
    Ok(())
}

/// `path`, with `.sqlite` added when its name does not end in `sqlite`, as
/// pm4py's `write_ocel_sqlite` and `write_ocel2_sqlite` do.
fn sqlite_path(path: &Path) -> PathBuf {
    if lower_name(path).ends_with("sqlite") {
        path.to_path_buf()
    } else {
        let mut name = path.as_os_str().to_owned();
        name.push(".sqlite");
        PathBuf::from(name)
    }
}

/// Writes an OCEL 1.0 log to a SQLite database (pm4py's
/// `write_ocel_sqlite`, `pandas_exporter` variant), replacing any file at
/// `path`. A name that does not end in `sqlite` gets `.sqlite` added.
///
/// The database holds three tables, as [`read_ocel_sqlite`] reads them:
/// `EVENTS`, `RELATIONS` and `OBJECTS`. The writer first runs
/// [`Ocel::make_consistent`] and [`Ocel::retain_related`] on a copy, as
/// pm4py's does. Object changes and object-to-object relations are not
/// written. A list or container attribute is an error.
pub fn write_ocel_sqlite(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    let p = Prepared::new(ocel);
    let log = &p.ocel;
    let mut events = Out::new("EVENTS");
    events.push_text(EVENT_ID, log.events.iter().map(|e| &*e.id));
    let times = log
        .events
        .iter()
        .map(|e| Value::Text(py_datetime(&e.timestamp)))
        .collect();
    events.push(EVENT_TIMESTAMP, ("TIMESTAMP", times));
    events.push_text(EVENT_ACTIVITY, log.events.iter().map(|e| &*e.activity));
    for name in &p.events.names {
        let values: Vec<_> = log.events.iter().map(|e| e.attributes.get(name)).collect();
        events.push(name, column(p.events.kind(name).into(), &values, false)?);
    }

    let by_id: HashMap<&str, &OcelEvent> = log.events.iter().rev().map(|e| (&*e.id, e)).collect();
    let types: HashMap<&str, &str> = log
        .objects
        .iter()
        .rev()
        .map(|o| (&*o.id, &*o.object_type))
        .collect();
    let mut relations = Out::new("RELATIONS");
    let rel = &log.relations;
    relations.push_text(EVENT_ID, rel.iter().map(|r| &*r.event));
    relations.push_text(
        EVENT_ACTIVITY,
        rel.iter().map(|r| &*by_id[&*r.event].activity),
    );
    let times = rel
        .iter()
        .map(|r| Value::Text(py_datetime(&by_id[&*r.event].timestamp)))
        .collect();
    relations.push(EVENT_TIMESTAMP, ("TIMESTAMP", times));
    relations.push_text(OBJECT_ID, rel.iter().map(|r| &*r.object));
    relations.push_text(OBJECT_TYPE, rel.iter().map(|r| types[&*r.object]));
    relations.push_text(
        QUALIFIER,
        rel.iter().map(|r| r.qualifier.as_deref().unwrap_or("")),
    );

    let mut objects = Out::new("OBJECTS");
    objects.push_text(OBJECT_ID, log.objects.iter().map(|o| &*o.id));
    objects.push_text(OBJECT_TYPE, log.objects.iter().map(|o| &*o.object_type));
    for name in &p.objects.names {
        let values: Vec<_> = log.objects.iter().map(|o| o.attributes.get(name)).collect();
        objects.push(name, column(p.objects.kind(name).into(), &values, false)?);
    }
    create(&sqlite_path(path.as_ref()), &[events, relations, objects])
}

/// pm4py's `names_stripping.apply`: each space-separated word capitalized,
/// joined, then only ASCII letters and digits kept, at most 100.
fn strip_name(name: &str) -> String {
    let mut joined = String::new();
    for word in name.split(' ') {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            // Python's title case: the first character of an upper-case
            // expansion, then the rest in lower case.
            let mut upper = first.to_uppercase();
            joined.extend(upper.next());
            joined.extend(upper.flat_map(char::to_lowercase));
            joined.extend(chars.flat_map(char::to_lowercase));
        }
    }
    joined
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(100)
        .collect()
}

/// pm4py's `_build_type_mapping`: each sorted type's table name, `Type`
/// when stripping leaves nothing, with `_2`, `_3` and so on added when a
/// name repeats ignoring case.
fn table_names(types: &[&Arc<str>]) -> Vec<String> {
    let mut used = HashSet::new();
    types
        .iter()
        .map(|t| {
            let mut base = strip_name(t);
            if base.is_empty() {
                base = "Type".into();
            }
            let mut candidate = base.clone();
            let mut suffix = 2;
            while used.contains(&candidate.to_ascii_lowercase()) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            used.insert(candidate.to_ascii_lowercase());
            candidate
        })
        .collect()
}

/// The sorted distinct values.
fn sorted<'a>(values: impl Iterator<Item = &'a Arc<str>>) -> Vec<&'a Arc<str>> {
    let mut out: Vec<&Arc<str>> = values.collect();
    out.sort();
    out.dedup();
    out
}

/// Writes an OCEL 2.0 log to a SQLite database (pm4py's
/// `write_ocel2_sqlite`, `ocel20` variant), replacing any file at `path`.
/// A name that does not end in `sqlite` gets `.sqlite` added.
///
/// The database holds the tables [`read_ocel2_sqlite`] reads. Each event
/// type's table holds `ocel_id`, `ocel_time`, `ocel:activity` and the
/// attributes with a value among its events. Each object type's table holds
/// `ocel_id` and the attributes with a value among its objects, then, when
/// the type has changes, one row per change with `ocel_changed_field`,
/// `ocel_time` and the changed value. A type's table name is its name with
/// each word capitalized and only ASCII letters and digits kept. The writer
/// first runs [`Ocel::make_consistent`] and [`Ocel::retain_related`] on a
/// copy, as pm4py's does. Event-to-event relations are not written. A list
/// or container attribute is an error.
pub fn write_ocel2_sqlite(ocel: &Ocel, path: impl AsRef<Path>) -> Result<()> {
    let p = Prepared::new(ocel);
    let log = &p.ocel;
    let mut tables = Vec::new();

    let mut event = Out::new("event");
    event.push_text("ocel_id", log.events.iter().map(|e| &*e.id));
    event.push_text("ocel_type", log.events.iter().map(|e| &*e.activity));
    event.dedup();
    let mut object = Out::new("object");
    object.push_text("ocel_id", log.objects.iter().map(|o| &*o.id));
    object.push_text("ocel_type", log.objects.iter().map(|o| &*o.object_type));
    object.dedup();
    let activities = sorted(log.events.iter().map(|e| &e.activity));
    let object_types = sorted(log.objects.iter().map(|o| &o.object_type));
    let event_names = table_names(&activities);
    let object_names = table_names(&object_types);
    tables.extend([event, object]);
    for (name, types, names) in [
        ("event_map_type", &activities, &event_names),
        ("object_map_type", &object_types, &object_names),
    ] {
        let mut map = Out::new(name);
        map.push_text("ocel_type", types.iter().map(|t| &***t));
        map.push_text("ocel_type_map", names.iter().map(String::as_str));
        tables.push(map);
    }
    let mut e2o = Out::new("event_object");
    let rel = &log.relations;
    e2o.push_text("ocel_event_id", rel.iter().map(|r| &*r.event));
    e2o.push_text("ocel_object_id", rel.iter().map(|r| &*r.object));
    e2o.push_text(
        "ocel_qualifier",
        rel.iter().map(|r| r.qualifier.as_deref().unwrap_or("")),
    );
    let mut o2o = Out::new("object_object");
    o2o.push_text("ocel_source_id", log.o2o.iter().map(|r| &*r.source));
    o2o.push_text("ocel_target_id", log.o2o.iter().map(|r| &*r.target));
    o2o.push_text(
        "ocel_qualifier",
        log.o2o.iter().map(|r| r.qualifier.as_deref().unwrap_or("")),
    );
    tables.extend([e2o, o2o]);

    for (activity, name) in activities.iter().zip(&event_names) {
        let rows: Vec<&OcelEvent> = log
            .events
            .iter()
            .filter(|e| e.activity == **activity)
            .collect();
        let mut table = Out::new(format!("event_{name}"));
        table.push_text("ocel_id", rows.iter().map(|e| &*e.id));
        let times = rows
            .iter()
            .map(|e| Value::Text(py_datetime(&e.timestamp)))
            .collect();
        table.push("ocel_time", ("TIMESTAMP", times));
        table.push_text(EVENT_ACTIVITY, rows.iter().map(|e| &*e.activity));
        for attribute in &p.events.names {
            let values: Vec<_> = rows.iter().map(|e| e.attributes.get(attribute)).collect();
            if values.iter().flatten().all(|v| missing(v)) {
                continue;
            }
            table.push(
                attribute,
                column(p.events.kind(attribute).into(), &values, true)?,
            );
        }
        table.dedup();
        tables.push(table);
    }

    for (object_type, name) in object_types.iter().zip(&object_names) {
        tables.push(object_table(&p, object_type, name)?);
    }
    create(&sqlite_path(path.as_ref()), &tables)
}

/// The table of one object type: its objects, then its changes, joined as
/// pandas' `concat` joins them.
fn object_table(p: &Prepared, object_type: &Arc<str>, name: &str) -> Result<Out> {
    let log = &p.ocel;
    let objects: Vec<&OcelObject> = log
        .objects
        .iter()
        .filter(|o| o.object_type == *object_type)
        .collect();
    let changes: Vec<(&ObjectChange, &Attributes)> = log
        .object_changes
        .iter()
        .zip(&p.change_rows)
        .filter(|(c, _)| c.object_type == *object_type)
        .collect();
    let has_value =
        |values: &[Option<&AttributeValue>]| !values.iter().flatten().all(|v| missing(v));

    // Per column: its part types (objects, changes) and values per row.
    type Part<'a> = (
        Option<Dtype>,
        Option<Dtype>,
        Vec<Option<&'a AttributeValue>>,
    );
    let mut columns: Vec<(&str, Part)> = Vec::new();
    for attribute in &p.objects.names {
        let values: Vec<_> = objects
            .iter()
            .map(|o| o.attributes.get(attribute))
            .collect();
        if has_value(&values) {
            columns.push((
                attribute,
                (Some(p.objects.kind(attribute).into()), None, values),
            ));
        }
    }
    let mut ids: Vec<&str> = objects.iter().map(|o| &*o.id).collect();
    let mut fields = Vec::new();
    let mut times = Vec::new();
    if !changes.is_empty() {
        for (c, _) in &changes {
            ids.push(&c.object);
            fields.push(Value::Text(c.field.to_string()));
            times.push(Value::Text(py_datetime(&c.timestamp)));
        }
        for (_, (_, _, values)) in &mut columns {
            values.extend(std::iter::repeat_n(None, changes.len()));
        }
        for attribute in &p.changes.names {
            let values: Vec<_> = changes.iter().map(|(_, r)| r.get(attribute)).collect();
            if !has_value(&values) {
                continue;
            }
            let dtype = Some(p.changes.kind(attribute).into());
            match columns.iter_mut().find(|(n, _)| *n == &**attribute) {
                Some((_, (_, part, all))) => {
                    *part = dtype;
                    all.truncate(objects.len());
                    all.extend(values);
                }
                None => {
                    let mut all = vec![None; objects.len()];
                    all.extend(values);
                    columns.push((attribute, (None, dtype, all)));
                }
            }
        }
    }

    let mut table = Out::new(format!("object_{name}"));
    table.push_text("ocel_id", ids.into_iter());
    let mut changed_columns = Vec::new();
    if !changes.is_empty() {
        let pad = |values: Vec<Value>| {
            let mut all = vec![Value::Null; objects.len()];
            all.extend(values);
            all
        };
        changed_columns.push(("ocel_changed_field", ("TEXT", pad(fields))));
        changed_columns.push(("ocel_time", ("TIMESTAMP", pad(times))));
    }
    let (from_objects, from_changes): (Vec<_>, Vec<_>) =
        columns.into_iter().partition(|(_, (o, _, _))| o.is_some());
    for (attribute, (o, c, values)) in from_objects {
        let dtype = if changes.is_empty() { o } else { combine(o, c) };
        table.push(
            attribute,
            column(dtype.unwrap_or(Dtype::Empty), &values, true)?,
        );
    }
    for (attribute, typed) in changed_columns {
        table.push(attribute, typed);
    }
    for (attribute, (o, c, values)) in from_changes {
        let dtype = combine(o, c).unwrap_or(Dtype::Empty);
        table.push(attribute, column(dtype, &values, true)?);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::{Dtype, combine, strip_escape_suffix, strip_name, table_names};
    use std::sync::Arc;

    #[test]
    fn names_follow_pm4py() {
        assert_eq!(strip_name("place order"), "PlaceOrder");
        assert_eq!(strip_name("ocel:type:item"), "Oceltypeitem");
        assert_eq!(strip_name("it\u{20ac}m"), "Itm");
        assert_eq!(strip_name("\u{df}a b"), "SsaB");
        let types: Vec<Arc<str>> = ["a b", "A-B", "ab", "\u{20ac}"]
            .into_iter()
            .map(Arc::from)
            .collect();
        let refs: Vec<&Arc<str>> = types.iter().collect();
        assert_eq!(table_names(&refs), ["AB", "Ab_2", "Ab_3", "Type"]);
    }

    #[test]
    fn escape_suffix() {
        let s = |t: &str| strip_escape_suffix(t.into());
        assert_eq!(s("e\\x0"), "e");
        assert_eq!(s("e\\x0\n"), "e\n");
        assert_eq!(s("1.0"), "1.0");
        assert_eq!(s("\\\n0"), "\\\n0");
    }

    #[test]
    fn concat_types() {
        use Dtype::*;
        assert_eq!(combine(Some(Int), None), Some(Float));
        assert_eq!(combine(Some(Int), Some(Bool)), Some(Int));
        assert_eq!(combine(Some(Float), Some(Bool)), Some(Float));
        assert_eq!(combine(Some(Empty), Some(Int)), Some(Object));
        assert_eq!(combine(Some(Str), None), Some(Str));
        assert_eq!(combine(Some(Bool), None), Some(Object));
        assert_eq!(combine(Some(Str), Some(Float)), Some(Object));
    }
}
