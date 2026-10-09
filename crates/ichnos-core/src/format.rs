//! Preparing a raw table for process mining. Port of pm4py's
//! `format_dataframe` and `rebase`, on Arrow `RecordBatch`es.

use std::sync::Arc;

use arrow::array::{Array, ArrayRef, AsArray, BooleanArray, Int64Array, TimestampNanosecondArray};
use arrow::compute::{
    CastOptions, SortColumn, and, cast_with_options, filter_record_batch, is_not_null,
    lexsort_to_indices, take_record_batch,
};
use arrow::datatypes::{DataType, Field, FieldRef, Schema, TimeUnit};
use arrow::record_batch::{RecordBatch, RecordBatchOptions};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};

use crate::error::{Error, Result};
use crate::keys::{self, EventKeys};
use crate::log::{EventLog, EventStream};

/// Column with each row's position after sorting, as pm4py's `@@index`.
pub const INDEX_COLUMN: &str = "@@index";
/// Column with each row's case number in sorted order, as pm4py's `@@case_index`.
pub const CASE_INDEX_COLUMN: &str = "@@case_index";

/// Formats tried, in order, when no explicit timestamp format is given.
/// Formats with an offset come first; naive results are read as UTC.
const OFFSET_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S%.f%:z",
    "%Y-%m-%d %H:%M:%S%.f%z",
    "%Y-%m-%dT%H:%M:%S%.f%:z",
    "%Y-%m-%dT%H:%M:%S%.f%z",
    "%Y-%m-%d %H:%M%:z",
    "%Y-%m-%dT%H:%M%:z",
];
const NAIVE_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%dT%H:%M",
    "%Y/%m/%d %H:%M:%S%.f",
    "%Y/%m/%d %H:%M",
];
const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%Y/%m/%d"];

/// Parses one timestamp string to nanoseconds since the epoch (UTC).
fn parse_timestamp(text: &str, format: Option<&str>) -> Option<i64> {
    let text = text.trim();
    let (offset, naive, dates): (&[&str], &[&str], &[&str]) = match format {
        Some(f) => (&[f][..], &[f][..], &[f][..]),
        None => (OFFSET_FORMATS, NAIVE_FORMATS, DATE_FORMATS),
    };
    let utc = if format.is_none() {
        DateTime::parse_from_rfc3339(text).ok().map(|d| d.to_utc())
    } else {
        None
    };
    utc.or_else(|| {
        offset
            .iter()
            .find_map(|f| DateTime::<FixedOffset>::parse_from_str(text, f).ok())
            .map(|d| d.to_utc())
    })
    .or_else(|| {
        naive
            .iter()
            .find_map(|f| NaiveDateTime::parse_from_str(text, f).ok())
            .map(|d| d.and_utc())
    })
    .or_else(|| {
        dates
            .iter()
            .find_map(|f| NaiveDate::parse_from_str(text, f).ok())
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|d| d.and_utc())
    })?
    .timestamp_nanos_opt()
}

/// Converts a column to `Timestamp(Nanosecond, "UTC")`. Strings are parsed;
/// timestamps keep their instant; a naive timestamp is read as UTC; dates
/// become midnight UTC. Any other type, numbers included, is an error:
/// pandas leaves a numeric column unparsed, and a cast would read the numbers
/// as nanoseconds since 1970.
fn to_utc_timestamps(name: &str, column: &ArrayRef, format: Option<&str>) -> Result<ArrayRef> {
    let target = DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()));
    let strict = CastOptions {
        safe: false,
        ..CastOptions::default()
    };
    match column.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View | DataType::Dictionary(..) => {
            let strings = cast_with_options(column, &DataType::Utf8, &strict)?;
            let values = strings
                .as_string::<i32>()
                .iter()
                .map(|v| {
                    v.map(|s| {
                        parse_timestamp(s, format).ok_or_else(|| Error::UnparseableTimestamp {
                            column: name.to_owned(),
                            value: s.to_owned(),
                        })
                    })
                    .transpose()
                })
                .collect::<Result<Vec<Option<i64>>>>()?;
            Ok(Arc::new(
                TimestampNanosecondArray::from(values).with_timezone("UTC"),
            ))
        }
        DataType::Timestamp(_, None) => {
            // Read naive values as UTC: relabel, then convert the unit.
            let ns = cast_with_options(
                column,
                &DataType::Timestamp(TimeUnit::Nanosecond, None),
                &strict,
            )?;
            Ok(Arc::new(
                ns.as_primitive::<arrow::datatypes::TimestampNanosecondType>()
                    .clone()
                    .with_timezone("UTC"),
            ))
        }
        DataType::Timestamp(_, Some(_)) | DataType::Date32 | DataType::Date64 => {
            Ok(cast_with_options(column, &target, &strict)?)
        }
        other => Err(Error::UnsupportedColumn {
            column: name.to_owned(),
            data_type: other.to_string(),
        }),
    }
}

