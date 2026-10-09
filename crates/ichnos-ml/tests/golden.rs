mod common;

use std::collections::BTreeSet;

use common::load;
use ichnos_core::EventLog;
use ichnos_golden::{Tolerance, as_f64, assert_close, golden};
use ichnos_ml::*;
use serde_json::Value;

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_owned())
        .collect()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| as_f64(x).unwrap())
        .collect()
}

fn case_ids(log: &EventLog) -> Vec<String> {
    log.traces
        .iter()
        .map(|t| t.case_id().unwrap().to_string())
        .collect()
}

/// Checks a feature table against a `{name: column}` map.
fn check_features(
    case: &str,
    names: &[String],
    column: impl Fn(&str) -> Option<Vec<f64>>,
    expected: &Value,
) {
    let expected = expected.as_object().unwrap();
    let ours: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let theirs: BTreeSet<&str> = expected.keys().map(String::as_str).collect();
    assert_eq!(ours, theirs, "{case}: feature names");
    assert_eq!(names.len(), ours.len(), "{case}: duplicate feature names");
    for (name, values) in expected {
        let actual = column(name).unwrap();
        let values = floats(values);
        assert_eq!(actual.len(), values.len(), "{case}: rows of {name}");
        for (a, b) in actual.iter().zip(&values) {
            assert!(
                Tolerance::METRIC.accepts(*a, *b),
                "{case}: {name}: {actual:?} != {values:?}"
            );
        }
    }
}

#[test]
fn oracle_split_train_test() {
    for name in ["running-example", "receipt"] {
        let e: Value = golden("ml", &format!("split-{name}")).expected_as();
        let log = load(name);
        let mut draws = e["draws"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.as_u64().unwrap() as usize);
        let (train, test) = split_train_test(&log, 0.8, |k| {
            let d = draws.next().unwrap();
            assert!(d < k);
            d
        })
        .unwrap();
        assert!(draws.next().is_none());
        assert_eq!(case_ids(&train), strings(&e["train"]), "{name}");
        assert_eq!(case_ids(&test), strings(&e["test"]), "{name}");
        assert_eq!(train.attributes, log.attributes);
    }
}

#[test]
fn oracle_get_prefixes_from_log() {
    for name in ["running-example", "receipt"] {
        let e: Value = golden("ml", &format!("prefixes-{name}")).expected_as();
        let log = load(name);
        for (length, rows) in e.as_object().unwrap() {
            let prefixed = get_prefixes_from_log(&log, length.parse().unwrap());
            let rows = rows.as_array().unwrap();
            assert_eq!(prefixed.traces.len(), rows.len());
            for (t, row) in prefixed.traces.iter().zip(rows) {
                assert_eq!(t.case_id().unwrap().to_string(), row[0].as_str().unwrap());
                assert_eq!(t.events.len() as u64, row[1].as_u64().unwrap());
                let last = t.events.last().unwrap().get("@@index").unwrap();
                assert_eq!(last.as_f64().unwrap(), row[2].as_f64().unwrap());
            }
        }
    }
}

#[test]
fn oracle_extract_features_dataframe() {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    let cases: Vec<(&str, &str, FeatureOptions)> = vec![
        ("running-example", "auto", FeatureOptions::default()),
        (
            "running-example",
            "count",
            FeatureOptions {
                count_occurrences: true,
                ..FeatureOptions::default()
            },
        ),
        (
            "running-example",
            "stats",
            FeatureOptions {
                enable_numeric_attribute_statistics: true,
                ..FeatureOptions::default()
            },
        ),
        (
            "running-example",
            "aggs",
            FeatureOptions {
                numeric_attribute_aggregations: Some(vec![
                    NumericAggregation::Sum,
                    NumericAggregation::Stdev,
                    NumericAggregation::Max,
                    NumericAggregation::Sum,
                ]),
                ..FeatureOptions::default()
            },
        ),
        (
            "running-example",
            "lists",
            FeatureOptions {
                str_ev_attr: s(&["concept:name", "org:resource"]),
                num_ev_attr: s(&["Costs"]),
                str_tr_attr: s(&["creator"]),
                ..FeatureOptions::default()
            },
        ),
        ("roadtraffic100traces", "auto", FeatureOptions::default()),
    ];
    for (name, variant, options) in cases {
        let id = format!("features-df-{name}-{variant}");
        let e: Value = golden("ml", &id).expected_as();
        let table = extract_features_dataframe(&load(name), &options).unwrap();
        assert_eq!(table.case_ids, strings(&e["case_ids"]), "{id}");
        check_features(&id, &table.names, |n| table.column(n), &e["features"]);
    }
}

