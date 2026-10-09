//! The Arrow columnar view: conversion between logs and `RecordBatch`es.
//!
//! The table has one row per event, in log order. Event attributes become
//! columns of the same name; trace attributes become columns named with the
//! case prefix (`case:concept:name`, ...). This is the table pm4py builds with
//! `convert_to_dataframe`.

use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayBuilder, ArrayRef, AsArray, BooleanBuilder, Float64Builder, Int64Builder,
    StringBuilder, TimestampNanosecondBuilder,
};
use arrow::compute::{CastOptions, cast_with_options};
use arrow::datatypes::{
    DataType, Field, Float64Type, Int64Type, Schema, TimeUnit, TimestampNanosecondType,
};
use arrow::record_batch::{RecordBatch, RecordBatchOptions};
use chrono::{DateTime, FixedOffset};
use rustc_hash::FxHashMap;

use crate::attribute::{AttributeValue, Attributes};
use crate::error::{Error, Result};
use crate::keys::{CONCEPT_NAME, EventKeys};
use crate::log::{Event, EventLog, EventStream, Extension, XesExtension};

/// Field metadata key that records the XES type of a column when Arrow alone
/// cannot. Its only value is `id`, set on string columns that hold XES IDs.
pub const XES_TYPE_METADATA_KEY: &str = "ichnos:xes-type";

impl EventLog {
    /// The log as an Arrow table, one row per event. Trace attributes become
    /// columns prefixed with `keys.case_prefix`, except the case ID (trace
    /// `concept:name`), which goes in column `keys.case_id`, so
    /// [`from_arrow`](Self::from_arrow) with the same keys reads the table
    /// back. See [`EventStream::to_arrow`] for how values map to Arrow types.
    ///
    /// The table holds traces and events only. Log attributes, extensions,
    /// globals and classifiers are not written; `from_arrow` declares
    /// extensions again from the column names.
    pub fn to_arrow(&self, keys: &EventKeys) -> Result<RecordBatch> {
        let rows: Vec<Row<'_>> = self
            .traces
            .iter()
            .flat_map(|t| {
                t.events.iter().map(|e| Row {
                    event: &e.attributes,
                    case: Some(&t.attributes),
                })
            })
            .collect();
        rows_to_batch(&rows, &keys.case_prefix, &keys.case_id)
    }

    /// Builds a log from an Arrow table: rows become events, grouped into
    /// traces by `keys.case_id` as [`EventStream::into_event_log`] describes.
    /// See [`EventStream::from_arrow`] for how Arrow types map to values.
    ///
    /// If `keys.case_id` starts with the case prefix (for example `case:id`),
    /// that column also becomes a trace attribute (`id`) holding the case ID,
    /// as in pm4py's `to_event_log`.
    pub fn from_arrow(batch: &RecordBatch, keys: &EventKeys) -> Result<Self> {
        EventStream::from_arrow(batch)?.into_event_log(keys)
    }

    /// Like [`from_arrow`](Self::from_arrow) for a table split into batches,
    /// as Parquet readers return it.
    pub fn from_arrow_batches<'a>(
        batches: impl IntoIterator<Item = &'a RecordBatch>,
        keys: &EventKeys,
    ) -> Result<Self> {
        EventStream::from_arrow_batches(batches)?.into_event_log(keys)
    }
}

impl EventStream {
    /// The stream as an Arrow table, one row per event, one column per
    /// attribute key in order of first appearance. A missing attribute is a
    /// null.
    ///
    /// Column types follow the values: `Utf8` for strings and IDs (IDs are
    /// marked with [`XES_TYPE_METADATA_KEY`]), `Int64`, `Float64` (ints and
    /// floats mixed), `Boolean`, and `Timestamp(Nanosecond, tz)` for dates.
    /// The time zone is the dates' common offset, or `UTC` if they differ.
    /// Any other mix of types becomes `Utf8`, with values formatted as
    /// Python's `str()` would. Meta-attributes are dropped. Lists and
    /// containers are an error.
    pub fn to_arrow(&self) -> Result<RecordBatch> {
        let rows: Vec<Row<'_>> = self
            .events
            .iter()
            .map(|e| Row {
                event: &e.attributes,
                case: None,
            })
            .collect();
        rows_to_batch(&rows, "", "")
    }

