mod common;

use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{golden, log::LogSummary};
use ichnos_io::{
    XesReadOptions, XesWriteOptions, read_xes, read_xes_from_reader, write_xes_to_writer,
};
use std::collections::BTreeMap;
use std::{fs, path::Path};

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
fn seed_summaries_match_pm4py() {
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        let expected: LogSummary = golden("log", &format!("{name}-xes")).expected_as();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/logs")
            .join(format!("{name}.xes"));
        let log = read_xes(path, &XesReadOptions::default()).unwrap();
        assert_summary_eq(&summarize(&log), &expected);
    }
}

#[test]
fn all_xes_fixtures_match_oracle_and_round_trip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for entry in fs::read_dir(root.join("fixtures/golden/io")).unwrap() {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(entry.unwrap().path()).unwrap()).unwrap();
        let Some(fixture) = value["meta"]["fixtures"]["log"].as_str() else {
            continue;
        };
        if !fixture.ends_with(".xes") && !fixture.ends_with(".xes.gz") {
            continue;
        }
        let started = std::time::Instant::now();
        let log = read_xes(root.join(fixture), &XesReadOptions::default())
            .unwrap_or_else(|e| panic!("{fixture}: {e}"));
        common::assert_golden(&log, &value["expected"], fixture);
        let mut xml = Vec::new();
        write_xes_to_writer(&log, &mut xml, &XesWriteOptions::default()).unwrap();
        let back = read_xes_from_reader(xml.as_slice(), &XesReadOptions::default()).unwrap();
        assert_eq!(log, back, "{fixture}");
        eprintln!("{fixture}: {:?}", started.elapsed());
    }
}