/// A batch under construction: fields and columns that can be replaced by name.
struct Table {
    fields: Vec<FieldRef>,
    columns: Vec<ArrayRef>,
}

impl Table {
    fn index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name() == name)
    }

    fn get(&self, name: &str) -> Result<ArrayRef> {
        self.index(name)
            .map(|i| self.columns[i].clone())
            .ok_or_else(|| Error::MissingColumn(name.to_owned()))
    }

    /// Sets column `name`, in place if it exists, else at the end.
    fn set(&mut self, name: &str, column: ArrayRef) {
        let field = Arc::new(Field::new(name, column.data_type().clone(), true));
        match self.index(name) {
            Some(i) => {
                self.fields[i] = field;
                self.columns[i] = column;
            }
            None => {
                self.fields.push(field);
                self.columns.push(column);
            }
        }
    }

    fn remove(&mut self, name: &str) {
        if let Some(i) = self.index(name) {
            self.fields.remove(i);
            self.columns.remove(i);
        }
    }

    /// Copies column `source` to the end as `target`, replacing any old
    /// `target`, unless the two names are the same.
    fn copy_as(&mut self, source: &str, target: &str) -> Result<()> {
        if source != target {
            let column = self.get(source)?;
            self.remove(target);
            self.set(target, column);
        }
        Ok(())
    }

    fn batch(self, rows: usize) -> Result<RecordBatch> {
        let options = RecordBatchOptions::new().with_row_count(Some(rows));
        Ok(RecordBatch::try_new_with_options(
            Arc::new(Schema::new(self.fields)),
            self.columns,
            &options,
        )?)
    }
}

