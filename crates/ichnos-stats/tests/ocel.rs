//! Per-entry OCEL statistics goldens and full real-log comparisons.
use ichnos_core::chrono::{DateTime, NaiveDate};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_golden::{JsonCompare, assert_json_eq, cases, golden};
use ichnos_ocel::{
    EventEvent, EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject,
};
use ichnos_stats::{Error, ocel::*, time::BusinessHours};
use serde_json::{Value, json};
use std::sync::Arc;
fn text(v: &Value) -> Arc<str> {
    v.as_str().expect("string").into()
}

fn qualifier(v: &Value) -> Option<Arc<str>> {
    v.as_str().map(Into::into)
}

fn value(v: &Value) -> AttributeValue {
    let pair = v.as_array().expect("[type, value]");
    let x = &pair[1];
    match pair[0].as_str().expect("type") {
        "string" => AttributeValue::String(text(x)),
        "int" => AttributeValue::Int(x.as_i64().expect("int")),
        "float" => AttributeValue::Float(x.as_f64().expect("float")),
        "boolean" => AttributeValue::Bool(x.as_bool().expect("bool")),
        "date" => AttributeValue::Date(
            DateTime::parse_from_rfc3339(x.as_str().expect("date")).expect("date"),
        ),
        other => panic!("unknown type {other}"),
    }
}

fn attributes(v: &Value) -> Attributes {
    v.as_object()
        .expect("attributes")
        .iter()
        .map(|(k, v)| (k.as_str(), value(v)))
        .collect()
}

fn rows(v: &Value, key: &str) -> Vec<Value> {
    v[key].as_array().expect(key).clone()
}

fn timestamp(v: &Value) -> ichnos_core::chrono::DateTime<ichnos_core::chrono::FixedOffset> {
    DateTime::parse_from_rfc3339(v.as_str().expect("timestamp")).expect("timestamp")
}

fn build(v: &Value) -> Ocel {
    Ocel {
        events: rows(v, "events")
            .iter()
            .map(|e| OcelEvent {
                id: text(&e["id"]),
                activity: text(&e["activity"]),
                timestamp: timestamp(&e["timestamp"]),
                attributes: attributes(&e["attributes"]),
            })
            .collect(),
        objects: rows(v, "objects")
            .iter()
            .map(|o| OcelObject {
                id: text(&o["id"]),
                object_type: text(&o["type"]),
                attributes: attributes(&o["attributes"]),
            })
            .collect(),
        relations: rows(v, "relations")
            .iter()
            .map(|r| EventObject {
                event: text(&r["event"]),
                object: text(&r["object"]),
                qualifier: qualifier(&r["qualifier"]),
            })
            .collect(),
        o2o: rows(v, "o2o")
            .iter()
            .map(|r| ObjectObject {
                source: text(&r["source"]),
                target: text(&r["target"]),
                qualifier: qualifier(&r["qualifier"]),
            })
            .collect(),
        e2e: rows(v, "e2e")
            .iter()
            .map(|r| EventEvent {
                source: text(&r["source"]),
                target: text(&r["target"]),
                qualifier: qualifier(&r["qualifier"]),
            })
            .collect(),
        object_changes: rows(v, "object_changes")
            .iter()
            .map(|c| ObjectChange {
                object: text(&c["object"]),
                object_type: text(&c["type"]),
                timestamp: timestamp(&c["timestamp"]),
                field: text(&c["field"]),
                value: (!c["value"].is_null()).then(|| value(&c["value"])),
            })
            .collect(),
        globals: Attributes::default(),
        naive_times: false,
    }
}

fn edge_json<T: serde::Serialize>(values: &EdgeMetric<T>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(t, edges)| {
                let rows = if edges.is_empty() {
                    json!({})
                } else {
                    json!(
                        edges
                            .iter()
                            .map(|((a, b), v)| json!([[a, b], v]))
                            .collect::<Vec<_>>()
                    )
                };
                (t.to_string(), rows)
            })
            .collect(),
    )
}