    /// Builds a stream from an Arrow table, one event per row, attributes in
    /// column order. Nulls are skipped. Standard XES extensions are declared
    /// for the prefixes found in the column names, as pm4py does.
    ///
    /// Strings (including large, view and dictionary-encoded) become strings,
    /// or IDs if marked with [`XES_TYPE_METADATA_KEY`]. Signed and unsigned
    /// integers become ints (an unsigned value above `i64::MAX` is an error).
    /// Floats and decimals become floats. Timestamps of any unit become dates
    /// with the column's fixed offset; a missing or named time zone is read as
    /// UTC. `Date32` and `Date64` become midnight UTC. Other types are an
    /// error.
    pub fn from_arrow(batch: &RecordBatch) -> Result<Self> {
        Self::from_arrow_batches([batch])
    }

    /// Like [`from_arrow`](Self::from_arrow) for a table split into batches.
    pub fn from_arrow_batches<'a>(
        batches: impl IntoIterator<Item = &'a RecordBatch>,
    ) -> Result<Self> {
        let mut stream = Self::new();
        let mut names: HashMap<String, Arc<str>> = HashMap::new();
        for batch in batches {
            let base = stream.events.len();
            let width = batch.num_columns();
            stream.events.extend(
                (0..batch.num_rows()).map(|_| Event::from(Attributes::with_capacity(width))),
            );
            let schema = batch.schema();
            for (field, column) in schema.fields().iter().zip(batch.columns()) {
                let name = names
                    .entry(field.name().clone())
                    .or_insert_with(|| {
                        declare_extensions(field.name(), &mut stream.extensions);
                        Arc::from(field.name().as_str())
                    })
                    .clone();
                decode_column(field, column, &name, &mut stream.events[base..])?;
            }
        }
        Ok(stream)
    }
}

impl TryFrom<&RecordBatch> for EventLog {
    type Error = Error;

    /// [`EventLog::from_arrow`] with the default keys.
    fn try_from(batch: &RecordBatch) -> Result<Self> {
        Self::from_arrow(batch, &EventKeys::default())
    }
}

impl TryFrom<&EventLog> for RecordBatch {
    type Error = Error;

    /// [`EventLog::to_arrow`] with the default keys.
    fn try_from(log: &EventLog) -> Result<Self> {
        log.to_arrow(&EventKeys::default())
    }
}

impl TryFrom<&RecordBatch> for EventStream {
    type Error = Error;

    fn try_from(batch: &RecordBatch) -> Result<Self> {
        Self::from_arrow(batch)
    }
}

impl TryFrom<&EventStream> for RecordBatch {
    type Error = Error;

    fn try_from(stream: &EventStream) -> Result<Self> {
        stream.to_arrow()
    }
}

/// Adds the standard extensions whose prefix is a `:`-separated part of `name`.
fn declare_extensions(name: &str, extensions: &mut Vec<Extension>) {
    for part in name.split(':') {
        if let Some(ext) = XesExtension::from_prefix(part)
            && !extensions.iter().any(|e| e.prefix == ext.prefix())
        {
            extensions.push(ext.into());
        }
    }
}

// ---------------------------------------------------------------------------
// Log to Arrow.

/// One output row: the event's attributes and, for a log, its trace's.
struct Row<'a> {
    event: &'a Attributes,
    case: Option<&'a Attributes>,
}

/// The Arrow type a column needs, widened as values are seen.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Bool,
    Int,
    Float,
    /// Dates; `Some` while every date so far has this offset.
    Date(Option<FixedOffset>),
    Str,
    Id,
}

impl Kind {
    fn of(key: &str, value: &AttributeValue) -> Result<Self> {
        Ok(match value.plain() {
            AttributeValue::String(_) => Self::Str,
            AttributeValue::Id(_) => Self::Id,
            AttributeValue::Int(_) => Self::Int,
            AttributeValue::Float(_) => Self::Float,
            AttributeValue::Bool(_) => Self::Bool,
            AttributeValue::Date(d) => Self::Date(Some(*d.offset())),
            other => {
                return Err(Error::NestedAttribute {
                    key: key.to_owned(),
                    kind: other.type_name(),
                });
            }
        })
    }

    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (a, b) if a == b => a,
            (Self::Int, Self::Float) | (Self::Float, Self::Int) => Self::Float,
            // Unequal offsets: fall back to UTC.
            (Self::Date(_), Self::Date(_)) => Self::Date(None),
            _ => Self::Str,
        }
    }
}

