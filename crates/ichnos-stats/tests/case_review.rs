mod common;
use ichnos_core::{
    AttributeValue, EventKeys, EventLog,
    chrono::{DateTime, Duration},
};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_stats::{
    attributes::{KdeOptions, Scalar},
    cases::*,
    time::*,
};
use serde_json::{Value, json};
fn id(v: &Value) -> Scalar {
    if let Some(s) = v.as_str() {
        Scalar::String(s.into())
    } else if let Some(b) = v.as_bool() {
        Scalar::Bool(b)
    } else if let Some(i) = v.as_i64() {
        Scalar::Int(i)
    } else {
        Scalar::Float(v.as_f64().unwrap().to_bits())
    }
}
fn id_value(v: &Scalar) -> (Value, &'static str) {
    match v {
        Scalar::String(v) => (json!(v), "str"),
        Scalar::Bool(v) => (json!(v), "bool"),
        Scalar::Int(v) => (json!(v), "int"),
        Scalar::Float(v) => (json!(f64::from_bits(*v)), "float"),
        _ => panic!("unexpected ID"),
    }
}
#[test]
fn oracle_typed_case_identifiers() {
    let e: Value = golden("stats", "case-typed-ids").expected_as();
    let keys = EventKeys::default();
    for case in e.as_array().unwrap() {
        let mut log = EventLog::from_trace_strings(
            case["ids"].as_array().unwrap().iter().map(|_| "A,B"),
            ",",
            &keys,
        );
        let base = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
        for (i, (t, cid)) in log
            .traces
            .iter_mut()
            .zip(case["ids"].as_array().unwrap())
            .enumerate()
        {
            let value = match id(cid) {
                Scalar::String(v) => AttributeValue::String(v.into()),
                Scalar::Bool(v) => AttributeValue::Bool(v),
                Scalar::Int(v) => AttributeValue::Int(v),
                Scalar::Float(v) => AttributeValue::Float(f64::from_bits(v)),
                _ => unreachable!(),
            };
            t.attributes.insert("concept:name", value);
            for (j, event) in t.events.iter_mut().enumerate() {
                event.attributes.insert(
                    "time:timestamp",
                    AttributeValue::Date(base + Duration::seconds((i * 10 + j * (i + 1)) as i64)),
                );
            }
        }
        let options = CaseOptions {
            sort: case["sort"].as_bool().unwrap().then_some(CaseSort::Id),
            ..Default::default()
        };
        let rows = get_cases_description(&log, &keys, &options)
            .unwrap()
            .into_iter()
            .map(|(id, d)| {
                let (v, typ) = id_value(&id);
                json!({"id":v,"type":typ,"duration":d.case_duration})
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(rows), case["typed_descriptions"]);
        let rows = index_log_caseid(&log, "concept:name")
            .unwrap()
            .into_iter()
            .map(|(id, t)| {
                let (v, typ) = id_value(&id);
                let d = t.events[1]
                    .get("time:timestamp")
                    .unwrap()
                    .as_date()
                    .unwrap()
                    .signed_duration_since(
                        t.events[0]
                            .get("time:timestamp")
                            .unwrap()
                            .as_date()
                            .unwrap(),
                    )
                    .num_seconds();
                json!({"id":v,"type":typ,"duration":d as f64})
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(rows), case["indexed"]);
        if !case["sort"].as_bool().unwrap() {
            assert!(get_cases_description(&log, &keys, &CaseOptions::default()).is_err());
            assert_eq!(index_log_caseid(&log, "concept:name").unwrap().len(), 2);
        }
    }
}
fn relations(mut rows: Vec<EventRelation>) -> Value {
    rows.sort_by_key(|r| (r.trace, r.source, r.target));
    json!(rows.into_iter().map(|r|json!({"trace":r.trace,"source":r.source,"target":r.target,"duration":r.duration})).collect::<Vec<_>>())
}
#[test]
fn missing_case_time_oracles() {
    for name in [
        "running-example",
        "receipt",
        "roadtraffic100traces",
        "interval_event_log",
    ] {
        let log = common::load(name);
        let keys = EventKeys::default();
        let e: Value =
            golden("stats", &format!("case-review-{}", name.replace('_', "-"))).expected_as();
        let check = |key: &str, v: Value| {
            let mut expected = e[key].clone();
            if key.ends_with("_rows") && !key.starts_with("polars_variants") {
                expected = json!(expected.as_array().unwrap().iter().map(|row| {
                    let fields=row.as_str().unwrap().split(',').collect::<Vec<_>>();
                    json!({"trace":fields[0].parse::<usize>().unwrap(),"source":fields[1].parse::<usize>().unwrap(),"target":fields[2].parse::<usize>().unwrap(),"duration":fields[3].parse::<f64>().unwrap(),"precise_duration":fields[4].parse::<f64>().unwrap()})
                }).collect::<Vec<_>>());
                let rows = expected.as_array_mut().unwrap();
                if key == "concurrent_rows" {
                    rows.retain(|r| r["precise_duration"].as_f64().unwrap() >= 0.0);
                }
                for row in rows {
                    let precise = row["precise_duration"].as_f64().unwrap();
                    if key != "partial_business_rows" {
                        assert_eq!(precise.trunc(), row["duration"].as_f64().unwrap());
                    }
                    row["duration"] = json!(precise);
                    row.as_object_mut().unwrap().remove("precise_duration");
                }
            }
            assert_json_eq(&v, &expected, &JsonCompare::default());
        };
        let rows = get_variants_df(&log, &keys)
            .unwrap()
            .into_iter()
            .map(|v| json!({"case_id":id_value(&v.case_id).0,"variant":v.variant}))
            .collect::<Vec<_>>();
        check("pandas_variants", json!(rows));
        check("polars_variants", json!(rows));
        check("polars_variants_and_list_rows", json!(rows));
        let (_, list) = get_variants_df_and_list(&log, &keys).unwrap();
        check(
            "polars_variants_list",
            json!(
                list.into_iter()
                    .map(|v| (v.variant, v.count))
                    .collect::<Vec<_>>()
            ),
        );
        let curve = get_kde_caseduration(
            &log,
            &keys,
            &CaseOptions::default(),
            KdeOptions {
                graph_points: 20,
                ..Default::default()
            },
        )
        .unwrap();
        check(
            "kde_log_json",
            json!(curve.x.into_iter().zip(curve.y).collect::<Vec<_>>()),
        );
        let curve = get_kde_case_duration_values(
            &[0.0, 1.0, 4.0, 8.0],
            KdeOptions {
                graph_points: 20,
                ..Default::default()
            },
        )
        .unwrap();
        check(
            "kde_values_json",
            json!(curve.x.into_iter().zip(curve.y).collect::<Vec<_>>()),
        );
        for (i, graph_points) in [2, 3].into_iter().enumerate() {
            let curve = get_kde_case_duration_values(
                &[0.0, 1.0, 4.0, 8.0],
                KdeOptions {
                    graph_points,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_json_eq(
                &json!([curve.x, curve.y]),
                &e["kde_small"][i],
                &JsonCompare::default(),
            );
        }
        let time = TimeOptions {
            use_start_timestamp: name == "interval_event_log",
            ..Default::default()
        };
        for (activity, expected) in e["passed_algorithm"].as_object().unwrap() {
            let mut passed = get_passed_time(&log, &keys, activity, &time).unwrap();
            passed.pre.sort_by(|a, b| a.activity.cmp(&b.activity));
            let actual = json!({"pre":passed.pre.into_iter().map(|n|(n.activity,n.duration,n.count)).collect::<Vec<_>>(),"pre_avg_perf":passed.pre_avg_perf});
            let mut expected = expected.clone();
            expected["pre"]
                .as_array_mut()
                .unwrap()
                .sort_by(|a, b| a[0].as_str().cmp(&b[0].as_str()));
            assert_json_eq(&actual, &expected, &JsonCompare::default());
        }
        let mut options = RelationOptions {
            time,
            ..Default::default()
        };
        check(
            "concurrent_rows",
            relations(get_concurrent_events(&log, &keys, &options).unwrap()),
        );
        check(
            "partial_rows",
            relations(get_partial_order(&log, &keys, &options).unwrap()),
        );
        options.time.business_hours = Some(BusinessHours::default());
        check(
            "concurrent_rows",
            relations(get_concurrent_events(&log, &keys, &options).unwrap()),
        );
        check(
            "partial_business_rows",
            relations(get_partial_order(&log, &keys, &options).unwrap()),
        );
    }
}