#[test]
fn oracle_trace_features() {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    let cases = [
        (
            "running-example",
            None,
            TraceFeatureOptions {
                str_ev_attr: s(&["concept:name", "org:resource"]),
                num_ev_attr: s(&["Costs"]),
                str_tr_attr: s(&["creator"]),
                str_evsucc_attr: s(&["concept:name"]),
                extras: TraceExtras::all(),
                ..TraceFeatureOptions::default()
            },
        ),
        (
            "interval_event_log",
            Some(12),
            TraceFeatureOptions {
                str_ev_attr: s(&["concept:name"]),
                start_timestamp_key: "start_timestamp".to_owned(),
                extras: TraceExtras::all(),
                ..TraceFeatureOptions::default()
            },
        ),
    ];
    for (name, cases, options) in cases {
        let id = format!("features-log-{}", name.replace('_', "-"));
        let e: Value = golden("ml", &id).expected_as();
        let mut log = load(name);
        if let Some(n) = cases {
            log.traces.truncate(n);
        }
        let table = trace_features(&log, &options).unwrap();
        assert_eq!(table.case_ids, strings(&e["case_ids"]), "{id}");
        check_features(&id, &table.names, |n| table.column(n), &e["features"]);
    }
}

#[test]
fn oracle_extract_outcome_enriched_dataframe() {
    for name in ["running-example", "roadtraffic100traces"] {
        let id = format!("outcome-{name}");
        let e: Value = golden("ml", &id).expected_as();
        let out =
            extract_outcome_enriched_dataframe(&load(name), &OutcomeOptions::default()).unwrap();
        assert_eq!(out.case_ids, strings(&e["case_ids"]), "{id}");
        let t = &out.times;
        for (col, ours) in [
            ("@@arrival_rate", &t.arrival_rate),
            ("@@finish_rate", &t.finish_rate),
            ("@@diff_start_end", &t.diff_start_end),
            ("@@service_time", &t.service_time),
            ("@@sojourn_time", &t.sojourn_time),
            ("@@waiting_time", &t.waiting_time),
        ] {
            let theirs = floats(&e["times"][col]);
            assert_eq!(ours.len(), theirs.len());
            for (a, b) in ours.iter().zip(&theirs) {
                assert_close(*a, *b, Tolerance::METRIC);
            }
        }
        assert_eq!(
            out.features.case_ids,
            strings(&e["feature_case_ids"]),
            "{id}"
        );
        for (r, c) in out.feature_rows.iter().zip(&out.case_ids) {
            assert_eq!(&out.features.case_ids[*r], c);
        }
        check_features(
            &id,
            &out.features.names,
            |n| out.features.column(n),
            &e["features"],
        );
    }
}

#[test]
fn oracle_extract_temporal_features_dataframe() {
    let cases = [
        ("running-example", "w", GrouperFreq::Week, "time:timestamp"),
        (
            "running-example",
            "d",
            GrouperFreq::Days(1),
            "time:timestamp",
        ),
        (
            "running-example",
            "2d",
            GrouperFreq::Days(2),
            "time:timestamp",
        ),
        (
            "running-example",
            "3h",
            GrouperFreq::Hours(3),
            "time:timestamp",
        ),
        (
            "running-example",
            "ms",
            GrouperFreq::MonthStart,
            "time:timestamp",
        ),
        (
            "running-example",
            "me",
            GrouperFreq::MonthEnd,
            "time:timestamp",
        ),
        (
            "running-example",
            "ys",
            GrouperFreq::YearStart,
            "time:timestamp",
        ),
        (
            "running-example",
            "ye",
            GrouperFreq::YearEnd,
            "time:timestamp",
        ),
        ("receipt", "w", GrouperFreq::Week, "time:timestamp"),
    ];
    for (name, freq, grouper_freq, start) in cases {
        let id = format!("temporal-{}-{freq}", name.replace('_', "-"));
        let e: Value = golden("ml", &id).expected_as();
        let options = TemporalOptions {
            grouper_freq,
            start_timestamp_key: start.to_owned(),
            ..TemporalOptions::default()
        };
        let rows = extract_temporal_features_dataframe(&load(name), &options).unwrap();
        let expected = e.as_array().unwrap();
        assert_eq!(rows.len(), expected.len(), "{id}");
        for (row, x) in rows.iter().zip(expected) {
            assert_eq!(
                row.timestamp.to_rfc3339(),
                x["timestamp"].as_str().unwrap(),
                "{id}"
            );
            let pairs = [
                ("unique_resources", row.unique_resources as f64),
                ("unique_activities", row.unique_activities as f64),
                ("num_events", row.num_events as f64),
                ("average_arrival_rate", row.average_arrival_rate),
                ("average_finish_rate", row.average_finish_rate),
                ("average_waiting_time", row.average_waiting_time),
                ("average_sojourn_time", row.average_sojourn_time),
                ("average_service_time", row.average_service_time),
                (
                    "total_number_of_reworked_activities",
                    row.total_number_of_reworked_activities as f64,
                ),
                ("avg_cases_per_resource", row.avg_cases_per_resource),
                ("avg_events_per_case", row.avg_events_per_case),
                ("number_of_cases", row.number_of_cases as f64),
                ("avg_resources_per_case", row.avg_resources_per_case),
            ];
            assert_eq!(x.as_object().unwrap().len(), pairs.len() + 1);
            for (key, ours) in pairs {
                assert!(
                    Tolerance::METRIC.accepts(ours, x[key].as_f64().unwrap()),
                    "{id} {}: {key}: {ours} != {}",
                    x["timestamp"],
                    x[key]
                );
            }
        }
    }
}

