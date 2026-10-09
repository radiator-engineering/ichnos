use ichnos_core::{AttributeValue, Event, EventKeys, EventLog, Trace, chrono::DateTime};
use ichnos_discovery::dfg::{Aggregation, BusinessHours};
use ichnos_discovery::*;
use ichnos_golden::{Golden, assert_json_eq, fixture_path, golden};
use ichnos_model::Dfg;
use serde_json::{Value, json};

fn load(g: &Golden) -> (EventLog, EventKeys) {
    let params = &g.meta().params;
    let keys = EventKeys::default()
        .with_activity(params["activity_key"].as_str().unwrap_or("concept:name"))
        .with_timestamp(params["timestamp_key"].as_str().unwrap_or("time:timestamp"))
        .with_start_timestamp(params["start_key"].as_str().unwrap_or("start_timestamp"));
    let log = if let Some(traces) = params["traces"].as_array() {
        let mut log = EventLog::default();
        for trace in traces {
            let mut t = Trace::new();
            for row in trace.as_array().unwrap() {
                let mut e = Event::new();
                e.attributes
                    .insert(keys.activity.as_str(), row[0].as_str().unwrap());
                for (key, value) in [(&keys.timestamp, &row[1]), (&keys.start_timestamp, &row[2])] {
                    e.attributes.insert(
                        key.as_str(),
                        AttributeValue::Date(
                            DateTime::parse_from_rfc3339(value.as_str().unwrap()).unwrap(),
                        ),
                    );
                }
                t.events.push(e);
            }
            log.traces.push(t);
        }
        log
    } else {
        let path = fixture_path(
            g.meta().fixtures["log"]
                .strip_prefix("fixtures/logs/")
                .unwrap(),
        );
        if path.extension().unwrap() == "xes" {
            let mut log = ichnos_io::read_xes(path, &Default::default()).unwrap();
            // pm4py's XES reader normalizes offsets to UTC; ichnos retains them.
            // Business hours use local wall clocks, so give both miners the
            // same timestamps before comparing that configured mode.
            for e in log.traces.iter_mut().flat_map(|t| &mut t.events) {
                for key in [&keys.timestamp, &keys.start_timestamp] {
                    if let Some(date) = e.get(key).and_then(AttributeValue::as_date) {
                        e.attributes.insert(
                            key.as_str(),
                            AttributeValue::Date(date.with_timezone(
                                &ichnos_core::chrono::FixedOffset::east_opt(0).unwrap(),
                            )),
                        );
                    }
                }
            }
            log
        } else {
            ichnos_io::read_csv(path, &Default::default()).unwrap()
        }
    };
    (log, keys)
}
fn edges<T: serde::Serialize>(
    map: &std::collections::BTreeMap<(ichnos_model::Label, ichnos_model::Label), T>,
) -> Value {
    json!(
        map.iter()
            .map(|((a, b), n)| json!([a, b, n]))
            .collect::<Vec<_>>()
    )
}
fn assert_counts(
    map: &std::collections::BTreeMap<(ichnos_model::Label, ichnos_model::Label), u64>,
    expected: &Value,
) {
    ichnos_golden::assert_map_eq(
        map.iter().map(|((a, b), n)| ((a.as_str(), b.as_str()), *n)),
        expected.as_array().unwrap().iter().map(|row| {
            (
                (row[0].as_str().unwrap(), row[1].as_str().unwrap()),
                row[2].as_u64().unwrap(),
            )
        }),
    );
}
fn assert_frequency(graph: &Dfg, expected: &Value) {
    assert_counts(&graph.graph, &expected["graph"]);
    ichnos_golden::assert_map_eq(
        graph.start_activities.iter().map(|(a, n)| (a.as_str(), *n)),
        expected["start_activities"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(a, n)| (a.as_str(), n.as_u64().unwrap())),
    );
    ichnos_golden::assert_map_eq(
        graph.end_activities.iter().map(|(a, n)| (a.as_str(), *n)),
        expected["end_activities"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(a, n)| (a.as_str(), n.as_u64().unwrap())),
    );
}
fn summaries(graph: &PerformanceDfg) -> Value {
    json!(
        graph
            .graph
            .iter()
            .map(|((a, b), p)| json!([a,b,{
                "mean": p.mean, "median": p.median, "min": p.min,
                "max": p.max, "sum": p.sum, "stdev": p.stdev,
            }]))
            .collect::<Vec<_>>()
    )
}