fn filters() -> [(&'static str, Prefilter); 3] {
    [
        ("none", Prefilter::None),
        ("start", Prefilter::Start),
        ("end", Prefilter::End),
    ]
}

fn activity_result(log: &Ocel, id: &str) -> Value {
    let mut result = serde_json::Map::new();
    for (name, prefilter) in filters() {
        let output = if id.starts_with("act-ot-dependent-") {
            let a = act_ot_dependent::find_associations_from_ocel(log, prefilter).unwrap();
            if id.ends_with("aggregate-events") {
                json!(act_ot_dependent::aggregate_events(&a))
            } else if id.ends_with("aggregate-unique-objects") {
                json!(act_ot_dependent::aggregate_unique_objects(&a))
            } else if id.ends_with("aggregate-total-objects") {
                json!(act_ot_dependent::aggregate_total_objects(&a))
            } else {
                json!(a)
            }
        } else {
            let a = if id.ends_with("find-associations-from-relations-df") {
                let events = log.event_index();
                let rows: Vec<_> = log
                    .relations
                    .iter()
                    .map(|r| ActivityRelation {
                        event: r.event.clone(),
                        object: r.object.clone(),
                        activity: log.events[events[r.event.as_ref()]].activity.clone(),
                    })
                    .collect();
                act_utils::find_associations_from_relations_df(&rows, prefilter)
            } else {
                act_utils::find_associations_from_ocel(log, prefilter).unwrap()
            };
            if id.ends_with("aggregate-events") {
                json!(act_utils::aggregate_events(&a))
            } else if id.ends_with("aggregate-unique-objects") {
                json!(act_utils::aggregate_unique_objects(&a))
            } else if id.ends_with("aggregate-total-objects") {
                json!(act_utils::aggregate_total_objects(&a))
            } else {
                json!(a)
            }
        };
        result.insert(name.into(), output);
    }
    Value::Object(result)
}

fn schedules() -> Vec<(&'static str, Option<BusinessHours>)> {
    vec![
        ("elapsed", None),
        ("business", Some(BusinessHours::default())),
        (
            "slots",
            Some(BusinessHours {
                slots: vec![
                    (7 * 3600, 12 * 3600),
                    (12 * 3600 + 1, 17 * 3600),
                    (10 * 3600, 13 * 3600),
                ],
                ..Default::default()
            }),
        ),
        (
            "holiday",
            Some(BusinessHours {
                non_working_dates: [NaiveDate::from_ymd_opt(2024, 1, 8).unwrap()].into(),
                ..Default::default()
            }),
        ),
    ]
}

fn compute(log: &Ocel, id: &str) -> Value {
    if id.starts_with("act-") {
        return activity_result(log, id);
    }
    if id == "objects-ot-count-get-objects-ot-count" {
        return json!(objects_ot_count::get_objects_ot_count(log).unwrap());
    }
    if id == "ot-activities-get-object-type-activities" {
        return json!(ot_activities::get_object_type_activities(log).unwrap());
    }
    let edges = edge_metrics::find_associations_per_edge(log).unwrap();
    match id {
        "edge-metrics-find-associations-per-edge" => edge_json(&edges),
        "edge-metrics-aggregate-ev-couples" => {
            edge_json(&edge_metrics::aggregate_ev_couples(&edges))
        }
        "edge-metrics-aggregate-unique-objects" => {
            edge_json(&edge_metrics::aggregate_unique_objects(&edges))
        }
        "edge-metrics-aggregate-total-objects" => {
            edge_json(&edge_metrics::aggregate_total_objects(&edges))
        }
        "edge-metrics-performance-calculation-ocel-aggregation" => {
            let mut result = serde_json::Map::new();
            for (name, schedule) in schedules() {
                let pairs = edge_metrics::aggregate_ev_couples(&edges);
                let triples = edge_metrics::aggregate_total_objects(&edges);
                result.insert(
                    format!("pairs-{name}"),
                    edge_json(
                        &edge_metrics::performance_calculation_ocel_aggregation(
                            log,
                            &pairs,
                            schedule.as_ref(),
                        )
                        .unwrap(),
                    ),
                );
                result.insert(
                    format!("triples-{name}"),
                    edge_json(
                        &edge_metrics::performance_calculation_ocel_aggregation(
                            log,
                            &triples,
                            schedule.as_ref(),
                        )
                        .unwrap(),
                    ),
                );
            }
            Value::Object(result)
        }
        _ => panic!("unknown function {id}"),
    }
}

fn associations(v: &Value) -> ActivityAssociations {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(a, pairs)| {
            (
                a.as_str().into(),
                pairs
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| (text(&p[0]), text(&p[1])))
                    .collect(),
            )
        })
        .collect()
}
fn parse_edges(v: &Value) -> EdgeAssociations {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(t, edges)| {
            let rows = edges
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|e| {
                            (
                                (text(&e[0][0]), text(&e[0][1])),
                                e[1].as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|r| (text(&r[0]), text(&r[1]), text(&r[2])))
                                    .collect(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            (t.as_str().into(), rows)
        })
        .collect()
}

fn compute_direct(v: &Value, id: &str) -> Value {
    if id.starts_with("act-ot-dependent-") {
        let a: TypedActivityAssociations = v
            .as_object()
            .unwrap()
            .iter()
            .map(|(t, v)| (t.as_str().into(), associations(v)))
            .collect();
        if id.ends_with("aggregate-events") {
            json!(act_ot_dependent::aggregate_events(&a))
        } else if id.ends_with("aggregate-unique-objects") {
            json!(act_ot_dependent::aggregate_unique_objects(&a))
        } else {
            assert!(std::ptr::eq(
                act_ot_dependent::aggregate_total_objects(&a),
                &a
            ));
            json!(act_ot_dependent::aggregate_total_objects(&a))
        }
    } else if id.ends_with("find-associations-from-relations-df") {
        let rows: Vec<_> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|r| ActivityRelation {
                event: text(&r["ocel:eid"]),
                object: text(&r["ocel:oid"]),
                activity: text(&r["ocel:activity"]),
            })
            .collect();
        Value::Object(
            filters()
                .into_iter()
                .map(|(name, p)| {
                    (
                        name.into(),
                        json!(act_utils::find_associations_from_relations_df(&rows, p)),
                    )
                })
                .collect(),
        )
    } else if id.starts_with("act-utils-") {
        let a = associations(v);
        if id.ends_with("aggregate-events") {
            json!(act_utils::aggregate_events(&a))
        } else if id.ends_with("aggregate-unique-objects") {
            json!(act_utils::aggregate_unique_objects(&a))
        } else {
            assert!(std::ptr::eq(act_utils::aggregate_total_objects(&a), &a));
            json!(act_utils::aggregate_total_objects(&a))
        }
    } else {
        let edges = parse_edges(v);
        if id.ends_with("aggregate-ev-couples") {
            edge_json(&edge_metrics::aggregate_ev_couples(&edges))
        } else if id.ends_with("aggregate-unique-objects") {
            edge_json(&edge_metrics::aggregate_unique_objects(&edges))
        } else {
            edge_json(&edge_metrics::aggregate_total_objects(&edges))
        }
    }
}

