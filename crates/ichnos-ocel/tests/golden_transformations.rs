//! Per-entry pm4py OCEL transformation goldens.
use chrono::{DateTime, SecondsFormat};
use ichnos_core::{AttributeValue, Attributes};
use ichnos_golden::{cases, golden};
use ichnos_ocel::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
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
fn attr(v: &AttributeValue) -> Value {
    match v.plain() {
        AttributeValue::String(s) | AttributeValue::Id(s) => json!(["string", s]),
        AttributeValue::Int(n) => json!(["int", n]),
        AttributeValue::Float(n) => json!(["float", n]),
        AttributeValue::Bool(b) => json!(["boolean", b]),
        AttributeValue::Date(t) => json!(["date", t.to_rfc3339_opts(SecondsFormat::Micros, false)]),
        other => panic!("unexpected {other:?}"),
    }
}
fn attrs(a: &Attributes) -> Value {
    Value::Object(a.iter().map(|(k, v)| (k.to_string(), attr(v))).collect())
}
fn wire(log: &Ocel, merge: bool) -> Value {
    let mut names = BTreeMap::new();
    for e in &log.events {
        let next = format!("event-{}", names.len());
        names.entry(e.id.clone()).or_insert(next);
    }
    let eid = |id: &Arc<str>| {
        if merge {
            names[id].clone()
        } else {
            id.to_string()
        }
    };
    let mut o2o = log.o2o.clone();
    o2o.sort_by_key(|r| {
        (
            r.source.clone(),
            r.target.clone(),
            r.qualifier.clone().unwrap_or_default(),
        )
    });
    let events: Vec<_> = log
        .events
        .iter()
        .map(|event| {
            json!({
                "id": eid(&event.id),
                "activity": event.activity,
                "timestamp": event.timestamp.to_rfc3339_opts(SecondsFormat::Micros, false),
                "attributes": attrs(&event.attributes),
            })
        })
        .collect();
    let objects: Vec<_> = log.objects.iter().map(|object| {
        json!({"id": object.id, "type": object.object_type, "attributes": attrs(&object.attributes)})
    }).collect();
    let relations: Vec<_> = log.relations.iter().map(|relation| {
        json!({"event": eid(&relation.event), "object": relation.object, "qualifier": relation.qualifier})
    }).collect();
    let o2o: Vec<_> = o2o.iter().map(|relation| {
        json!({"source": relation.source, "target": relation.target, "qualifier": relation.qualifier})
    }).collect();
    let e2e: Vec<_> = log.e2e.iter().map(|relation| {
        json!({"source": relation.source, "target": relation.target, "qualifier": relation.qualifier})
    }).collect();
    let changes: Vec<_> = log
        .object_changes
        .iter()
        .map(|change| {
            json!({
                "object": change.object,
                "type": change.object_type,
                "timestamp": change.timestamp.to_rfc3339_opts(SecondsFormat::Micros, false),
                "field": change.field,
                "value": change.value.as_ref().map(attr),
            })
        })
        .collect();
    json!({"events": events, "objects": objects, "relations": relations,
           "o2o": o2o, "e2e": e2e, "object_changes": changes})
}
fn event_field(s: &str) -> EventField<'_> {
    match s {
        "ocel:eid" => EventField::Id,
        "ocel:activity" => EventField::Activity,
        "ocel:timestamp" => EventField::Timestamp,
        s => EventField::Attribute(s),
    }
}
fn object_field(s: &str) -> ObjectField<'_> {
    match s {
        "ocel:oid" => ObjectField::Id,
        "ocel:type" => ObjectField::Type,
        s => ObjectField::Attribute(s),
    }
}
fn graph(s: &str) -> ObjectGraphKind {
    s.strip_suffix("_graph")
        .and_then(ObjectGraphKind::from_name)
        .unwrap_or_else(|| panic!("{s}"))
}
fn result(input: &Ocel, s: &Value) -> Value {
    let mut log = input.clone();
    let args = &s["args"];
    let kw = &s["kwargs"];
    let arg = |i| args[i].as_str().unwrap();
    match s["prep"].as_str() {
        Some("drill") => {
            let f = if log
                .objects
                .iter()
                .any(|o| o.attributes.contains_key("category"))
            {
                ObjectField::Attribute("category")
            } else {
                ObjectField::Id
            };
            log = ocel_drill_down(&log, arg(0), f).unwrap();
        }
        Some("unfold") => log = ocel_unfold(&log, arg(0), arg(1), None).unwrap(),
        _ => {}
    }
    let function = s["function"].as_str().unwrap();
    if function == "cluster_equivalent_ocel" {
        let options = EquivalentOcelOptions {
            object_type: arg(0),
            max_objects: kw["max_objs"].as_u64().map(|n| n as usize),
            exclude_object_types_from_renaming: kw["exclude_object_types_from_renaming"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_str().unwrap()).collect())
                .unwrap_or_default(),
        };
        let clusters = cluster_equivalent_ocel(&log, &options).unwrap();
        let mut rows: Vec<_> = clusters.iter().map(|(key, executions)| {
            let centers: Vec<_> = executions.iter().map(|execution| &execution.central_object).collect();
            let logs: Vec<_> = executions.iter().map(|execution| wire(&execution.log, false)).collect();
            json!({"description": [key.events, key.objects], "central_objects": centers, "logs": logs})
        }).collect();
        rows.sort_by_key(|v| v["description"].to_string());
        let mut expected = s["output"].as_array().unwrap().clone();
        expected.sort_by_key(|v| v["description"].to_string());
        assert_eq!(rows, expected, "cluster result");
        return Value::Array(expected);
    }
    let output = match function {
        "ocel_o2o_enrichment" => {
            let graphs: Option<Vec<_>> = kw["included_graphs"]
                .as_array()
                .map(|a| a.iter().map(|v| graph(v.as_str().unwrap())).collect());
            ocel_o2o_enrichment(&log, graphs.as_deref())
        }
        "ocel_e2o_lifecycle_enrichment" => ocel_e2o_lifecycle_enrichment(&log),
        "ocel_drop_duplicates" => ocel_drop_duplicates(&log).unwrap(),
        "ocel_merge_duplicates" => {
            ocel_merge_duplicates(&log, kw["have_common_object"].as_bool().unwrap()).unwrap()
        }
        "ocel_sort_by_additional_column" => ocel_sort_by_additional_column(
            &log,
            event_field(arg(0)),
            event_field(kw["primary_column"].as_str().unwrap_or("ocel:timestamp")),
        )
        .unwrap(),
        "ocel_add_index_based_timedelta" => ocel_add_index_based_timedelta(&log).unwrap(),
        "ocel_drill_down" => ocel_drill_down(&log, arg(0), object_field(arg(1))).unwrap(),
        "ocel_roll_up" => ocel_roll_up(
            &log,
            arg(0),
            args.get(1).map(|v| object_field(v.as_str().unwrap())),
        )
        .unwrap(),
        "ocel_unfold" => {
            let qs: Option<Vec<_>> = kw["qualifiers"]
                .as_array()
                .map(|a| a.iter().map(Value::as_str).collect());
            ocel_unfold(&log, arg(0), arg(1), qs.as_deref()).unwrap()
        }
        "ocel_fold" => ocel_fold(&log, arg(0), arg(1)),
        s => panic!("{s}"),
    };
    wire(&output, function == "ocel_merge_duplicates")
}
#[test]
fn transformation_goldens() {
    let ids = cases("ocel_transformations");
    assert_eq!(ids.len(), 16);
    for id in ids {
        let g = golden("ocel_transformations", &id);
        let log = build(&g.expected["input"]);
        if id.starts_with("sample-") {
            continue;
        }
        for scenario in g.expected["results"].as_array().unwrap() {
            assert_eq!(
                result(&log, scenario),
                scenario["output"],
                "{id}: {scenario}"
            );
        }
        if let Some(variants) = g.expected["variants"].as_array() {
            for variant in variants {
                let input = build(&variant["input"]);
                for scenario in variant["results"].as_array().unwrap() {
                    assert_eq!(
                        result(&input, scenario),
                        scenario["output"],
                        "{id} variant: {scenario}"
                    );
                }
            }
        }
    }
}
#[test]
fn sampling_matches_oracle_outcome_space() {
    for components in [false, true] {
        let id = if components {
            "sample-ocel-connected-components"
        } else {
            "sample-ocel-objects"
        };
        let g = golden("ocel_transformations", id);
        let log = build(&g.expected["input"]);
        for s in g.expected["results"].as_array().unwrap() {
            let kw = &s["kwargs"];
            let outputs = s["outputs"].as_array().unwrap();
            let mut seen = BTreeSet::new();
            for seed in 0..512 {
                let mut rng = ChaCha8Rng::seed_from_u64(seed);
                let sampled = if components {
                    sample_ocel_connected_components(
                        &log,
                        ComponentSampleOptions {
                            count: kw["connected_components"].as_u64().unwrap() as usize,
                            max_events: kw["max_num_events_per_cc"]
                                .as_u64()
                                .map(|n| n as usize)
                                .unwrap_or(usize::MAX),
                            max_objects: kw["max_num_objects_per_cc"]
                                .as_u64()
                                .map(|n| n as usize)
                                .unwrap_or(usize::MAX),
                            max_relations: kw["max_num_e2o_relations_per_cc"]
                                .as_u64()
                                .map(|n| n as usize)
                                .unwrap_or(usize::MAX),
                        },
                        &mut rng,
                    )
                } else {
                    sample_ocel_objects(
                        &log,
                        kw["num_objects"].as_u64().unwrap() as usize,
                        &mut rng,
                    )
                };
                let output = wire(&sampled, false);
                assert!(outputs.contains(&output), "{id} seed {seed}: {output}");
                seen.insert(output.to_string());
            }
            let valid: BTreeSet<_> = outputs.iter().map(Value::to_string).collect();
            assert_eq!(
                seen, valid,
                "{id}: every oracle outcome should be reachable"
            );
        }
    }
}
#[test]
fn typed_errors_and_metadata() {
    let g = golden("ocel_transformations", "ocel-drill-down");
    let mut log = build(&g.expected["input"]);
    log.globals.insert("marker", "kept");
    log.naive_times = true;
    let original = log.clone();
    assert!(matches!(
        ocel_drill_down(&log, "absent", ObjectField::Id),
        Err(TransformationError::MissingObjectType(_))
    ));
    assert!(matches!(
        ocel_drill_down(&log, "item", ObjectField::Attribute("absent")),
        Err(TransformationError::MissingAttribute(_))
    ));
    assert!(matches!(
        ocel_roll_up(&log, "item", Some(ObjectField::Attribute("absent"))),
        Err(TransformationError::MissingAttribute(_))
    ));
    assert!(matches!(
        ocel_sort_by_additional_column(
            &log,
            EventField::Attribute("absent"),
            EventField::Timestamp
        ),
        Err(TransformationError::MissingAttribute(_))
    ));
    log.events[0].attributes.insert("mixed", 1);
    log.events[1].attributes.insert("mixed", "string");
    assert!(matches!(
        ocel_sort_by_additional_column(&log, EventField::Attribute("mixed"), EventField::Timestamp),
        Err(TransformationError::IncomparableField(_))
    ));
    let mut dangling = original.clone();
    dangling.relations[0].event = "absent".into();
    assert!(matches!(
        ocel_drop_duplicates(&dangling),
        Err(TransformationError::MissingEvent(_))
    ));
    assert!(matches!(
        ocel_merge_duplicates(&dangling, false),
        Err(TransformationError::MissingEvent(_))
    ));
    dangling = original.clone();
    dangling.relations[0].object = "absent".into();
    assert!(matches!(
        ocel_unfold(&dangling, "create", "item", None),
        Err(TransformationError::MissingObject(_))
    ));
    let mut unrelated = original.clone();
    let mut orphan = unrelated.events[0].clone();
    orphan.id = "orphan".into();
    unrelated.events.push(orphan);
    assert!(matches!(
        ocel_merge_duplicates(&unrelated, false),
        Err(TransformationError::UnrelatedEvent(_))
    ));
    let mut overflow = original.clone();
    overflow.events[1].timestamp = chrono::DateTime::<chrono::Utc>::MAX_UTC.fixed_offset();
    assert!(matches!(
        ocel_add_index_based_timedelta(&overflow),
        Err(TransformationError::TimestampOverflow(_))
    ));
    let outputs = vec![
        ocel_o2o_enrichment(&original, None),
        ocel_e2o_lifecycle_enrichment(&original),
        ocel_drop_duplicates(&original).unwrap(),
        ocel_merge_duplicates(&original, false).unwrap(),
        ocel_sort_by_additional_column(&original, EventField::Activity, EventField::Timestamp)
            .unwrap(),
        ocel_add_index_based_timedelta(&original).unwrap(),
        ocel_drill_down(&original, "item", ObjectField::Id).unwrap(),
        ocel_roll_up(&original, "item", None).unwrap(),
        ocel_unfold(&original, "create", "item", None).unwrap(),
        ocel_fold(&original, "create", "item"),
        sample_ocel_objects(&original, 2, &mut ChaCha8Rng::seed_from_u64(7)),
    ];
    for mut output in outputs {
        assert_eq!(output.globals, original.globals);
        assert!(output.naive_times);
        output.globals.insert("marker", "changed");
        assert_eq!(
            original.globals.get("marker").unwrap().as_str(),
            Some("kept")
        );
    }
    let component = sample_ocel_connected_components(
        &original,
        ComponentSampleOptions::default(),
        &mut ChaCha8Rng::seed_from_u64(7),
    );
    assert!(component.globals.is_empty());
    assert!(component.o2o.is_empty());
    assert!(!component.naive_times);
}
#[test]
fn lifecycle_pairs_avoid_separator_collisions() {
    let g = golden("ocel_transformations", "ocel-e2o-lifecycle-enrichment");
    let mut log = build(&g.expected["input"]);
    // Both concatenate to a@@b@@c in pm4py, but remain distinct pairs here.
    log.relations = vec![
        EventObject {
            event: "first".into(),
            object: "c".into(),
            qualifier: None,
        },
        EventObject {
            event: "a@@b".into(),
            object: "c".into(),
            qualifier: None,
        },
        EventObject {
            event: "a".into(),
            object: "b@@c".into(),
            qualifier: None,
        },
    ];
    let enriched = ocel_e2o_lifecycle_enrichment(&log);
    assert_eq!(
        enriched.relations[1].qualifier.as_deref(),
        Some("termination")
    );
    assert_eq!(enriched.relations[2].qualifier.as_deref(), Some("creation"));
}