/// Prepares a raw table for process mining, as pm4py's `format_dataframe`
/// does for a DataFrame. Use [`EventKeys::default`] on the result.
///
/// 1. Copies the columns named by `keys.case_id`, `keys.activity` and
///    `keys.timestamp` to `case:concept:name`, `concept:name` and
///    `time:timestamp` (to the end of the table, replacing old columns of
///    those names).
/// 2. Converts `time:timestamp` and `keys.start_timestamp` (if present) to
///    `Timestamp(Nanosecond, "UTC")`. Strings are parsed with
///    `timestamp_format` (chrono syntax) if given, else as ISO 8601 / RFC 3339
///    with or without offset; a value without offset is read as UTC. Other
///    timestamp columns are relabelled to UTC too.
/// 3. Drops rows with a null case ID, activity or timestamp.
/// 4. Casts the case ID and activity to strings.
/// 5. Sorts by case ID, timestamp and original row order, and writes
///    `@@index` (row position) and `@@case_index` (case number).
/// 6. Copies `keys.start_timestamp` to `start_timestamp` if present.
///
/// Fails if a required column is missing or a timestamp does not parse.
pub fn format_batch(
    batch: &RecordBatch,
    keys: &EventKeys,
    timestamp_format: Option<&str>,
) -> Result<RecordBatch> {
    let schema = batch.schema();
    let mut table = Table {
        fields: schema.fields().iter().cloned().collect(),
        columns: batch.columns().to_vec(),
    };
    for required in [&keys.case_id, &keys.activity, &keys.timestamp] {
        table.get(required)?;
    }
    table.copy_as(&keys.case_id, keys::CASE_CONCEPT_NAME)?;
    table.copy_as(&keys.activity, keys::CONCEPT_NAME)?;
    table.copy_as(&keys.timestamp, keys::TIME_TIMESTAMP)?;

    for name in [keys::TIME_TIMESTAMP, keys.start_timestamp.as_str()] {
        if let Ok(column) = table.get(name) {
            let converted = to_utc_timestamps(name, &column, timestamp_format)?;
            table.set(name, converted);
        }
    }
    let other_timestamps: Vec<String> = table
        .fields
        .iter()
        .filter(|f| matches!(f.data_type(), DataType::Timestamp(..)))
        .map(|f| f.name().clone())
        .collect();
    for name in other_timestamps {
        let converted = to_utc_timestamps(&name, &table.get(&name)?, None)?;
        table.set(&name, converted);
    }

    // Drop rows that lack a case ID, activity or timestamp.
    let mut keep: Option<BooleanArray> = None;
    for name in [
        keys::CASE_CONCEPT_NAME,
        keys::CONCEPT_NAME,
        keys::TIME_TIMESTAMP,
    ] {
        let present = is_not_null(&table.get(name)?)?;
        keep = Some(match keep {
            Some(k) => and(&k, &present)?,
            None => present,
        });
    }
    let rows = batch.num_rows();
    let fields = table.fields.clone();
    let mut table = match keep {
        Some(keep) if keep.true_count() < rows => {
            let filtered = filter_record_batch(&table.batch(rows)?, &keep)?;
            Table {
                fields,
                columns: filtered.columns().to_vec(),
            }
        }
        _ => table,
    };
    let rows = table.columns.first().map_or(rows, |c| c.len());

    let strict = CastOptions {
        safe: false,
        ..CastOptions::default()
    };
    for name in [keys::CASE_CONCEPT_NAME, keys::CONCEPT_NAME] {
        let strings = cast_with_options(&table.get(name)?, &DataType::Utf8, &strict)?;
        table.set(name, strings);
    }

    // Sort by case ID, timestamp and original row order.
    let original: ArrayRef = Arc::new(Int64Array::from_iter_values(0..rows as i64));
    table.set(INDEX_COLUMN, original.clone());
    let order = lexsort_to_indices(
        &[
            SortColumn {
                values: table.get(keys::CASE_CONCEPT_NAME)?,
                options: None,
            },
            SortColumn {
                values: table.get(keys::TIME_TIMESTAMP)?,
                options: None,
            },
            SortColumn {
                values: original,
                options: None,
            },
        ],
        None,
    )?;
    let fields = table.fields.clone();
    let sorted = take_record_batch(&table.batch(rows)?, &order)?;
    let mut table = Table {
        fields,
        columns: sorted.columns().to_vec(),
    };
    table.set(
        INDEX_COLUMN,
        Arc::new(Int64Array::from_iter_values(0..rows as i64)),
    );
    let cases = table.get(keys::CASE_CONCEPT_NAME)?;
    let cases = cases.as_string::<i32>();
    let mut case_index = Vec::with_capacity(rows);
    let mut current = -1_i64;
    for i in 0..rows {
        if i == 0 || cases.value(i) != cases.value(i - 1) {
            current += 1;
        }
        case_index.push(current);
    }
    table.set(CASE_INDEX_COLUMN, Arc::new(Int64Array::from(case_index)));

    if keys.start_timestamp != keys::START_TIMESTAMP
        && let Ok(column) = table.get(&keys.start_timestamp)
    {
        table.set(keys::START_TIMESTAMP, column);
    }
    table.batch(rows)
}

impl EventLog {
    /// Re-keys the log: flattens it to a table, applies [`format_batch`] with
    /// `keys`, and groups it back with the default keys. Port of pm4py's
    /// `rebase` on an `EventLog`. The events gain `@@index` and
    /// `@@case_index`.
    pub fn rebase(&self, keys: &EventKeys) -> Result<EventLog> {
        let defaults = EventKeys::default();
        let table = format_batch(&self.to_arrow(&defaults)?, keys, None)?;
        EventLog::from_arrow(&table, &defaults)
    }
}

