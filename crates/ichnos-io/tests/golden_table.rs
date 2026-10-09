mod common;

use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{golden, log::LogSummary};
use ichnos_io::{ParquetReadOptions, read_csv, read_parquet};
use std::{collections::BTreeMap, fs, path::Path};

#[test]
fn all_table_fixtures_match_oracle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for entry in fs::read_dir(root.join("fixtures/golden/io")).unwrap() {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap();
        let fixture = value["meta"]["fixtures"]["log"].as_str().unwrap();
        let log = if fixture.ends_with(".csv") {
            read_csv(root.join(fixture), &Default::default())
        } else if fixture.ends_with(".parquet") {
            read_parquet(
                root.join(fixture),
                &ParquetReadOptions {
                    batch_size: 7,
                    ..Default::default()
                },
            )
        } else {
            continue;
        }
        .unwrap_or_else(|e| panic!("{fixture}: {e}"));
        common::assert_golden(&log, &value["expected"], fixture);
    }
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
fn csv_seed_summary_matches_pm4py() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/logs");
    let log = read_csv(root.join("running-example.csv"), &Default::default()).unwrap();
    let expected: LogSummary = golden("log", "running-example-csv").expected_as();
    assert_summary_eq(&summarize(&log), &expected);
}
