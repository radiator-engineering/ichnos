//! OCEL 1.0 as CSV (pm4py's `read_ocel_csv` and `write_ocel_csv`): one
//! row per event, with a column `ocel:type:<type>` per object type that
//! holds the related objects as a Python list, such as `['o1', 'o2']`. The
//! objects can come with a second table, with the columns `ocel:oid`,
//! `ocel:type` and the object attributes.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use ichnos_core::python::repr;
use ichnos_core::{AttributeValue, Attributes};
use ichnos_ocel::constants::{
    EVENT_ACTIVITY, EVENT_ID, EVENT_TIMESTAMP, OBJECT_ID, OBJECT_TYPE, OBJECT_TYPE_PREFIX_EXTENDED,
};
use ichnos_ocel::{EventObject, Ocel, OcelEvent, OcelObject};

use super::frame::{self, Frame};
use super::literal::{self, Item};
use super::open;
use super::write::{Columns, Kind, Prepared, as_float, missing, to_path};
use crate::error::{Error, Result};

fn error(detail: impl Into<String>) -> Error {
    Error::Ocel(detail.into())
}

/// Reads an OCEL 1.0 CSV file (pm4py's `read_ocel_csv`), with its objects
/// from `objects` when given. A path ending in `.gz` is decompressed.
///
/// The table is read as pandas reads it; see [`read_ocel_csv_from_reader`]
/// for how the values are read.
pub fn read_ocel_csv(path: impl AsRef<Path>, objects: Option<&Path>) -> Result<Ocel> {
    let table = open(path.as_ref())?;
    match objects {
        Some(objects) => {
            let mut objects = open(objects)?;
            read_ocel_csv_from_reader(table, Some(&mut objects))
        }
        None => read_ocel_csv_from_reader(table, None),
    }
}

