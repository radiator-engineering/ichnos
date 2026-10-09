mod common;
use common::load;
use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{Tolerance, assert_close, assert_multiset_eq, golden};
use ichnos_stats::{attributes::Scalar, variants::*};
use serde_json::{Value, json};

fn rows(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.to_string())
        .collect()
}
#[test]
fn oracle_variants() {
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        let log = load(name);
        let keys = EventKeys::default();
        let e: Value = golden("stats", &format!("variants-{name}")).expected_as();
        let groups = get_variants_from_log_trace_idx(&log, &keys).unwrap();
        let counts = get_variants(&log, &keys).unwrap();
        assert_eq!(
            get_variants_set(&log, &keys).unwrap(),
            counts.keys().cloned().collect()
        );
        assert_eq!(get_variants_count(&log, &keys).unwrap(), counts);
        let objects = convert_variants_trace_idx_to_trace_obj(&log, &groups).unwrap();
        assert_eq!(get_variant_traces(&log, &keys).unwrap(), objects);
        let durations = get_variants_along_with_case_durations(&log, &keys).unwrap();
        for expected in e["variants"].as_array().unwrap() {
            let v: Variant = serde_json::from_value(expected["variant"].clone()).unwrap();
            assert_eq!(json!(groups[&v]), expected["indices"]);
            assert_eq!(json!(counts[&v]), expected["count"]);
            assert_eq!(objects[&v].len(), counts[&v]);
            let d: Vec<f64> = serde_json::from_value(expected["durations"].clone()).unwrap();
            for (a, b) in durations[&v].durations.iter().zip(d) {
                assert_close(*a, b, Tolerance::METRIC);
            }
        }
        assert_eq!(json!(get_variants_sorted_by_count(&counts)), e["sorted"]);
        let language = get_stochastic_language(&log, &keys).unwrap();
        for row in e["language"].as_array().unwrap() {
            let v: Variant = serde_json::from_value(row["variant"].clone()).unwrap();
            assert_close(
                language[&v],
                row["probability"].as_f64().unwrap(),
                Tolerance::METRIC,
            );
        }
        let split = split_by_process_variant(&log, &keys).unwrap();
        for (v, l) in split {
            assert_eq!(l.len(), counts[&v]);
            assert_eq!(l.attributes, log.attributes);
        }
        assert_eq!(json!(get_rework(&log, &keys).unwrap()), e["rework"]);
        let cases = get_rework_cases(&log, &keys)
            .unwrap()
            .into_iter()
            .map(|(k, r)| {
                let k = match k {
                    Scalar::String(v) => v,
                    Scalar::Int(v) => v.to_string(),
                    _ => panic!(),
                };
                (
                    k,
                    json!({"number_activities":r.number_activities,"rework":r.rework}),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(json!(cases), e["cases"]);
        let chaotic = get_chaotic_activities(&log, &keys, None).unwrap();
        for expected in e["chaotic"].as_array().unwrap() {
            let a = chaotic
                .iter()
                .find(|a| a.activity == expected["activity"].as_str().unwrap())
                .unwrap();
            assert_eq!(json!(a.freq), expected["freq"]);
            for (key, value) in [
                ("entropy", a.entropy),
                ("entropy_smooth", a.entropy_smooth),
                ("entropy_gain", a.entropy_gain),
                ("chaotic_score", a.chaotic_score),
            ] {
                assert_close(value, expected[key].as_f64().unwrap(), Tolerance::METRIC);
            }
        }
        let sequences = log.activity_sequences(&keys).unwrap();
        let traces = sequences
            .traces
            .iter()
            .map(|t| {
                t.iter()
                    .map(|&a| sequences.activities.name(a).to_owned())
                    .collect()
            })
            .collect::<Vec<_>>();
        assert_close(
            total_entropy(&traces, None).unwrap(),
            e["entropy"].as_f64().unwrap(),
            Tolerance::METRIC,
        );
        let segments =
            get_frequent_trace_segments(&log, &keys, e["minimum"].as_u64().unwrap() as usize)
                .unwrap();
        let actual = json!(
            segments
                .into_iter()
                .map(|(v, n)| json!({"variant":v,"count":n}))
                .collect::<Vec<_>>()
        );
        assert_multiset_eq(rows(&actual), rows(&e["segments"]));
        let paths = get_variants_paths_duration(&log, &keys, DurationAggregation::Mean).unwrap();
        for (label, aggregation) in [
            ("median", DurationAggregation::Median),
            ("min", DurationAggregation::Min),
            ("max", DurationAggregation::Max),
            ("sum", DurationAggregation::Sum),
        ] {
            let paths = get_variants_paths_duration(&log, &keys, aggregation).unwrap();
            let expected: Vec<f64> =
                serde_json::from_value(e["path_aggregations"][label].clone()).unwrap();
            assert_eq!(paths.len(), expected.len());
            for (a, b) in paths.iter().zip(expected) {
                assert_close(a.duration, b, Tolerance::METRIC);
            }
        }
        assert_eq!(paths.len(), e["paths"].as_array().unwrap().len());
        for (a, b) in paths.iter().zip(e["paths"].as_array().unwrap()) {
            assert_eq!(json!(a.variant), b["variant"]);
            assert_eq!(json!(a.position), b["position"]);
            assert_eq!(json!(a.variant_count), b["count"]);
            assert_eq!(json!(a.cumulative_occurrence), b["occurrence"]);
            assert_close(
                a.duration,
                b["duration"].as_f64().unwrap(),
                Tolerance::METRIC,
            );
        }
    }
}
#[test]
fn empty_and_invalid_logs() {
    let log = EventLog::default();
    let keys = EventKeys::default();
    assert!(get_variants(&log, &keys).unwrap().is_empty());
    assert!(get_stochastic_language(&log, &keys).unwrap().is_empty());
    assert!(
        get_chaotic_activities(&log, &keys, None)
            .unwrap()
            .is_empty()
    );
    assert!(get_frequent_trace_segments(&log, &keys, 0).is_err());
    let missing = EventLog {
        traces: vec![ichnos_core::Trace::new()],
        ..Default::default()
    };
    assert!(matches!(
        get_rework_cases(&missing, &keys),
        Err(ichnos_stats::Error::MissingCaseId(0))
    ));
    let log = EventLog::from_trace_strings(["A,B,A", "A,A", ""], ",", &keys);
    assert_eq!(get_rework(&log, &keys).unwrap()["A"], 2);
    assert!(get_chaotic_activities(&log, &keys, Some(-1.0)).is_err());
}
