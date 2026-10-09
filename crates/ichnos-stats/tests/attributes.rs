use ichnos_core::EventKeys;
use ichnos_golden::{Tolerance, golden};
use ichnos_stats::attributes::*;
use serde_json::Value;
use std::collections::BTreeMap;

mod common;
use common::load;
fn strings(counts: indexmap::IndexMap<Scalar, usize>) -> BTreeMap<String, usize> {
    counts
        .into_iter()
        .map(|(k, n)| {
            (
                match k {
                    Scalar::String(s) => s,
                    Scalar::Int(i) => i.to_string(),
                    other => panic!("unexpected {other:?}"),
                },
                n,
            )
        })
        .collect()
}

#[test]
fn scalar_and_missing_attributes() {
    use ichnos_core::{AttributeValue, Event, EventLog, Trace};
    let keys = EventKeys::default();
    let mut trace = Trace::new();
    for value in [
        AttributeValue::from(1_i64),
        AttributeValue::from(1.0),
        AttributeValue::from(true),
    ] {
        let mut event = Event::new();
        event.attributes.insert("value", value);
        trace.events.push(event);
    }
    let mut log = EventLog::default();
    log.traces.push(trace);
    let counts =
        get_event_attribute_values(&log, &keys, "value", AttributeCountOptions::default()).unwrap();
    assert_eq!(counts[&Scalar::Int(1)], 3);
    assert!(
        get_event_attribute_values(&log, &keys, "missing", AttributeCountOptions::default())
            .unwrap()
            .is_empty()
    );
    assert!(get_events_distribution(&log, &keys, Distribution::Hours).is_err());
    assert!(!verify_if_trace_attribute_is_in_each_trace(
        &log, &keys, "value"
    ));
    assert!(verify_if_event_attribute_is_in_each_trace(
        &log, &keys, "value"
    ));
    let selected =
        select_attributes_from_log_for_tree(&log, &keys, SelectionOptions::default()).unwrap();
    assert!(selected.numeric_event.is_empty());
    assert!(get_kde_numeric_values(&[f64::NAN], KdeOptions::default()).is_err());
    assert!(
        get_kde_numeric_values(
            &[1.0],
            KdeOptions {
                graph_points: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(get_attributes_threshold::<String>(&[], 0.6, 1, 25).is_err());
}
#[test]
fn oracle_attributes() {
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        let log = load(name);
        let keys = EventKeys::default();
        let g = golden("stats", &format!("attributes-{name}"));
        let e: Value = g.expected_as();
        let check = |field: &str, actual: Value| assert_eq!(actual, e[field], "{name}: {field}");
        check(
            "event_attributes",
            serde_json::json!(get_event_attributes(&log, &keys)),
        );
        check(
            "trace_attributes",
            serde_json::json!(get_trace_attributes(&log, &keys)),
        );
        check(
            "activities",
            serde_json::json!(strings(
                get_event_attribute_values(
                    &log,
                    &keys,
                    "concept:name",
                    AttributeCountOptions::default()
                )
                .unwrap()
            )),
        );
        check(
            "once",
            serde_json::json!(strings(
                get_event_attribute_values(
                    &log,
                    &keys,
                    "concept:name",
                    AttributeCountOptions {
                        keep_once_per_case: true
                    }
                )
                .unwrap()
            )),
        );
        check(
            "trace_ids",
            serde_json::json!(strings(
                get_trace_attribute_values(&log, &keys, "concept:name").unwrap()
            )),
        );
        let start = get_start_activities(&log, &keys)
            .unwrap()
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let end = get_end_activities(&log, &keys)
            .unwrap()
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        check("start", serde_json::json!(start));
        check("end", serde_json::json!(end));
        let counts = get_event_attribute_values(
            &log,
            &keys,
            "concept:name",
            AttributeCountOptions::default(),
        )
        .unwrap();
        let sorted = get_sorted_attributes_list(&counts);
        let ordered: Vec<_> = sorted
            .iter()
            .map(|(v, n)| match v {
                Scalar::String(s) => (s.clone(), *n),
                _ => panic!(),
            })
            .collect();
        check("sorted", serde_json::json!(ordered));
        let start = get_start_activities(&log, &keys).unwrap();
        let end = get_end_activities(&log, &keys).unwrap();
        check(
            "start_threshold",
            serde_json::json!(
                get_start_activities_threshold(&get_sorted_start_activities_list(&start), 0.6)
                    .unwrap()
            ),
        );
        check(
            "end_threshold",
            serde_json::json!(
                get_end_activities_threshold(&get_sorted_end_activities_list(&end), 0.6).unwrap()
            ),
        );
        check(
            "threshold",
            serde_json::json!(get_attributes_threshold(&sorted, 0.6, 1, 25).unwrap()),
        );
        for (d, label) in [
            (Distribution::DaysMonth, "days_month"),
            (Distribution::Months, "months"),
            (Distribution::Years, "years"),
            (Distribution::Hours, "hours"),
            (Distribution::DaysWeek, "days_week"),
            (Distribution::Weeks, "weeks"),
        ] {
            assert_eq!(
                serde_json::json!(get_events_distribution(&log, &keys, d).unwrap()),
                e["distributions"][label]
            );
        }
        let attrs = get_event_attributes(&log, &keys)
            .into_iter()
            .collect::<Vec<_>>();
        check(
            "event_presence",
            serde_json::json!(check_event_attributes_presence(&log, &keys, &attrs)),
        );
        let attrs = get_trace_attributes(&log, &keys)
            .into_iter()
            .collect::<Vec<_>>();
        check(
            "trace_presence",
            serde_json::json!(check_trace_attributes_presence(&log, &keys, &attrs)),
        );
        let selected = select_attributes_from_log_for_tree(
            &log,
            &keys,
            SelectionOptions {
                max_cases: log.len(),
                ..Default::default()
            },
        )
        .unwrap();
        for (i, actual) in [
            selected.string_trace,
            selected.string_event,
            selected.numeric_trace,
            selected.numeric_event,
        ]
        .iter()
        .enumerate()
        {
            let expected: Vec<String> = serde_json::from_value(e["selection"][i].clone()).unwrap();
            ichnos_golden::assert_multiset_eq(actual.clone(), expected);
        }
        let options = KdeOptions {
            graph_points: 20,
            ..Default::default()
        };
        for (i, values) in [
            vec![],
            vec![0.0, 0.0],
            vec![1.0, 2.0, 4.0, 8.0],
            vec![-8.0, -4.0, -2.0, -1.0],
            vec![-2.0, 0.0, 3.0],
        ]
        .iter()
        .enumerate()
        {
            let density = get_kde_numeric_values(values, options).unwrap();
            for (axis, actual) in [density.x, density.y].iter().enumerate() {
                let expected: Vec<f64> =
                    serde_json::from_value(e["numeric_kde"][i][axis].clone()).unwrap();
                assert_eq!(actual.len(), expected.len());
                for (a, b) in actual.iter().zip(expected) {
                    assert!(Tolerance::METRIC.accepts(*a, b), "{a} != {b}");
                }
            }
        }
        let density = get_kde_date_attribute(&log, &keys, &keys.timestamp, options).unwrap();
        let numeric = get_kde_numeric_attribute(&log, &keys, "@@index", options).unwrap();
        for (i, axis) in [numeric.x, numeric.y].iter().enumerate() {
            let expected: Vec<f64> =
                serde_json::from_value(e["event_numeric_kde"][i].clone()).unwrap();
            assert_eq!(axis.len(), expected.len());
            for (a, b) in axis.iter().zip(expected) {
                ichnos_golden::assert_close(*a, b, Tolerance::METRIC);
            }
        }
        let expected_x: Vec<f64> = serde_json::from_value(e["date_x"].clone()).unwrap();
        assert_eq!(density.x.len(), expected_x.len());
        for (a, b) in density.x.iter().zip(expected_x) {
            ichnos_golden::assert_close(*a, b, Tolerance::absolute(1e-6));
        }
        let expected: Vec<f64> = serde_json::from_value(e["date_kde"].clone()).unwrap();
        for (a, b) in density.y.iter().zip(expected) {
            assert!(Tolerance::METRIC.accepts(*a, b));
        }
        for (key, bandwidth) in [
            ("silverman", Bandwidth::Silverman),
            ("factor", Bandwidth::Factor(0.5)),
        ] {
            let actual = get_kde_numeric_values(
                &[1.0, 2.0, 4.0, 8.0],
                KdeOptions {
                    bandwidth,
                    ..options
                },
            )
            .unwrap();
            for (i, axis) in [actual.x, actual.y].iter().enumerate() {
                let expected: Vec<f64> = serde_json::from_value(e[key][i].clone()).unwrap();
                assert_eq!(axis.len(), expected.len());
                for (a, b) in axis.iter().zip(expected) {
                    ichnos_golden::assert_close(*a, b, Tolerance::METRIC);
                }
            }
        }
    }
}