impl EventStream {
    /// Re-keys the stream with [`format_batch`]. Port of pm4py's `rebase` on
    /// an `EventStream`.
    pub fn rebase(&self, keys: &EventKeys) -> Result<EventStream> {
        EventStream::from_arrow(&format_batch(&self.to_arrow()?, keys, None)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::StringArray;
    use arrow::datatypes::Int64Type;

    fn raw() -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                Field::new("case", DataType::Int64, true),
                Field::new("task", DataType::Utf8, true),
                Field::new("when", DataType::Utf8, true),
            ])),
            vec![
                Arc::new(Int64Array::from(vec![
                    Some(2),
                    Some(1),
                    Some(2),
                    None,
                    Some(1),
                ])),
                Arc::new(StringArray::from(vec![
                    Some("b"),
                    Some("x"),
                    Some("a"),
                    Some("z"),
                    Some("y"),
                ])),
                Arc::new(StringArray::from(vec![
                    Some("2020-01-01 10:00:00+01:00"),
                    Some("2020-01-01T08:00:00Z"),
                    Some("2020-01-01 08:30:00"),
                    Some("2020-01-01 00:00:00"),
                    Some("2020-01-01 08:00"),
                ])),
            ],
        )
        .unwrap()
    }

    #[test]
    fn format_batch_renames_parses_filters_and_sorts() {
        let keys = EventKeys::default()
            .with_case_id("case")
            .with_activity("task")
            .with_timestamp("when");
        let out = format_batch(&raw(), &keys, None).unwrap();
        let schema = out.schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            names,
            [
                "case",
                "task",
                "when",
                "case:concept:name",
                "concept:name",
                "time:timestamp",
                "@@index",
                "@@case_index"
            ]
        );
        assert_eq!(out.num_rows(), 4);
        let acts = out.column(4).as_string::<i32>();
        // Case "1": x and y tie at 08:00 UTC, so input order decides.
        let acts: Vec<&str> = (0..4).map(|i| acts.value(i)).collect();
        assert_eq!(acts, ["x", "y", "a", "b"]);
        let case_index = out.column(7).as_primitive::<Int64Type>();
        assert_eq!(case_index.values().to_vec(), [0, 0, 1, 1]);
        assert_eq!(
            out.column(5).data_type(),
            &DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
        );
        // 10:00+01:00 is 09:00 UTC, after 08:30 UTC.
        let log = EventLog::from_arrow(&out, &EventKeys::default()).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log.traces[1].case_id().unwrap().as_str(), Some("2"));
    }

    #[test]
    fn explicit_format_and_errors() {
        assert_eq!(
            parse_timestamp("30-12-2010 11:02:00", Some("%d-%m-%Y %H:%M:%S")),
            Some(1_293_706_920_000_000_000)
        );
        assert!(parse_timestamp("2010-12-30", None).is_some());
        assert!(parse_timestamp("yesterday", None).is_none());
        let keys = EventKeys::default()
            .with_case_id("case")
            .with_activity("task");
        assert!(matches!(
            format_batch(&raw(), &keys, None),
            Err(Error::MissingColumn(c)) if c == "time:timestamp"
        ));
        let keys = keys.with_timestamp("task");
        assert!(matches!(
            format_batch(&raw(), &keys, None),
            Err(Error::UnparseableTimestamp { .. })
        ));
        // A numeric timestamp column is refused, not read as nanoseconds.
        let keys = keys.with_timestamp("case");
        assert!(matches!(
            format_batch(&raw(), &keys, None),
            Err(Error::UnsupportedColumn { column, .. }) if column == "time:timestamp"
        ));
    }

    #[test]
    fn rebase_renames_the_activity() {
        let mut log = EventLog::from_trace_strings(["A,B", "C"], ",", &EventKeys::default());
        for (i, e) in log
            .traces
            .iter_mut()
            .flat_map(|t| &mut t.events)
            .enumerate()
        {
            e.insert("task", format!("t{i}"));
        }
        let rebased = log
            .rebase(&EventKeys::default().with_activity("task"))
            .unwrap();
        let first = &rebased.traces[0].events[0];
        assert_eq!(first.get("concept:name").unwrap().as_str(), Some("t0"));
        assert_eq!(first.get("@@case_index").unwrap().as_i64(), Some(0));
        assert_eq!(
            rebased.traces[1].events[0].get("@@index").unwrap().as_i64(),
            Some(2)
        );
    }
}
