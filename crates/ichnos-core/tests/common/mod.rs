//! Shared loading for the golden tests: CSV fixtures read as pandas and
//! pm4py read them.

use std::path::Path;
use std::sync::Arc;

use ichnos_core::arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray};
use ichnos_core::arrow::datatypes::{DataType, Field, Schema};
use ichnos_core::arrow::record_batch::RecordBatch;
use ichnos_core::{EventKeys, EventLog, format_batch};
use ichnos_golden::fixture_path;

/// Reads a CSV into a batch, typing each column as pandas' `read_csv` would
/// for these fixtures: all ints, else all floats, else strings. Empty cells
/// are nulls. An unnamed column `i` is named `Unnamed: i`, as in pandas.
pub fn read_csv(path: &Path) -> RecordBatch {
    let mut reader = csv::Reader::from_path(path).expect("open csv");
    let headers: Vec<String> = reader
        .headers()
        .expect("csv header")
        .iter()
        .enumerate()
        .map(|(i, h)| {
            if h.is_empty() {
                format!("Unnamed: {i}")
            } else {
                h.to_owned()
            }
        })
        .collect();
    let mut cells: Vec<Vec<Option<String>>> = vec![Vec::new(); headers.len()];
    for record in reader.records() {
        let record = record.expect("csv record");
        for (column, value) in cells.iter_mut().zip(record.iter()) {
            column.push((!value.is_empty()).then(|| value.to_owned()));
        }
    }
    let mut fields = Vec::new();
    let mut columns: Vec<ArrayRef> = Vec::new();
    for (name, values) in headers.iter().zip(cells) {
        let present = || values.iter().flatten();
        let column: ArrayRef = if present().all(|v| v.parse::<i64>().is_ok()) {
            Arc::new(Int64Array::from_iter(
                values
                    .iter()
                    .map(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap())),
            ))
        } else if present().all(|v| v.parse::<f64>().is_ok()) {
            Arc::new(Float64Array::from_iter(
                values
                    .iter()
                    .map(|v| v.as_ref().map(|s| s.parse::<f64>().unwrap())),
            ))
        } else {
            Arc::new(StringArray::from_iter(values.iter().map(|v| v.as_deref())))
        };
        let data_type: DataType = column.data_type().clone();
        fields.push(Field::new(name, data_type, true));
        columns.push(column);
    }
    RecordBatch::try_new(Arc::new(Schema::new(fields)), columns).expect("batch")
}

/// The CSV fixture `name` as an event log, loaded as the golden harness
/// loads it: `read_csv`, then `format_batch` (pm4py's `format_dataframe`),
/// then grouped by case.
pub fn load_csv_log(name: &str) -> EventLog {
    let keys = EventKeys::default();
    let raw = read_csv(&fixture_path(name));
    let table = format_batch(&raw, &keys, None).expect("format");
    EventLog::from_arrow(&table, &keys).expect("log")
}
