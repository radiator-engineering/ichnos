//! Compares log statistics computed from an `EventLog` with pm4py's golden
//! output in `fixtures/golden/log/`.
//!
//! The CSV case goes through `format_batch`, as pm4py's goes through
//! `pandas.read_csv` and `pm4py.format_dataframe`. The XES cases need the XES
//! reader in `ichnos-io` and are tested there.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use ichnos_core::arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray};
use ichnos_core::arrow::datatypes::{DataType, Field, Schema};
use ichnos_core::arrow::record_batch::RecordBatch;
use ichnos_core::{EventKeys, EventLog, format_batch};
use ichnos_golden::{fixture_path, golden, log::LogSummary};

/// Reads a CSV into a batch, typing each column as pandas' `read_csv` would
/// for these fixtures: all ints, else all floats, else strings. Empty cells
/// are nulls.
fn read_csv(path: &Path) -> RecordBatch {
    let mut reader = csv::Reader::from_path(path).expect("open csv");
    let headers: Vec<String> = reader
        .headers()
        .expect("csv header")
        .iter()
        .map(str::to_owned)
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

/// The statistics pm4py reports, computed from the interned activity view.
fn summarize(log: &EventLog) -> LogSummary {
    let keys = EventKeys::default();
    let seqs = log.activity_sequences(&keys).expect("activities");
    let name = |id| seqs.activities.name(id).to_owned();
    let mut activities = BTreeMap::new();
    let mut start_activities = BTreeMap::new();
    let mut end_activities = BTreeMap::new();
    let mut dfg: BTreeMap<(String, String), u64> = BTreeMap::new();
    for trace in &seqs.traces {
        for &a in trace {
            *activities.entry(name(a)).or_insert(0) += 1;
        }
        if let (Some(&first), Some(&last)) = (trace.first(), trace.last()) {
            *start_activities.entry(name(first)).or_insert(0) += 1;
            *end_activities.entry(name(last)).or_insert(0) += 1;
        }
        for pair in trace.windows(2) {
            *dfg.entry((name(pair[0]), name(pair[1]))).or_insert(0) += 1;
        }
    }
    let variants = log.variants(&keys).expect("variants");
    let mut variant_list: Vec<ichnos_golden::log::VariantCount> = variants
        .iter()
        .map(|v| ichnos_golden::log::VariantCount {
            activities: variants.names(v).map(str::to_owned).collect(),
            count: v.count() as u64,
        })
        .collect();
    variant_list.sort_by(|a, b| a.activities.cmp(&b.activities));
    LogSummary {
        n_cases: log.len() as u64,
        n_events: log.num_events() as u64,
        activities,
        start_activities,
        end_activities,
        variants: variant_list,
        dfg: dfg
            .into_iter()
            .map(|((source, target), count)| ichnos_golden::log::DfgEdge {
                source,
                target,
                count,
            })
            .collect(),
    }
}

fn assert_summary_eq(actual: &LogSummary, expected: &LogSummary) {
    assert_eq!(actual.n_cases, expected.n_cases, "n_cases");
    assert_eq!(actual.n_events, expected.n_events, "n_events");
    assert_eq!(actual.activities, expected.activities, "activities");
    assert_eq!(
        actual.start_activities, expected.start_activities,
        "start activities"
    );
    assert_eq!(
        actual.end_activities, expected.end_activities,
        "end activities"
    );
    assert_eq!(
        actual.variant_counts(),
        expected.variant_counts(),
        "variants"
    );
    assert_eq!(actual.dfg_counts(), expected.dfg_counts(), "dfg");
}

#[test]
fn running_example_csv_matches_pm4py() {
    let g = golden("log", "running-example-csv");
    let expected: LogSummary = g.expected_as();
    let raw = read_csv(&fixture_path("running-example.csv"));
    let keys = EventKeys::default();
    let table = format_batch(&raw, &keys, None).expect("format");
    let log = EventLog::from_arrow(&table, &keys).expect("log");
    assert_summary_eq(&summarize(&log), &expected);
}