/// A column being built: its name, kind and position by name.
struct Columns {
    names: Vec<Arc<str>>,
    kinds: Vec<Option<Kind>>,
    by_name: FxHashMap<Arc<str>, usize>,
    /// Trace key to prefixed column name, so each prefix is formatted once.
    prefixed: FxHashMap<Arc<str>, Arc<str>>,
}

impl Columns {
    fn column(&mut self, name: &Arc<str>) -> usize {
        if let Some(&i) = self.by_name.get(name) {
            return i;
        }
        self.names.push(name.clone());
        self.kinds.push(None);
        self.by_name.insert(name.clone(), self.names.len() - 1);
        self.names.len() - 1
    }

    /// The column of trace attribute `key`: `case_id` for the case ID,
    /// `prefix` + `key` for the others.
    fn prefixed(&mut self, prefix: &str, case_id: &str, key: &Arc<str>) -> Arc<str> {
        self.prefixed
            .entry(key.clone())
            .or_insert_with(|| {
                if key.as_ref() == CONCEPT_NAME {
                    case_id.into()
                } else {
                    format!("{prefix}{key}").into()
                }
            })
            .clone()
    }
}

/// Whether event attribute `key` is overwritten by a trace attribute in the
/// flat table (pm4py lets the trace attribute win).
fn shadowed(key: &str, prefix: &str, case_id: &str, case: Option<&Attributes>) -> bool {
    case.is_some_and(|case| {
        if key == case_id {
            case.contains_key(CONCEPT_NAME)
        } else {
            key.strip_prefix(prefix)
                .is_some_and(|rest| rest != CONCEPT_NAME && case.contains_key(rest))
        }
    })
}

/// Whether trace attribute `key`, written to column `name`, collides with
/// the case ID column. The case ID wins.
fn collides_with_case_id(key: &str, name: &str, case_id: &str, row: &Row<'_>) -> bool {
    key != CONCEPT_NAME
        && name == case_id
        && row.case.is_some_and(|case| case.contains_key(CONCEPT_NAME))
}

