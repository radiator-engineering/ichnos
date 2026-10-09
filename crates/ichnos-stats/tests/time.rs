mod common;
use common::load;
use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{JsonCompare, Tolerance, assert_close, assert_json_eq, golden};
use ichnos_stats::time::*;
use serde_json::{Value, json};

fn edges(counts: std::collections::BTreeMap<(String, String), usize>) -> Value {
    json!(
        counts
            .into_iter()
            .map(|((a, b), n)| json!({"source":a,"target":b,"count":n}))
            .collect::<Vec<_>>()
    )
}
fn passed(mut value: PassedTime) -> Value {
    value.pre.sort_by(|a, b| a.activity.cmp(&b.activity));
    value.post.sort_by(|a, b| a.activity.cmp(&b.activity));
    let neighbors = |v: Vec<PassedTimeNeighbor>| {
        v.into_iter()
            .map(|v| json!([v.activity, v.duration, v.count]))
            .collect::<Vec<_>>()
    };
    json!({"pre":neighbors(value.pre),"post":neighbors(value.post),"pre_avg_perf":value.pre_avg_perf,"post_avg_perf":value.post_avg_perf})
}
#[test]
fn oracle_time_statistics() {
    for name in [
        "running-example",
        "receipt",
        "roadtraffic100traces",
        "interval_event_log",
    ] {
        let log = load(name);
        let keys = EventKeys::default();
        let options = TimeOptions {
            use_start_timestamp: name == "interval_event_log",
            ..Default::default()
        };
        let relation = RelationOptions {
            time: options.clone(),
            ..Default::default()
        };
        let e: Value = golden("stats", &format!("time-{}", name.replace('_', "-"))).expected_as();
        let check = |field: &str, v: Value| assert_json_eq(&v, &e[field], &JsonCompare::default());
        check(
            "cycle",
            json!(get_cycle_time(&log, &keys, &options).unwrap()),
        );
        for (i, (v, n)) in [
            (vec![(0.0, 5.0), (3.0, 8.0)], 1),
            (vec![(0.0, 3.0), (5.0, 9.0), (11.0, 12.0)], 2),
        ]
        .iter()
        .enumerate()
        {
            assert_close(
                cycle_time(v, *n).unwrap(),
                e["cycle_values"][i].as_f64().unwrap(),
                Tolerance::METRIC,
            );
        }
        for (label, aggregation) in [
            ("mean", Aggregation::Mean),
            ("median", Aggregation::Median),
            ("min", Aggregation::Min),
            ("max", Aggregation::Max),
            ("sum", Aggregation::Sum),
        ] {
            let actual = get_service_time(
                &log,
                &keys,
                &TimeOptions {
                    aggregation,
                    ..options.clone()
                },
            )
            .unwrap();
            assert_json_eq(
                &json!(actual),
                &e["service"][label],
                &JsonCompare::default(),
            );
        }
        let business = TimeOptions {
            business_hours: Some(BusinessHours::default()),
            ..options.clone()
        };
        check(
            "business_service",
            json!(get_service_time(&log, &keys, &business).unwrap()),
        );
        check(
            "concurrent",
            edges(get_concurrent_activities(&log, &keys, &relation).unwrap()),
        );
        let strict = RelationOptions {
            strict: true,
            ..relation.clone()
        };
        check(
            "concurrent_strict",
            edges(get_concurrent_activities(&log, &keys, &strict).unwrap()),
        );
        assert_eq!(
            get_concurrent_events(&log, &keys, &strict).unwrap().len(),
            get_concurrent_activities(&log, &keys, &strict)
                .unwrap()
                .values()
                .sum::<usize>()
        );
        check(
            "eventually",
            edges(get_eventually_follows(&log, &keys, &relation).unwrap()),
        );
        let first = RelationOptions {
            keep_first_following: true,
            ..relation.clone()
        };
        check(
            "eventually_first",
            edges(get_eventually_follows(&log, &keys, &first).unwrap()),
        );
        check(
            "eventually_sequences",
            edges(get_eventually_follows_sequences(&log, &keys).unwrap()),
        );
        check(
            "overlap_cases",
            json!(get_case_overlap(&log, &keys, &options, OverlapOptions::default()).unwrap()),
        );
        check(
            "overlap_events",
            json!(
                get_interval_event_overlap(&log, &keys, &options, OverlapOptions::default())
                    .unwrap()
            ),
        );
        check(
            "overlap_values",
            json!(
                get_overlap(
                    &[(0.0, 1.0), (0.0, 1.0), (1.0, 2.0), (3.0, 4.0)],
                    OverlapOptions::default()
                )
                .unwrap()
            ),
        );
        for activity in e["passed"].as_object().unwrap().keys() {
            let actual = passed(get_passed_time(&log, &keys, activity, &options).unwrap());
            for direction in ["pre", "post"] {
                let mut expected = e[direction][activity].clone();
                expected[direction]
                    .as_array_mut()
                    .unwrap()
                    .sort_by(|a, b| a[0].as_str().cmp(&b[0].as_str()));
                let avg = format!("{direction}_avg_perf");
                assert_json_eq(
                    &actual[direction],
                    &expected[direction],
                    &JsonCompare::default(),
                );
                assert_json_eq(&actual[&avg], &expected[&avg], &JsonCompare::default());
            }
            for (label, opts) in [
                ("passed", options.clone()),
                (
                    "passed_median",
                    TimeOptions {
                        aggregation: Aggregation::Median,
                        ..options.clone()
                    },
                ),
                ("business_passed", business.clone()),
                (
                    "passed_stdev",
                    TimeOptions {
                        aggregation: Aggregation::StandardDeviation,
                        ..options.clone()
                    },
                ),
            ] {
                let actual = passed(get_passed_time(&log, &keys, activity, &opts).unwrap());
                let mut expected = e[label][activity].clone();
                for direction in ["pre", "post"] {
                    expected[direction]
                        .as_array_mut()
                        .unwrap()
                        .sort_by(|a, b| a[0].as_str().cmp(&b[0].as_str()));
                }
                assert_json_eq(&actual, &expected, &JsonCompare::default());
            }
        }
    }
}
#[test]
fn oracle_exact_matching() {
    let e: Value = golden("stats", "stats-util-matching").expected_as();
    for row in e.as_array().unwrap() {
        let left: Vec<f64> = serde_json::from_value(row["left"].clone()).unwrap();
        let right: Vec<f64> = serde_json::from_value(row["right"].clone()).unwrap();
        let expected: Vec<(f64, f64)> = serde_json::from_value(row["pairs"].clone()).unwrap();
        let actual = exact_match_minimum_average(&left, &right).unwrap();
        assert_eq!(actual.len(), expected.len());
        assert_close(
            actual.iter().map(|(a, b)| b - a).sum(),
            expected.iter().map(|(a, b)| b - a).sum(),
            Tolerance::METRIC,
        );
        let mut a = left.clone();
        let mut b = right.clone();
        for (l, r) in actual {
            assert!(l <= r);
            let i = a.iter().position(|v| *v == l).unwrap();
            a.remove(i);
            let i = b.iter().position(|v| *v == r).unwrap();
            b.remove(i);
        }
    }
}
#[test]
fn empty_intervals_and_options() {
    let log = EventLog::default();
    let keys = EventKeys::default();
    let options = TimeOptions::default();
    assert_eq!(get_cycle_time(&log, &keys, &options).unwrap(), 0.0);
    assert!(
        get_case_overlap(&log, &keys, &options, OverlapOptions::default())
            .unwrap()
            .is_empty()
    );
    assert!(
        get_partial_order(&log, &keys, &RelationOptions::default())
            .unwrap()
            .is_empty()
    );
    assert!(exact_match_minimum_average(&[], &[1.0]).unwrap().is_empty());
    assert!(exact_match_minimum_average(&[f64::NAN], &[1.0]).is_err());
    assert!(exact_match_minimum_average(&[-f64::MAX], &[f64::MAX]).is_err());
    assert!(get_overlap(&[(2.0, 1.0)], OverlapOptions::default()).is_err());
    assert_eq!(
        get_overlap(&[(0.0, 1.0), (1.0, 2.0)], OverlapOptions { epsilon: 0.0 }).unwrap(),
        [1, 1]
    );
}

#[test]
fn oracle_business_schedules() {
    use ichnos_core::chrono::{DateTime, NaiveDate};
    let e: Value = golden("stats", "time-business-options").expected_as();
    let default = BusinessHours::default();
    let slots = BusinessHours {
        slots: vec![
            (7 * 3600, 12 * 3600),
            (12 * 3600 + 1, 17 * 3600),
            (10 * 3600, 13 * 3600),
        ],
        ..Default::default()
    };
    let holiday = BusinessHours {
        non_working_dates: [NaiveDate::from_ymd_opt(2024, 1, 8).unwrap()]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    for row in e.as_array().unwrap() {
        let start = DateTime::parse_from_rfc3339(row["start"].as_str().unwrap()).unwrap();
        let end = DateTime::parse_from_rfc3339(row["end"].as_str().unwrap()).unwrap();
        for (label, schedule) in [
            ("default", &default),
            ("slots", &slots),
            ("holiday", &holiday),
        ] {
            assert_close(
                schedule.seconds_between(start, end).unwrap(),
                row[label].as_f64().unwrap(),
                Tolerance::METRIC,
            );
        }
    }
}