/// Reads an OCEL 1.0 CSV table, with its objects from `objects` when given
/// (pm4py's `read_ocel_csv`).
///
/// - The columns `ocel:eid`, `ocel:activity` and `ocel:timestamp` must be
///   there. Every other column that does not start with `ocel:type:` holds
///   event attributes, kept as strings.
/// - The timestamps take the form of the first one, as pandas guesses it:
///   an ISO 8601 date, optionally with a time to the hour, minute, second or
///   fraction of a second, after `T` or a space, and optionally with an
///   offset. Every timestamp must have that same form and offset. A
///   timestamp without an offset is read as UTC.
/// - A cell of an `ocel:type:` column that starts with `[` and is a Python
///   list literal gives one relation per item, in order: each item must be
///   a string, a UTF-8 byte string or `None`, which is skipped. Any other
///   cell gives no relations. An event with relations must have an id and
///   an activity.
/// - Without an objects table, the objects are those of the relations, per
///   column in order of first appearance.
///
/// The events and relations are then sorted by timestamp, keeping file
/// order for ties, and [`Ocel::make_consistent`] runs. Unlike the other
/// readers, this one keeps events and objects without relations, as
/// pm4py's does.
pub fn read_ocel_csv_from_reader(table: impl Read, objects: Option<&mut dyn Read>) -> Result<Ocel> {
    let frame = frame::read(table)?;
    let column = |name: &str| {
        frame
            .column(name)
            .ok_or_else(|| error(format!("the CSV file has no {name} column")))
    };
    let (eid, activity, ts) = (
        column(EVENT_ID)?,
        column(EVENT_ACTIVITY)?,
        column(EVENT_TIMESTAMP)?,
    );
    // pandas' `ot.split(prefix)[1]`.
    let types: Vec<(usize, Arc<str>)> = frame
        .columns
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            let rest = name.strip_prefix(OBJECT_TYPE_PREFIX_EXTENDED)?;
            let object_type = rest.split(OBJECT_TYPE_PREFIX_EXTENDED).next().unwrap_or("");
            Some((i, Arc::from(object_type)))
        })
        .collect();
    let attributes: Vec<(usize, Arc<str>)> = frame
        .columns
        .iter()
        .enumerate()
        .filter(|(i, name)| {
            ![eid, activity, ts].contains(i) && !name.starts_with(OBJECT_TYPE_PREFIX_EXTENDED)
        })
        .map(|(i, name)| (i, Arc::from(name.as_str())))
        .collect();
    let times = timestamps(&frame, ts)?;

    let text =
        |row: &[Option<String>], i: usize| -> Arc<str> { row[i].as_deref().unwrap_or("").into() };
    let mut ocel = Ocel::new();
    let mut relations: Vec<(DateTime<FixedOffset>, EventObject)> = Vec::new();
    // Per object type column, the objects in first-appearance order. A byte
    // string and a string with the same text are two objects, as in
    // pm4py's set.
    let mut found: Vec<(Vec<Arc<str>>, HashSet<(bool, Arc<str>)>)> =
        vec![Default::default(); types.len()];
    for (r, row) in frame.rows.iter().enumerate() {
        ocel.events.push(OcelEvent {
            id: text(row, eid),
            activity: text(row, activity),
            timestamp: times[r],
            attributes: attributes
                .iter()
                .filter_map(|(i, name)| {
                    let value = row[*i].as_deref()?;
                    Some((name.clone(), AttributeValue::String(value.into())))
                })
                .collect::<Attributes>(),
        });
        for (t, (i, object_type)) in types.iter().enumerate() {
            let items = match row[*i].as_deref() {
                Some(cell) if cell.starts_with('[') => {
                    literal::parse_list(cell).unwrap_or_default()
                }
                _ => continue,
            };
            if items.is_empty() {
                continue;
            }
            if row[eid].is_none() || row[activity].is_none() {
                return Err(error(format!(
                    "CSV record {} has related objects but no event id or activity",
                    r + 2
                )));
            }
            for item in items {
                let (bytes, id): (bool, Arc<str>) = match item {
                    Item::Str(Some(s)) => (false, s.into()),
                    Item::Bytes(b) => (
                        true,
                        String::from_utf8(b)
                            .map_err(|_| error("an object id is a byte string that is not UTF-8"))?
                            .into(),
                    ),
                    Item::None => continue,
                    Item::Str(None) => {
                        return Err(error("an object id holds a lone surrogate"));
                    }
                    Item::NamedEscape => {
                        return Err(error(
                            "an object id uses a \\N{...} escape, which is not supported",
                        ));
                    }
                    Item::Other => return Err(error("an object id is not a string")),
                };
                let (order, seen) = &mut found[t];
                if seen.insert((bytes, id.clone())) {
                    order.push(id.clone());
                }
                // pm4py's consistency step drops a relation of the empty
                // type.
                if !object_type.is_empty() {
                    relations.push((
                        times[r],
                        EventObject {
                            event: text(row, eid),
                            object: id,
                            qualifier: None,
                        },
                    ));
                }
            }
        }
    }
    ocel.objects = match objects {
        Some(input) => read_objects(frame::read(input)?)?,
        None => types
            .iter()
            .zip(found)
            .flat_map(|((_, object_type), (ids, _))| {
                ids.into_iter().map(move |id| OcelObject {
                    id,
                    object_type: object_type.clone(),
                    attributes: Attributes::default(),
                })
            })
            .collect(),
    };
    ocel.events.sort_by_key(|e| e.timestamp);
    relations.sort_by_key(|(t, _)| *t);
    ocel.relations = relations.into_iter().map(|(_, r)| r).collect();
    ocel.make_consistent();
    Ok(ocel)
}