#[test]
fn oracle_extract_target_vector() {
    for name in ["running-example", "roadtraffic100traces"] {
        let e: Value = golden("ml", &format!("target-{name}")).expected_as();
        let log = load(name);
        for (key, variant) in [
            ("next_activity", TargetVariant::NextActivity),
            ("next_time", TargetVariant::NextTime),
            ("remaining_time", TargetVariant::RemainingTime),
        ] {
            let (target, classes) =
                extract_target_vector(&log, variant, &TargetOptions::default()).unwrap();
            assert_eq!(classes, strings(&e[key]["classes"]), "{name} {key}");
            let expected = e[key]["target"].as_array().unwrap();
            match target {
                TargetVector::Activities(t) => {
                    let ours: Vec<Vec<f64>> =
                        t.into_iter().flatten().flatten().map(|v| vec![v]).collect();
                    let theirs: Vec<Vec<f64>> = expected
                        .iter()
                        .flat_map(|tr| tr.as_array().unwrap().iter().flat_map(floats))
                        .map(|v| vec![v])
                        .collect();
                    assert_eq!(ours, theirs, "{name} {key}");
                }
                TargetVector::Times(t) => {
                    assert_eq!(t.len(), expected.len());
                    for (ours, theirs) in t.iter().zip(expected) {
                        let theirs = floats(theirs);
                        assert_eq!(ours.len(), theirs.len());
                        for (a, b) in ours.iter().zip(&theirs) {
                            assert_close(*a, *b, Tolerance::METRIC);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn oracle_extract_ocel_features() {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    let full = |str_attr: &str, num_attr: &str| OcelFeatureOptions {
        work_in_progress: true,
        str_attributes: s(&[str_attr]),
        num_attributes: s(&[num_attr]),
        ..OcelFeatureOptions::default()
    };
    let cases = [
        ("example-log-order", "order", full("oattr1", "oattr2")),
        ("example-log-element", "element", full("oattr1", "oattr2")),
        ("example-log-delivery", "delivery", full("oattr1", "oattr2")),
        (
            "example-log-order-defaults",
            "order",
            OcelFeatureOptions {
                lifecycle_paths: false,
                ..OcelFeatureOptions::default()
            },
        ),
        (
            "example-log-missing-type",
            "nothing",
            OcelFeatureOptions::default(),
        ),
        (
            "ocel20-example-invoice",
            "Invoice",
            full("is_blocked", "po_quantity"),
        ),
        (
            "ocel20-example-purchase-order",
            "Purchase Order",
            full("is_blocked", "po_quantity"),
        ),
    ];
    for (case, object_type, options) in cases {
        let id = format!("ocel-{case}");
        let g = golden("ml", &id);
        let path = g.fixture("log");
        let ocel = if case.starts_with("ocel20") {
            ichnos_io::read_ocel2_json(&path).unwrap()
        } else {
            ichnos_io::read_ocel_json(&path).unwrap()
        };
        let e: Value = g.expected_as();
        let features = extract_ocel_features(&ocel, object_type, &options).unwrap();
        assert_eq!(features.object_ids, strings(&e["object_ids"]), "{id}");
        check_features(&id, &features.names, |n| features.column(n), &e["features"]);
    }
}
