mod common;
use common::load;
use ichnos_core::{AttributeValue, EventKeys, EventLog};
use ichnos_golden::{assert_multiset_eq, golden};
use ichnos_stats::{attributes::Scalar, filters::*};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn strings(v: &Value) -> Vec<String> {
    serde_json::from_value(v.clone()).unwrap()
}
fn scalar(v: &Value) -> Scalar {
    if let Some(s) = v.as_str() {
        Scalar::String(s.to_owned())
    } else if let Some(n) = v.as_i64() {
        Scalar::Int(n)
    } else {
        panic!("unexpected scalar {v}")
    }
}
fn rows(log: &EventLog) -> Vec<String> {
    log.traces.iter().map(|t| json!({
        "id": t.attributes.get("concept:name").map(ToString::to_string).unwrap_or_default(),
        "subcase": t.attributes.get("case:concept:name").map(ToString::to_string),
        "indices": t.events.iter().map(|e| e.get("@@index").unwrap().as_i64().unwrap().to_string()).collect::<Vec<_>>().join(",")
    }).to_string()).collect()
}

fn apply(
    log: &EventLog,
    keys: &EventKeys,
    function: &str,
    p: &Value,
) -> ichnos_stats::Result<EventLog> {
    let retention = |v: bool| {
        if v {
            Retention::Retain
        } else {
            Retention::Exclude
        }
    };
    let retain = retention(p["retain"].as_bool().unwrap_or(true));
    let text = |k: &str| p[k].as_str().unwrap();
    let number = |k: &str| p[k].as_f64().unwrap();
    let size = |k: &str| p[k].as_u64().unwrap() as usize;
    let range = || DurationRange {
        minimum: number("min_performance"),
        maximum: number("max_performance"),
    };
    let slice = || SliceOptions {
        strict: p["strict"].as_bool().unwrap(),
        occurrence: if text("first_or_last") == "first" {
            Occurrence::First
        } else {
            Occurrence::Last
        },
    };
    match function {
        "filter_start_activities" => {
            filter_start_activities(log, keys, &strings(&p["activities"]), retain)
        }
        "filter_end_activities" => {
            filter_end_activities(log, keys, &strings(&p["activities"]), retain)
        }
        "filter_event_attribute_values" => filter_event_attribute_values(
            log,
            text("attribute_key"),
            &p["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(scalar)
                .collect::<Vec<_>>(),
            if text("level") == "event" {
                FilterLevel::Events
            } else {
                FilterLevel::Cases
            },
            retain,
        ),
        "filter_trace_attribute_values" => filter_trace_attribute_values(
            log,
            text("attribute_key"),
            &p["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(scalar)
                .collect::<Vec<_>>(),
            retain,
        ),
        "filter_variants" => filter_variants(
            log,
            keys,
            &serde_json::from_value::<Vec<Vec<String>>>(p["variants"].clone()).unwrap(),
            retain,
        ),
        "filter_directly_follows_relation" => filter_directly_follows_relation(
            log,
            keys,
            &serde_json::from_value::<Vec<(String, String)>>(p["relations"].clone()).unwrap(),
            retain,
        ),
        "filter_eventually_follows_relation" => filter_eventually_follows_relation(
            log,
            keys,
            &serde_json::from_value::<Vec<Vec<String>>>(p["relations"].clone()).unwrap(),
            retain,
        ),
        "filter_paths_performance" => {
            let path = strings(&p["path"]);
            filter_paths_performance(
                log,
                keys,
                (&path[0], &path[1]),
                range(),
                retention(p["keep"].as_bool().unwrap()),
            )
        }
        "filter_four_eyes_principle" => filter_four_eyes_principle(
            log,
            keys,
            text("activity1"),
            text("activity2"),
            retention(!p["keep_violations"].as_bool().unwrap()),
        ),
        "filter_activity_done_different_resources" => filter_activity_done_different_resources(
            log,
            keys,
            text("activity"),
            retention(p["keep_violations"].as_bool().unwrap()),
        ),
        "filter_trace_segments" => {
            let patterns = serde_json::from_value::<Vec<Vec<String>>>(p["admitted_traces"].clone())
                .unwrap()
                .into_iter()
                .map(|p| {
                    p.into_iter()
                        .map(|a| {
                            if a == "..." {
                                SegmentToken::AnyActivities
                            } else {
                                SegmentToken::Activity(a)
                            }
                        })
                        .collect()
                })
                .collect::<Vec<_>>();
            filter_trace_segments(
                log,
                keys,
                &patterns,
                retention(p["positive"].as_bool().unwrap()),
            )
        }
        "filter_log_relative_occurrence_event_attribute" => {
            filter_log_relative_occurrence_event_attribute(
                log,
                "concept:name",
                number("min_relative_stake"),
                if text("level") == "cases" {
                    FilterLevel::Cases
                } else {
                    FilterLevel::Events
                },
            )
        }
        "filter_case_size" => filter_case_size(log, size("min_size"), size("max_size")),
        "filter_case_performance" => filter_case_performance(log, keys, range()),
        "filter_activities_rework" => {
            filter_activities_rework(log, keys, text("activity"), size("min_occurrences"))
        }
        "filter_variants_top_k" => filter_variants_top_k(log, keys, size("k")),
        "filter_variants_by_coverage_percentage" => {
            filter_variants_by_coverage_percentage(log, keys, number("min_coverage_percentage"))
        }
        "filter_prefixes" => filter_prefixes(log, keys, text("activity"), slice()),
        "filter_suffixes" => filter_suffixes(log, keys, text("activity"), slice()),
        "filter_between" => filter_between(
            log,
            keys,
            &strings(&p["act1"]),
            &strings(&p["act2"]),
            &BetweenOptions {
                case_id_attribute: "case:concept:name".into(),
                ..BetweenOptions::default()
            },
        ),
        "filter_time_range" => filter_time_range(
            log,
            keys,
            TimeRange {
                start: ichnos_core::chrono::DateTime::parse_from_rfc3339(text("dt1")).unwrap(),
                end: ichnos_core::chrono::DateTime::parse_from_rfc3339(text("dt2")).unwrap(),
                mode: match text("mode") {
                    "events" => TimeRangeMode::Events,
                    "traces_contained" => TimeRangeMode::Contained,
                    "traces_intersecting" => TimeRangeMode::Intersecting,
                    s if s.starts_with("traces_starting") => TimeRangeMode::Starting,
                    _ => TimeRangeMode::Completing,
                },
                retention: retention(!text("mode").ends_with("_exclude")),
            },
        ),
        _ => panic!("unhandled filter {function}"),
    }
}
fn edge_log() -> EventLog {
    let mut log = EventLog::default();
    let base = ichnos_core::chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
    let mut index = 0_i64;
    for (i, (activities, resources)) in [
        (
            vec!["A", "B", "A", "B"],
            vec![Some("r1"), Some("r2"), Some("r2"), Some("r1")],
        ),
        (
            vec!["A", "A", "B"],
            vec![Some("r1"), Some("r1"), Some("r1")],
        ),
        (vec!["B"], vec![None]),
        (vec!["A", "B"], vec![Some("r1"), Some("r2")]),
        (vec![], vec![]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut trace = ichnos_core::Trace::new();
        trace.attributes.insert("concept:name", format!("s{i}"));
        trace.attributes.insert("marker", "kept");
        for (activity, resource) in activities.into_iter().zip(resources) {
            let mut event = ichnos_core::Event::new();
            event.attributes.insert("concept:name", activity);
            event.attributes.insert("@@index", index);
            event.attributes.insert(
                "time:timestamp",
                AttributeValue::Date(base + ichnos_core::chrono::Duration::seconds(index * 5)),
            );
            if let Some(resource) = resource {
                event.attributes.insert("org:resource", resource);
            }
            trace.events.push(event);
            index += 1;
        }
        log.traces.push(trace);
    }
    log
}
#[test]
fn between_default_preserves_distinct_case_ids() {
    let log = edge_log();
    let keys = EventKeys::default();
    let result = filter_between(
        &log,
        &keys,
        &["A".into()],
        &["B".into()],
        &BetweenOptions::default(),
    )
    .unwrap();
    let ids = result
        .traces
        .iter()
        .map(|t| t.case_id().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["s0##@@0", "s0##@@1", "s1##@@0", "s3##@@0"]);
    let descriptions =
        ichnos_stats::cases::get_cases_description(&result, &keys, &Default::default()).unwrap();
    assert_eq!(descriptions.len(), 4);
    assert_eq!(
        ichnos_stats::cases::index_log_caseid(&result, "concept:name")
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn relative_occurrence_case_basis_still_filters_events() {
    let result = filter_log_relative_occurrence_event_attribute(
        &edge_log(),
        "concept:name",
        0.8,
        FilterLevel::Cases,
    )
    .unwrap();
    assert_eq!(result.traces.len(), 4);
    assert!(
        result.traces.iter().flat_map(|t| &t.events).all(|e| e
            .get("concept:name")
            .unwrap()
            .to_string()
            == "B")
    );
}

#[test]
fn oracle_log_filters() {
    for name in [
        "running-example",
        "receipt",
        "roadtraffic100traces",
        "edges",
    ] {
        let log = if name == "edges" {
            edge_log()
        } else {
            load(name)
        };
        let keys = EventKeys::default();
        let e: Value = golden("stats", &format!("filters-log-{name}")).expected_as();
        for case in e.as_array().unwrap() {
            let function = case["function"].as_str().unwrap();
            let actual = apply(&log, &keys, function, &case["params"])
                .unwrap_or_else(|e| panic!("{name} {function} {}: {e}", case["params"]));
            if case["result"].is_object() {
                let mut actual_rows = rows(&actual);
                actual_rows.sort();
                let values = actual_rows
                    .iter()
                    .map(|r| serde_json::from_str::<Value>(r).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(
                    json!({
                        "case_count": values.len(),
                        "rows_sha256": format!("{:x}", Sha256::digest(serde_json::to_vec(&values).unwrap())),
                        "sample": values.iter().take(3).collect::<Vec<_>>()
                    }),
                    case["result"],
                    "{name}: {function}: {}",
                    case["params"]
                );
            } else {
                let expected = case["result"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>();
                std::panic::catch_unwind(|| assert_multiset_eq(rows(&actual), expected))
                    .unwrap_or_else(|_| panic!("{name}: {function}: {}", case["params"]));
            }
            assert_eq!(actual.attributes, log.attributes);
            assert_eq!(actual.extensions, log.extensions);
            assert_eq!(actual.globals, log.globals);
            assert_eq!(actual.classifiers, log.classifiers);
            // Every resulting case keeps source trace attributes except between's subcase identifier.
            for trace in &actual.traces {
                let id = trace.attributes.get("concept:name").unwrap().to_string();
                let id = if function == "filter_between" {
                    id.split("##@@").next().unwrap()
                } else {
                    &id
                };
                let original = log
                    .traces
                    .iter()
                    .find(|t| t.attributes.get("concept:name").unwrap().to_string() == id)
                    .unwrap();
                let mut attrs = trace.attributes.clone();
                if function == "filter_between" {
                    attrs.remove("case:concept:name");
                }
                attrs.insert(
                    "concept:name",
                    original.attributes.get("concept:name").unwrap().clone(),
                );
                assert_eq!(attrs, original.attributes);
            }
        }
    }
}
fn dfg(v: &Value) -> ichnos_model::dfg::Dfg {
    let mut d = ichnos_model::dfg::Dfg::new();
    for e in v["edges"].as_array().unwrap() {
        d.add_edge(
            e[0].as_str().unwrap(),
            e[1].as_str().unwrap(),
            e[2].as_u64().unwrap(),
        );
    }
    for (a, n) in v["start"].as_object().unwrap() {
        d.add_start(a.as_str(), n.as_u64().unwrap());
    }
    for (a, n) in v["end"].as_object().unwrap() {
        d.add_end(a.as_str(), n.as_u64().unwrap());
    }
    d
}
#[test]
fn oracle_dfg_filters() {
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        let e: Value = golden("stats", &format!("filters-dfg-{name}")).expected_as();
        let input = dfg(&e["input"]);
        for case in e["filters"].as_array().unwrap() {
            let p = case["percentage"].as_f64().unwrap();
            let actual = if case["function"] == "filter_dfg_activities_percentage" {
                filter_dfg_activities_percentage(&input, p)
            } else {
                filter_dfg_paths_percentage(&input, p)
            }
            .unwrap();
            assert_eq!(actual, dfg(&case["result"]), "{name}: {case}");
        }
    }
}
#[test]
fn boundaries_metadata_and_errors() {
    let keys = EventKeys::default();
    let mut log = EventLog::from_trace_strings(["A,B,A", "A"], ",", &keys);
    log.traces.push(ichnos_core::Trace::new());
    log.attributes
        .insert("metadata", AttributeValue::from("retained"));
    assert_eq!(filter_case_size(&log, 0, 0).unwrap().len(), 1);
    let prefixes = filter_prefixes(&log, &keys, "A", SliceOptions::default()).unwrap();
    assert!(prefixes.traces.iter().all(|t| t.is_empty()));
    assert_eq!(prefixes.attributes, log.attributes);
    assert!(filter_case_size(&log, 2, 1).is_err());
    assert!(filter_variants_by_coverage_percentage(&log, &keys, f64::NAN).is_err());
    assert!(filter_dfg_paths_percentage(&ichnos_model::dfg::Dfg::new(), 1.1).is_err());
    // Literal commas and regex punctuation remain one activity in typed segment patterns.
    let comma = EventLog::from_trace_strings(["a,b|x.*"], "|", &keys);
    let patterns = vec![vec![
        SegmentToken::Activity("a,b".into()),
        SegmentToken::Activity("x.*".into()),
    ]];
    assert_eq!(
        filter_trace_segments(&comma, &keys, &patterns, Retention::Retain)
            .unwrap()
            .len(),
        1
    );
    // Absent event attributes are nonmatches, rather than causing Python's KeyError.
    assert_eq!(
        filter_event_attribute_values(&log, "absent", &[], FilterLevel::Events, Retention::Exclude)
            .unwrap()
            .len(),
        2
    );
    log.traces[0].events[0].attributes.remove(&keys.timestamp);
    assert!(
        filter_case_performance(
            &log,
            &keys,
            DurationRange {
                minimum: 0.0,
                maximum: 10.0
            }
        )
        .is_err()
    );
}