/// The objects table: `ocel:oid`, `ocel:type` and attribute columns, kept as
/// strings.
fn read_objects(frame: Frame) -> Result<Vec<OcelObject>> {
    let column = |name: &str| {
        frame
            .column(name)
            .ok_or_else(|| error(format!("the objects CSV file has no {name} column")))
    };
    let (oid, object_type) = (column(OBJECT_ID)?, column(OBJECT_TYPE)?);
    Ok(frame
        .rows
        .iter()
        .map(|row| OcelObject {
            id: row[oid].as_deref().unwrap_or("").into(),
            object_type: row[object_type].as_deref().unwrap_or("").into(),
            attributes: frame
                .columns
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != oid && *i != object_type)
                .filter_map(|(i, name)| {
                    let value = row[i].as_deref()?;
                    Some((name.as_str(), AttributeValue::String(value.into())))
                })
                .collect(),
        })
        .collect())
}

/// The form of a timestamp: what pandas' format guess fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Form {
    /// The byte between date and time, if there is a time.
    pub(super) separator: Option<u8>,
    /// How many of hours, minutes and seconds the time gives.
    pub(super) parts: u8,
    /// Whether the seconds have a fraction.
    pub(super) fraction: bool,
    /// The offset in seconds, if there is one.
    pub(super) offset: Option<i32>,
}

/// Parses each row's timestamp, which must all have the first one's form.
fn timestamps(frame: &Frame, ts: usize) -> Result<Vec<DateTime<FixedOffset>>> {
    let mut first: Option<Form> = None;
    frame
        .rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            let text = row[ts]
                .as_deref()
                .ok_or_else(|| error(format!("CSV record {} has no timestamp", r + 2)))?;
            let (local, form) = parse_timestamp(text)
                .ok_or_else(|| error(format!("cannot read the timestamp {text:?}")))?;
            match first {
                None => first = Some(form),
                Some(f) if f == form => {}
                Some(_) => {
                    return Err(error(format!(
                        "the timestamp {text:?} does not have the form of the first one"
                    )));
                }
            }
            let offset = FixedOffset::east_opt(form.offset.unwrap_or(0))
                .ok_or_else(|| error(format!("cannot read the timestamp {text:?}")))?;
            offset
                .from_local_datetime(&local)
                .single()
                .ok_or_else(|| error(format!("cannot read the timestamp {text:?}")))
        })
        .collect()
}

/// `YYYY-MM-DD[{T| }HH[:MM[:SS[.f]]]][Z|±HH:MM|±HHMM]`, with up to nine
/// fraction digits.
pub(super) fn parse_timestamp(text: &str) -> Option<(NaiveDateTime, Form)> {
    let s = text.as_bytes();
    let mut pos = 0;
    let number = |n: usize, pos: &mut usize| -> Option<u32> {
        let part = s.get(*pos..*pos + n)?;
        if !part.iter().all(u8::is_ascii_digit) {
            return None;
        }
        *pos += n;
        Some(part.iter().fold(0, |v, c| v * 10 + u32::from(c - b'0')))
    };
    let year = number(4, &mut pos)?;
    let sep = |c: u8, pos: &mut usize| -> bool {
        if s.get(*pos) == Some(&c) {
            *pos += 1;
            true
        } else {
            false
        }
    };
    if !sep(b'-', &mut pos) {
        return None;
    }
    let month = number(2, &mut pos)?;
    if !sep(b'-', &mut pos) {
        return None;
    }
    let day = number(2, &mut pos)?;
    let date = NaiveDate::from_ymd_opt(year as i32, month, day)?;
    let mut form = Form {
        separator: None,
        parts: 0,
        fraction: false,
        offset: None,
    };
    let mut hms = [0u32; 3];
    let mut nanos = 0;
    if let Some(&c @ (b'T' | b' ')) = s.get(pos) {
        pos += 1;
        form.separator = Some(c);
        hms[0] = number(2, &mut pos)?;
        form.parts = 1;
        for part in 1..3 {
            if s.get(pos) != Some(&b':') {
                break;
            }
            pos += 1;
            hms[part] = number(2, &mut pos)?;
            form.parts += 1;
        }
        if form.parts == 3 && s.get(pos) == Some(&b'.') {
            pos += 1;
            let digits = s[pos..].iter().take_while(|c| c.is_ascii_digit()).count();
            if !(1..=9).contains(&digits) {
                return None;
            }
            let value = number(digits, &mut pos)?;
            nanos = value * 10u32.pow(9 - digits as u32);
            form.fraction = true;
        }
        match s.get(pos) {
            Some(b'Z') => {
                pos += 1;
                form.offset = Some(0);
            }
            Some(&c @ (b'+' | b'-')) => {
                pos += 1;
                let hours = number(2, &mut pos)?;
                sep(b':', &mut pos);
                let minutes = number(2, &mut pos)?;
                if hours > 23 || minutes > 59 {
                    return None;
                }
                let seconds = (hours * 3600 + minutes * 60) as i32;
                form.offset = Some(if c == b'-' { -seconds } else { seconds });
            }
            _ => {}
        }
    }
    if pos != s.len() {
        return None;
    }
    let time = NaiveTime::from_hms_nano_opt(hms[0], hms[1], hms[2], nanos)?;
    Some((date.and_time(time), form))
}