fn rows_to_batch(rows: &[Row<'_>], prefix: &str, case_id: &str) -> Result<RecordBatch> {
    let mut cols = Columns {
        names: Vec::new(),
        kinds: Vec::new(),
        by_name: FxHashMap::default(),
        prefixed: FxHashMap::default(),
    };

    // Pass 1: column order and types.
    for row in rows {
        for (key, value) in row.event {
            let i = cols.column(key);
            if !shadowed(key, prefix, case_id, row.case) {
                let kind = Kind::of(key, value)?;
                cols.kinds[i] = Some(cols.kinds[i].map_or(kind, |k| k.merge(kind)));
            }
        }
        for (key, value) in row.case.into_iter().flatten() {
            let name = cols.prefixed(prefix, case_id, key);
            if collides_with_case_id(key, &name, case_id, row) {
                continue;
            }
            let i = cols.column(&name);
            let kind = Kind::of(&name, value)?;
            cols.kinds[i] = Some(cols.kinds[i].map_or(kind, |k| k.merge(kind)));
        }
    }

    // Pass 2: values.
    let mut builders: Vec<Builder> = cols
        .kinds
        .iter()
        .map(|k| Builder::new(k.unwrap_or(Kind::Str), rows.len()))
        .collect();
    for (r, row) in rows.iter().enumerate() {
        for (key, value) in row.event {
            if !shadowed(key, prefix, case_id, row.case) {
                builders[cols.by_name[key]].push(r, key, value)?;
            }
        }
        for (key, value) in row.case.into_iter().flatten() {
            let name = &cols.prefixed[key];
            if collides_with_case_id(key, name, case_id, row) {
                continue;
            }
            builders[cols.by_name[name]].push(r, name, value)?;
        }
    }

    let mut fields = Vec::with_capacity(builders.len());
    let mut arrays = Vec::with_capacity(builders.len());
    for ((name, kind), builder) in cols.names.iter().zip(&cols.kinds).zip(builders) {
        let array = builder.finish(rows.len());
        let mut field = Field::new(name.as_ref(), array.data_type().clone(), true);
        if *kind == Some(Kind::Id) {
            field = field.with_metadata(HashMap::from([(
                XES_TYPE_METADATA_KEY.to_owned(),
                "id".to_owned(),
            )]));
        }
        fields.push(field);
        arrays.push(array);
    }
    let options = RecordBatchOptions::new().with_row_count(Some(rows.len()));
    Ok(RecordBatch::try_new_with_options(
        Arc::new(Schema::new(fields)),
        arrays,
        &options,
    )?)
}

/// The Arrow time zone for a column of dates with this common offset.
fn timezone(offset: Option<FixedOffset>) -> Arc<str> {
    match offset {
        Some(o) if o.local_minus_utc() != 0 => o.to_string().into(),
        _ => "UTC".into(),
    }
}

enum Builder {
    Bool(BooleanBuilder),
    Int(Int64Builder),
    Float(Float64Builder),
    Date(TimestampNanosecondBuilder),
    Str(StringBuilder),
}

impl Builder {
    fn new(kind: Kind, rows: usize) -> Self {
        match kind {
            Kind::Bool => Self::Bool(BooleanBuilder::with_capacity(rows)),
            Kind::Int => Self::Int(Int64Builder::with_capacity(rows)),
            Kind::Float => Self::Float(Float64Builder::with_capacity(rows)),
            Kind::Date(offset) => Self::Date(
                TimestampNanosecondBuilder::with_capacity(rows).with_timezone(timezone(offset)),
            ),
            Kind::Str | Kind::Id => Self::Str(StringBuilder::with_capacity(rows, rows * 8)),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Bool(b) => b.len(),
            Self::Int(b) => b.len(),
            Self::Float(b) => b.len(),
            Self::Date(b) => b.len(),
            Self::Str(b) => b.len(),
        }
    }

    /// Appends nulls until the column has `rows` rows.
    fn pad(&mut self, rows: usize) {
        let missing = rows - self.len();
        match self {
            Self::Bool(b) => b.append_nulls(missing),
            Self::Int(b) => b.append_nulls(missing),
            Self::Float(b) => b.append_nulls(missing),
            Self::Date(b) => b.append_nulls(missing),
            Self::Str(b) => {
                for _ in 0..missing {
                    b.append_null();
                }
            }
        }
    }

    /// Sets row `row` to `value`. The kind pass guarantees the value fits.
    fn push(&mut self, row: usize, key: &str, value: &AttributeValue) -> Result<()> {
        self.pad(row);
        let value = value.plain();
        match (self, value) {
            (Self::Bool(b), AttributeValue::Bool(v)) => b.append_value(*v),
            (Self::Int(b), AttributeValue::Int(v)) => b.append_value(*v),
            (Self::Float(b), v) => b.append_option(v.as_f64()),
            (Self::Date(b), AttributeValue::Date(d)) => {
                let ns = d
                    .timestamp_nanos_opt()
                    .ok_or_else(|| Error::TimestampOutOfRange {
                        key: key.to_owned(),
                        value: d.to_rfc3339(),
                    })?;
                b.append_value(ns);
            }
            (Self::Str(b), AttributeValue::String(s) | AttributeValue::Id(s)) => b.append_value(s),
            (Self::Str(b), v) => b.append_value(v.to_string()),
            (_, v) => unreachable!("column kind admits {}", v.type_name()),
        }
        Ok(())
    }

    fn finish(mut self, rows: usize) -> ArrayRef {
        self.pad(rows);
        match self {
            Self::Bool(mut b) => Arc::new(b.finish()),
            Self::Int(mut b) => Arc::new(b.finish()),
            Self::Float(mut b) => Arc::new(b.finish()),
            Self::Date(mut b) => Arc::new(b.finish()),
            Self::Str(mut b) => Arc::new(b.finish()),
        }
    }
}

// ---------------------------------------------------------------------------
// Arrow to log.

/// Parses an Arrow time zone. Fixed offsets keep their offset; anything else
/// (none, `UTC`, a named zone) reads as UTC. Arrow stores UTC instants, so
/// the instant is right in every case.
fn parse_offset(tz: Option<&str>) -> FixedOffset {
    let utc = FixedOffset::east_opt(0).expect("zero offset");
    let Some(tz) = tz else { return utc };
    let Some(sign) = tz.chars().next().filter(|c| *c == '+' || *c == '-') else {
        return utc;
    };
    let digits: String = tz[1..].chars().filter(char::is_ascii_digit).collect();
    let (hours, minutes) = match digits.len() {
        2 => (&digits[..2], "0"),
        4 => (&digits[..2], &digits[2..]),
        _ => return utc,
    };
    let (Ok(h), Ok(m)) = (hours.parse::<i32>(), minutes.parse::<i32>()) else {
        return utc;
    };
    let seconds = (h * 3600 + m * 60) * if sign == '-' { -1 } else { 1 };
    FixedOffset::east_opt(seconds).unwrap_or(utc)
}

fn decode_column(
    field: &Field,
    column: &ArrayRef,
    name: &Arc<str>,
    events: &mut [Event],
) -> Result<()> {
    let strict = CastOptions {
        safe: false,
        ..CastOptions::default()
    };
    let mut put = |row: usize, value: AttributeValue| {
        events[row].attributes.insert(name.clone(), value);
    };
    match column.data_type() {
        DataType::Null => {}
        DataType::Boolean => {
            for (row, v) in column.as_boolean().iter().enumerate() {
                if let Some(v) = v {
                    put(row, AttributeValue::Bool(v));
                }
            }
        }
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => {
            let ints = cast_with_options(column, &DataType::Int64, &strict)?;
            for (row, v) in ints.as_primitive::<Int64Type>().iter().enumerate() {
                if let Some(v) = v {
                    put(row, AttributeValue::Int(v));
                }
            }
        }
        DataType::Float16
        | DataType::Float32
        | DataType::Float64
        | DataType::Decimal128(..)
        | DataType::Decimal256(..) => {
            let floats = cast_with_options(column, &DataType::Float64, &strict)?;
            for (row, v) in floats.as_primitive::<Float64Type>().iter().enumerate() {
                if let Some(v) = v {
                    put(row, AttributeValue::Float(v));
                }
            }
        }
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => {
            let strings = cast_with_options(column, &DataType::Utf8, &strict)?;
            let is_id = field
                .metadata()
                .get(XES_TYPE_METADATA_KEY)
                .is_some_and(|t| t == "id");
            let mut shared: FxHashMap<&str, Arc<str>> = FxHashMap::default();
            for (row, v) in strings.as_string::<i32>().iter().enumerate() {
                if let Some(s) = v {
                    let s = shared.entry(s).or_insert_with(|| s.into()).clone();
                    put(
                        row,
                        if is_id {
                            AttributeValue::Id(s)
                        } else {
                            AttributeValue::String(s)
                        },
                    );
                }
            }
        }
        DataType::Dictionary(_, values) => {
            let plain = cast_with_options(column, values, &strict)?;
            decode_column(field, &plain, name, events)?;
        }
        DataType::Timestamp(_, tz) => {
            let offset = parse_offset(tz.as_deref());
            let target = DataType::Timestamp(TimeUnit::Nanosecond, tz.clone());
            decode_timestamps(&cast_with_options(column, &target, &strict)?, offset, put);
        }
        DataType::Date32 | DataType::Date64 => {
            let target = DataType::Timestamp(TimeUnit::Nanosecond, None);
            decode_timestamps(
                &cast_with_options(column, &target, &strict)?,
                parse_offset(None),
                put,
            );
        }
        other => {
            return Err(Error::UnsupportedColumn {
                column: field.name().clone(),
                data_type: other.to_string(),
            });
        }
    }
    Ok(())
}

fn decode_timestamps(
    column: &ArrayRef,
    offset: FixedOffset,
    mut put: impl FnMut(usize, AttributeValue),
) {
    for (row, v) in column
        .as_primitive::<TimestampNanosecondType>()
        .iter()
        .enumerate()
    {
        if let Some(ns) = v {
            let date = DateTime::from_timestamp_nanos(ns).with_timezone(&offset);
            put(row, AttributeValue::Date(date));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{
        DictionaryArray, Int32Array, StringArray, TimestampMillisecondArray, UInt64Array,
    };
    use arrow::datatypes::Int32Type;
    use chrono::TimeZone;

    use crate::log::{Classifier, Trace};

    fn sample_log() -> EventLog {
        let plus2 = FixedOffset::east_opt(7200).unwrap();
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B", "C"], ",", &keys);
        log.traces[0].attributes.insert("region", "north");
        log.traces[1].attributes.insert("region", "south");
        log.traces[1].attributes.insert("vip", true);
        let events = &mut log.traces[0].events;
        events[0].insert("cost", 3);
        events[1].insert("cost", 2.5);
        events[0].insert("org:resource", AttributeValue::id("r-1"));
        events[1].insert("due", plus2.with_ymd_and_hms(2024, 5, 1, 12, 0, 0).unwrap());
        log.traces[1].events[0].insert("cost", 1);
        log.extensions.push(XesExtension::Concept.into());
        log.extensions.push(XesExtension::Time.into());
        log.extensions.push(XesExtension::Organizational.into());
        log
    }

    #[test]
    fn round_trip_keeps_values_and_types() {
        let keys = EventKeys::default();
        let log = sample_log();
        let batch = log.to_arrow(&keys).unwrap();
        assert_eq!(batch.num_rows(), 3);
        let schema = batch.schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            names,
            [
                "concept:name",
                "time:timestamp",
                "cost",
                "org:resource",
                "case:concept:name",
                "case:region",
                "due",
                "case:vip",
            ]
        );
        assert_eq!(schema.field(2).data_type(), &DataType::Float64);
        assert_eq!(
            schema.field(1).data_type(),
            &DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
        );
        assert_eq!(
            schema.field(6).data_type(),
            &DataType::Timestamp(TimeUnit::Nanosecond, Some("+02:00".into()))
        );

        let mut expected = log.clone();
        // Ints in a mixed int/float column come back as floats.
        expected.traces[0].events[0].insert("cost", 3.0);
        expected.traces[1].events[0].insert("cost", 1.0);
        let back = EventLog::from_arrow(&batch, &keys).unwrap();
        assert_eq!(back.traces, expected.traces);
        // Column `cost` also declares the XES Cost extension, as in pm4py.
        let prefixes: Vec<&str> = back.extensions.iter().map(|e| e.prefix.as_str()).collect();
        assert_eq!(prefixes, ["concept", "time", "cost", "org"]);
        let due = back.traces[0].events[1]
            .get("due")
            .unwrap()
            .as_date()
            .unwrap();
        assert_eq!(due.offset().local_minus_utc(), 7200);
        assert!(matches!(
            back.traces[0].events[0].get("org:resource"),
            Some(AttributeValue::Id(_))
        ));
    }

    #[test]
    fn round_trip_drops_log_metadata() {
        let keys = EventKeys::default();
        let mut log = sample_log();
        log.attributes.insert("source", "test");
        log.classifiers.push(Classifier {
            name: "Activity".into(),
            keys: vec!["concept:name".into()],
        });
        let back = EventLog::from_arrow(&log.to_arrow(&keys).unwrap(), &keys).unwrap();
        assert_eq!(back.traces.len(), log.traces.len());
        assert!(back.attributes.is_empty());
        assert!(back.classifiers.is_empty());
    }

    #[test]
    fn round_trip_with_custom_case_id_column() {
        let keys = EventKeys::default().with_case_id("case:id");
        let mut log = sample_log();
        log.traces[0].attributes.insert("id", "hidden");
        log.traces[0].events[0].insert("case:id", "hidden too");
        let batch = log.to_arrow(&keys).unwrap();
        let schema = batch.schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&"case:id"));
        assert!(!names.contains(&"case:concept:name"));
        let ids = batch
            .column(schema.index_of("case:id").unwrap())
            .as_string::<i32>();
        assert_eq!(ids.value(0), "0");
        assert_eq!(ids.value(2), "1");

        let back = EventLog::from_arrow(&batch, &keys).unwrap();
        let case_ids: Vec<_> = back
            .iter()
            .map(|t| t.case_id().unwrap().to_string())
            .collect();
        assert_eq!(case_ids, ["0", "1"]);
        assert_eq!(back.num_events(), log.num_events());
    }

    #[test]
    fn trace_attribute_wins_over_prefixed_event_attribute() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_traces(vec![Trace::with_case_id("c1")]);
        log.traces[0]
            .events
            .push(Event::from_iter([("case:concept:name", 5)]));
        let batch = log.to_arrow(&keys).unwrap();
        assert_eq!(batch.num_columns(), 1);
        let col = batch.column(0).as_string::<i32>();
        assert_eq!(col.value(0), "c1");
    }

    #[test]
    fn mixed_types_become_strings_and_nested_values_fail() {
        let stream = EventStream {
            events: vec![
                Event::from_iter([("x", AttributeValue::from(1))]),
                Event::from_iter([("x", AttributeValue::from(true))]),
                Event::new(),
            ],
            ..EventStream::default()
        };
        let batch = stream.to_arrow().unwrap();
        let col = batch.column(0).as_string::<i32>();
        assert_eq!(
            (col.value(0), col.value(1), col.is_null(2)),
            ("1", "True", true)
        );

        let nested = EventStream {
            events: vec![Event::from_iter([(
                "l",
                AttributeValue::List(vec![("item".into(), 1.into())]),
            )])],
            ..EventStream::default()
        };
        assert!(matches!(
            nested.to_arrow(),
            Err(Error::NestedAttribute { kind: "list", .. })
        ));
    }

    #[test]
    fn empty_stream_gives_empty_batch() {
        let batch = EventStream::new().to_arrow().unwrap();
        assert_eq!((batch.num_rows(), batch.num_columns()), (0, 0));
        let stream = EventStream {
            events: vec![Event::new(), Event::new()],
            ..EventStream::default()
        };
        assert_eq!(stream.to_arrow().unwrap().num_rows(), 2);
    }

    #[test]
    fn from_arrow_accepts_other_arrow_types() {
        let dict: DictionaryArray<Int32Type> = vec!["a", "b", "a"].into_iter().collect();
        let schema = Schema::new(vec![
            Field::new("case:concept:name", DataType::Utf8, false),
            Field::new("concept:name", dict.data_type().clone(), false),
            Field::new(
                "time:timestamp",
                DataType::Timestamp(TimeUnit::Millisecond, Some("Europe/Rome".into())),
                false,
            ),
            Field::new("n", DataType::UInt64, true),
            Field::new("small", DataType::Int32, true),
        ]);
        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(StringArray::from(vec!["1", "2", "1"])),
                Arc::new(dict),
                Arc::new(
                    TimestampMillisecondArray::from(vec![1_000, 2_000, 3_000])
                        .with_timezone("Europe/Rome"),
                ),
                Arc::new(UInt64Array::from(vec![Some(7), None, Some(9)])),
                Arc::new(Int32Array::from(vec![Some(-1), Some(2), None])),
            ],
        )
        .unwrap();
        let log = EventLog::try_from(&batch).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log.traces[0].len(), 2);
        let e = &log.traces[0].events[1];
        assert_eq!(e.get("concept:name").unwrap().as_str(), Some("a"));
        assert_eq!(e.get("n").unwrap().as_i64(), Some(9));
        assert!(e.get("small").is_none());
        let ts = e.get("time:timestamp").unwrap().as_date().unwrap();
        assert_eq!(ts.timestamp_millis(), 3_000);
        assert!(log.traces[1].events[0].get("n").is_none());
        let prefixes: Vec<&str> = log.extensions.iter().map(|e| e.prefix.as_str()).collect();
        assert_eq!(prefixes, ["concept", "time"]);

        let too_big = RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new("n", DataType::UInt64, false)])),
            vec![Arc::new(UInt64Array::from(vec![u64::MAX]))],
        )
        .unwrap();
        assert!(matches!(
            EventStream::from_arrow(&too_big),
            Err(Error::Arrow(_))
        ));
    }

    #[test]
    fn unsupported_column_type_is_an_error() {
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new("b", DataType::Binary, false)])),
            vec![Arc::new(arrow::array::BinaryArray::from(vec![
                b"x".as_ref(),
            ]))],
        )
        .unwrap();
        assert!(matches!(
            EventStream::from_arrow(&batch),
            Err(Error::UnsupportedColumn { .. })
        ));
    }

    #[test]
    fn batches_append() {
        let keys = EventKeys::default();
        let log = sample_log();
        let batch = log.to_arrow(&keys).unwrap();
        let both = EventLog::from_arrow_batches([&batch, &batch], &keys).unwrap();
        assert_eq!(both.len(), 2);
        assert_eq!(both.num_events(), 6);
    }

    #[test]
    fn offsets_parse() {
        assert_eq!(parse_offset(Some("+02:00")).local_minus_utc(), 7200);
        assert_eq!(parse_offset(Some("-0530")).local_minus_utc(), -19800);
        assert_eq!(parse_offset(Some("+01")).local_minus_utc(), 3600);
        assert_eq!(parse_offset(Some("UTC")).local_minus_utc(), 0);
        assert_eq!(parse_offset(None).local_minus_utc(), 0);
    }
}