#[test]
fn ocel_statistics_goldens() {
    let ids = cases("ocel_stats");
    assert_eq!(ids.len(), 19, "{ids:?}");
    for id in ids {
        let g = golden("ocel_stats", &id);
        let e = &g.expected;
        if id.starts_with("all-") {
            let log = build(&e["input"]);
            let original = log.clone();
            for (function, expected) in e["expected"].as_object().unwrap() {
                assert_json_eq(&compute(&log, function), expected, &JsonCompare::default());
            }
            assert_eq!(log, original, "{id}: input mutation");
        } else {
            for scenario in e["scenarios"].as_array().unwrap() {
                let log = build(&scenario["input"]);
                let original = log.clone();
                assert_json_eq(
                    &compute(&log, &id),
                    &scenario["expected"],
                    &JsonCompare::default(),
                );
                assert_eq!(log, original, "{id}: input mutation");
            }
            if let Some(direct) = e.get("direct") {
                assert_json_eq(
                    &compute_direct(&direct["input"], &id),
                    &direct["expected"],
                    &JsonCompare::default(),
                );
            }
        }
    }
}

#[test]
fn dangling_references_and_invalid_schedules_are_errors() {
    let mut log = Ocel::default();
    log.relations.push(EventObject {
        event: "absent-event".into(),
        object: "absent-object".into(),
        qualifier: None,
    });
    assert!(matches!(
        act_utils::find_associations_from_ocel(&log, Prefilter::None),
        Err(Error::MissingOcelEvent(_))
    ));
    assert!(matches!(
        act_ot_dependent::find_associations_from_ocel(&log, Prefilter::Start),
        Err(Error::MissingOcelEvent(_))
    ));
    assert!(matches!(
        objects_ot_count::get_objects_ot_count(&log),
        Err(Error::MissingOcelEvent(_))
    ));
    assert!(matches!(
        ot_activities::get_object_type_activities(&log),
        Err(Error::MissingOcelEvent(_))
    ));
    assert!(matches!(
        edge_metrics::find_associations_per_edge(&log),
        Err(Error::MissingOcelEvent(_))
    ));
    log.events.push(OcelEvent {
        id: "absent-event".into(),
        activity: "a".into(),
        timestamp: timestamp(&json!("2024-01-01T00:00:00Z")),
        attributes: Attributes::default(),
    });
    assert!(matches!(
        act_utils::find_associations_from_ocel(&log, Prefilter::None),
        Err(Error::MissingOcelObject(_))
    ));
    assert!(matches!(
        edge_metrics::find_associations_per_edge(&log),
        Err(Error::MissingOcelObject(_))
    ));
    let aggregation: EdgeMetric<std::collections::BTreeSet<Association>> = [(
        "type".into(),
        [(
            ("a".into(), "b".into()),
            [("absent-event".into(), "bad".into())].into(),
        )]
        .into(),
    )]
    .into();
    assert!(matches!(
        edge_metrics::performance_calculation_ocel_aggregation(&log, &aggregation, None),
        Err(Error::MissingOcelEvent(_))
    ));
    let invalid = BusinessHours {
        slots: vec![(3, 2)],
        ..Default::default()
    };
    assert!(matches!(
        edge_metrics::performance_calculation_ocel_aggregation::<Association>(
            &Ocel::default(),
            &Default::default(),
            Some(&invalid)
        ),
        Err(Error::InvalidOption(_))
    ));
}

/// `pm4py.ocel_object_type_activities` and `pm4py.ocel_objects_ot_count`
/// wrap the statistics above; their goldens sit with the other OCEL summaries.
#[test]
fn ocel_summary_wrappers_match_pm4py() {
    let ids = cases("ocel_summaries");
    assert!(ids.len() >= 7, "{ids:?}");
    for id in ids {
        let g = golden("ocel_summaries", &id);
        let e = &g.expected;
        let log = build(&e["input"]);
        let activities = ot_activities::get_object_type_activities(&log).unwrap();
        assert_eq!(
            serde_json::to_value(&activities).unwrap(),
            e["object_type_activities"]["ok"],
            "{id}"
        );
        let counts = objects_ot_count::get_objects_ot_count(&log).unwrap();
        assert_eq!(
            serde_json::to_value(&counts).unwrap(),
            e["objects_ot_count"]["ok"],
            "{id}"
        );
    }
}