/// Writes an OCEL 1.0 CSV file (pm4py's `write_ocel_csv`), and the objects
/// to `objects_path` when given. A path ending in `.gz` is compressed.
///
/// See [`write_ocel_csv_to_writer`] for what is written.
pub fn write_ocel_csv(
    ocel: &Ocel,
    path: impl AsRef<Path>,
    objects_path: Option<&Path>,
) -> Result<()> {
    let prepared = Prepared::new(ocel);
    to_path(path.as_ref(), |out| write_table(&prepared, out))?;
    if let Some(objects_path) = objects_path {
        to_path(objects_path, |out| write_objects(&prepared, out))?;
    }
    Ok(())
}

/// Writes an OCEL 1.0 CSV table, and the objects table to `objects` when
/// given (pm4py's `write_ocel_csv`).
///
/// The log is first made consistent and limited to related events and
/// objects, as for the other writers. The table has the columns
/// `ocel:eid`, `ocel:activity`, `ocel:timestamp`, the event attributes and
/// one `ocel:type:<type>` column per object type, as in
/// [`Ocel::extended_table`]. The objects table has `ocel:oid`, `ocel:type`
/// and the object attributes. Values are written as pandas' `to_csv` writes
/// them: times as `2022-01-09 14:00:00+00:00`, with a fraction of the
/// second only when there is one, floats in Python's shortest form,
/// booleans as `True` and `False`, and lists of object ids as Python lists.
/// Fields are quoted as Python's `csv` module quotes them, and lines end
/// in `\n`.
pub fn write_ocel_csv_to_writer(
    ocel: &Ocel,
    mut table: impl Write,
    objects: Option<&mut dyn Write>,
) -> Result<()> {
    let prepared = Prepared::new(ocel);
    write_table(&prepared, &mut table)?;
    if let Some(objects) = objects {
        write_objects(&prepared, objects)?;
    }
    Ok(())
}

fn write_table(p: &Prepared, out: &mut dyn Write) -> Result<()> {
    let table = p.ocel.extended_table();
    let mut header: Vec<String> = [EVENT_ID, EVENT_ACTIVITY, EVENT_TIMESTAMP]
        .map(String::from)
        .into();
    header.extend(p.events.names.iter().map(|n| n.to_string()));
    header.extend(
        table
            .object_types
            .iter()
            .map(|t| format!("{OBJECT_TYPE_PREFIX_EXTENDED}{t}")),
    );
    write_record(out, &header, "\n")?;
    for row in &table.rows {
        let e = &p.ocel.events[row.event];
        let mut fields = vec![
            e.id.to_string(),
            e.activity.to_string(),
            timestamp_str(&e.timestamp),
        ];
        fields.extend(values(&e.attributes, &p.events));
        fields.extend(row.objects.iter().map(|ids| {
            if ids.is_empty() {
                String::new()
            } else {
                let items: Vec<String> = ids.iter().map(|id| repr(id)).collect();
                format!("[{}]", items.join(", "))
            }
        }));
        write_record(out, &fields, "\n")?;
    }
    Ok(())
}

