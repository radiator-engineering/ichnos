//! Shared helpers for the golden tests: CSV fixtures read as pandas and
//! pm4py read them, and a canonical form for comparing process trees.
//!
//! The CSV reader is a copy of the one in `ichnos-core`'s golden tests. It
//! goes away when the discovery tests switch to XES fixtures.

use std::path::Path;
use std::sync::Arc;

use ichnos_core::arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray};
use ichnos_core::arrow::datatypes::{DataType, Field, Schema};
use ichnos_core::arrow::record_batch::RecordBatch;
use ichnos_core::{EventKeys, EventLog, format_batch};
use ichnos_model::{Operator, ProcessTree};

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

/// The CSV log at `path`, loaded as the golden harness loads it:
/// `read_csv`, then `format_batch` (pm4py's `format_dataframe`), then grouped
/// by case.
pub fn load_csv_log(path: &Path) -> EventLog {
    let keys = EventKeys::default();
    let table = format_batch(&read_csv(path), &keys, None).expect("format");
    EventLog::from_arrow(&table, &keys).expect("log")
}

/// `tree` with the children of every XOR, parallel and OR node sorted by
/// their pm4py string form. Two trees that differ only in the order of
/// such children have the same canonical form.
pub fn canonical(tree: &ProcessTree) -> ProcessTree {
    match tree {
        ProcessTree::Node(op, children) => {
            let mut children: Vec<ProcessTree> = children.iter().map(canonical).collect();
            if matches!(op, Operator::Xor | Operator::Parallel | Operator::Or) {
                children.sort_by_cached_key(ProcessTree::to_string);
            }
            ProcessTree::Node(*op, children)
        }
        leaf => leaf.clone(),
    }
}