#[test]
fn discovery_dfg_matches_reference() {
    for name in [
        "running-example-xes",
        "receipt-xes",
        "roadtraffic100traces-xes",
        "interleavings-receipt_even-csv",
        "interleavings-receipt_odd-csv",
        "empty",
        "intervals",
    ] {
        let g = golden("discovery", &format!("dfg-mining-{name}"));
        let expected: Value = g.expected_as();
        let (log, keys) = load(&g);
        let before = log.clone();
        let graph = dfg(&log, &keys, &Default::default()).unwrap();
        assert_frequency(&graph, &expected["dfg"]);
        assert_frequency(
            &dfg_typed(&log, &keys, &Default::default()).unwrap(),
            &expected["typed"],
        );
        assert_counts(
            &directly_follows_graph(&log, &keys, &Default::default())
                .unwrap()
                .graph,
            &expected["alias"],
        );
        for row in expected["frequency_options"].as_array().unwrap() {
            let options = DfgOptions {
                window: row["window"].as_u64().unwrap() as usize,
                keep_once_per_case: row["keep_once_per_case"].as_bool().unwrap(),
            };
            assert_counts(&dfg(&log, &keys, &options).unwrap().graph, &row["graph"]);
        }
        assert_eq!(
            json!(derive_minimum_self_distance(&log, &keys).unwrap()),
            expected["minimum_self_distance"],
            "{name}: minimum distance"
        );
        assert_counts(
            &eventually_follows_graph(&log, &keys, &Default::default()).unwrap(),
            &expected["eventually"],
        );
        for row in expected["eventually_options"].as_array().unwrap() {
            let options = EventuallyFollowsOptions {
                use_start_timestamp: row["interval"].as_bool().unwrap(),
                keep_first_following: row["first"].as_bool().unwrap(),
            };
            assert_counts(
                &eventually_follows_graph(&log, &keys, &options).unwrap(),
                &row["graph"],
            );
        }
        let performance = performance_dfg(&log, &keys, &Default::default()).unwrap();
        assert_json_eq(
            &json!({"graph": summaries(&performance), "start_activities": performance.start_activities,
                              "end_activities": performance.end_activities}),
            &expected["performance"],
            &Default::default(),
        );
        for row in expected["performance_options"].as_array().unwrap() {
            let schedule = row["business"].as_bool().unwrap().then(|| BusinessHours {
                non_working_dates: g.meta().params["excluded_dates"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| s.as_str().unwrap().parse().unwrap())
                    .collect(),
                ..Default::default()
            });
            let options = PerformanceDfgOptions {
                use_start_timestamp: row["interval"].as_bool().unwrap(),
                business_hours: schedule,
                keep_raw_values: row["raw_values"].is_array(),
            };
            let actual = performance_dfg(&log, &keys, &options).unwrap();
            assert_json_eq(&summaries(&actual), &row["graph"], &Default::default());
            for (i, p) in actual.graph.values().enumerate() {
                for (aggregation, key) in [
                    (Aggregation::Mean, "mean"),
                    (Aggregation::Median, "median"),
                    (Aggregation::Min, "min"),
                    (Aggregation::Max, "max"),
                    (Aggregation::Sum, "sum"),
                    (Aggregation::StandardDeviation, "stdev"),
                ] {
                    ichnos_golden::assert_close(
                        p.aggregate(aggregation),
                        row["graph"][i][2][key].as_f64().unwrap(),
                        ichnos_golden::Tolerance::METRIC,
                    );
                }
                if let Some(raw) = &p.raw_values {
                    assert_eq!(p.count, raw.len());
                } else {
                    assert!(p.count > 0);
                }
            }
            if row["raw_values"].is_array() {
                let raw: std::collections::BTreeMap<_, _> = actual
                    .graph
                    .iter()
                    .map(|(edge, p)| (edge.clone(), p.raw_values.as_ref().unwrap()))
                    .collect();
                assert_json_eq(&edges(&raw), &row["raw_values"], &Default::default());
            }
            assert_eq!(
                actual.business_hours.is_some(),
                options.business_hours.is_some()
            );
        }
        assert_eq!(log, before, "{name}: input mutated");
    }
}