fn write_objects(p: &Prepared, out: &mut dyn Write) -> Result<()> {
    let mut header: Vec<String> = vec![OBJECT_ID.into(), OBJECT_TYPE.into()];
    header.extend(p.objects.names.iter().map(|n| n.to_string()));
    write_record(out, &header, "\n")?;
    for o in &p.ocel.objects {
        let mut fields = vec![o.id.to_string(), o.object_type.to_string()];
        fields.extend(values(&o.attributes, &p.objects));
        write_record(out, &fields, "\n")?;
    }
    Ok(())
}

/// A row's attribute values in column order, as pandas' `to_csv` writes
/// them.
fn values<'a>(row: &'a Attributes, columns: &'a Columns) -> impl Iterator<Item = String> + 'a {
    columns.names.iter().map(move |name| {
        let Some(value) = row.get(name).filter(|v| !missing(v)) else {
            return String::new();
        };
        let kind = columns.kind(name);
        if kind == Kind::Float
            && let Some(f) = as_float(value)
        {
            return AttributeValue::Float(f).to_string();
        }
        match value.plain() {
            AttributeValue::Date(d) => timestamp_str(d),
            other => other.to_string(),
        }
    })
}

/// A time as pandas' `str(Timestamp)` writes it, in the time's own offset:
/// a fraction of the second only when there is one, with nine digits when
/// the time has nanoseconds.
fn timestamp_str(d: &DateTime<FixedOffset>) -> String {
    let nanos = d.nanosecond() % 1_000_000_000;
    let fraction = if !nanos.is_multiple_of(1_000) {
        format!(".{nanos:09}")
    } else if nanos != 0 {
        format!(".{:06}", nanos / 1_000)
    } else {
        String::new()
    };
    format!(
        "{}{fraction}{}",
        d.format("%Y-%m-%d %H:%M:%S"),
        d.format("%:z")
    )
}

/// Writes one record as Python's `csv.writer` does with `QUOTE_MINIMAL`: a
/// field with a comma, a quote or a line break is quoted, with its quotes
/// doubled, and a record of one empty field is written as `""`.
pub(super) fn write_record(out: &mut dyn Write, fields: &[String], end: &str) -> Result<()> {
    let mut line = String::new();
    if let [only] = fields
        && only.is_empty()
    {
        line.push_str("\"\"");
    }
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            line.push(',');
        }
        if field.contains([',', '"', '\n', '\r']) {
            line.push('"');
            line.push_str(&field.replace('"', "\"\""));
            line.push('"');
        } else {
            line.push_str(field);
        }
    }
    line.push_str(end);
    out.write_all(line.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_timestamp;

    #[test]
    fn timestamp_forms() {
        let form = |text: &str| {
            parse_timestamp(text).map(|(_, f)| (f.separator, f.parts, f.fraction, f.offset))
        };
        assert_eq!(form("2022-01-09"), Some((None, 0, false, None)));
        assert_eq!(form("2022-01-09 14"), Some((Some(b' '), 1, false, None)));
        assert_eq!(
            form("2022-01-09T14:00:00.5Z"),
            Some((Some(b'T'), 3, true, Some(0)))
        );
        assert_eq!(
            form("2022-01-09T14:00:00+0100"),
            Some((Some(b'T'), 3, false, Some(3600)))
        );
        assert_eq!(
            form("2022-01-09 14:00-00:30"),
            Some((Some(b' '), 2, false, Some(-1800)))
        );
        for bad in [
            "2022-02-30",
            "20/05/2019 07:07",
            "2022-01-09Z",
            "2022-01-09 24:00",
            "2022-01-09 14:00:00.1234567890",
            "yesterday",
        ] {
            assert_eq!(form(bad), None, "{bad}");
        }
    }
}
