//! Complete row and auxiliary-table comparisons against pm4py OCEL filters.
use std::sync::Arc;

use chrono::DateTime;
use ichnos_core::{AttributeValue, Attributes};
use ichnos_golden::{cases, golden};
use ichnos_ocel::{
    EventEvent, EventObject, ObjectChange, ObjectObject, Ocel, OcelEvent, OcelObject, filtering::*,
};
use serde_json::Value;

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

fn timestamp(v: &Value) -> chrono::DateTime<chrono::FixedOffset> {
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

fn strings(v: &Value) -> Vec<&str> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect()
}
fn selection(v: &Value) -> Selection {
    if v["positive"].as_bool().unwrap_or(true) {
        Selection::Keep
    } else {
        Selection::Remove
    }
}
fn event_field(key: &str) -> EventField<'_> {
    match key {
        "ocel:eid" => EventField::Id,
        "ocel:activity" => EventField::Activity,
        "ocel:timestamp" => EventField::Timestamp,
        key => EventField::Attribute(key),
    }
}
fn time_field(key: &str) -> EventTimeField<'_> {
    match key {
        "ocel:timestamp" => EventTimeField::Timestamp,
        key => EventTimeField::DateAttribute(key),
    }
}
fn object_field(key: &str) -> ObjectField<'_> {
    match key {
        "ocel:oid" => ObjectField::Id,
        "ocel:type" => ObjectField::Type,
        key => ObjectField::Attribute(key),
    }
}
fn scalar(v: &Value) -> AttributeValue {
    match v {
        Value::String(s) => AttributeValue::string(s.as_str()),
        Value::Bool(b) => AttributeValue::Bool(*b),
        Value::Number(n) if n.is_i64() => AttributeValue::Int(n.as_i64().unwrap()),
        Value::Number(n) => AttributeValue::Float(n.as_f64().unwrap()),
        _ => panic!("unexpected scenario scalar"),
    }
}
fn apply(log: &Ocel, row: &Value) -> Ocel {
    let args = &row["args"];
    let kwargs = &row["kwargs"];
    let select = selection(kwargs);
    let options = ObjectFilterOptions {
        selection: select,
        level: kwargs["level"].as_u64().unwrap_or(1) as usize,
    };
    match row["function"].as_str().unwrap() {
        "filter_ocel_event_attribute" => filter_ocel_event_attribute(
            log,
            event_field(args[0].as_str().unwrap()),
            &args[1]
                .as_array()
                .unwrap()
                .iter()
                .map(scalar)
                .collect::<Vec<_>>(),
            select,
        ),
        "filter_ocel_object_attribute" => filter_ocel_object_attribute(
            log,
            object_field(args[0].as_str().unwrap()),
            &args[1]
                .as_array()
                .unwrap()
                .iter()
                .map(scalar)
                .collect::<Vec<_>>(),
            select,
        ),
        "filter_ocel_object_types_allowed_activities" => {
            filter_ocel_object_types_allowed_activities(
                log,
                &serde_json::from_value(args[0].clone()).unwrap(),
            )
        }
        "filter_ocel_object_per_type_count" => filter_ocel_object_per_type_count(
            log,
            &serde_json::from_value(args[0].clone()).unwrap(),
        ),
        "filter_ocel_start_events_per_object_type" => {
            filter_ocel_start_events_per_object_type(log, args[0].as_str().unwrap())
        }
        "filter_ocel_end_events_per_object_type" => {
            filter_ocel_end_events_per_object_type(log, args[0].as_str().unwrap())
        }
        "filter_ocel_events_timestamp" => filter_ocel_events_timestamp(
            log,
            timestamp(&args[0]),
            timestamp(&args[1]),
            time_field(kwargs["timestamp_key"].as_str().unwrap_or("ocel:timestamp")),
        ),
        "filter_ocel_object_types" => filter_ocel_object_types(log, &strings(&args[0]), options),
        "filter_ocel_objects" => filter_ocel_objects(log, &strings(&args[0]), options),
        "filter_ocel_events" => filter_ocel_events(log, &strings(&args[0]), select),
        "filter_ocel_activities_connected_object_type" => {
            filter_ocel_activities_connected_object_type(log, args[0].as_str().unwrap())
        }
        "filter_ocel_cc_object" => filter_ocel_cc_object(log, args[0].as_str().unwrap(), None),
        "filter_ocel_cc_length" => filter_ocel_cc_length(
            log,
            args[0].as_u64().unwrap() as usize,
            args[1].as_u64().unwrap() as usize,
        ),
        "filter_ocel_cc_otype" => filter_ocel_cc_otype(log, args[0].as_str().unwrap(), select),
        "filter_ocel_cc_activity" => filter_ocel_cc_activity(log, args[0].as_str().unwrap()),
        name => panic!("unknown filter {name}"),
    }
}

#[test]
fn all_fifteen_filters_match_pm4py_tables() {
    let ids = cases("ocel_filters");
    assert_eq!(ids.len(), 18);
    let mut functions = std::collections::BTreeSet::new();
    for id in ids {
        let g = golden("ocel_filters", &id);
        let e = &g.expected;
        let mut input = build(&e["input"]);
        input.globals.insert("custom", "retained");
        input.naive_times = true;
        let snapshot = input.clone();
        if !e["components"].is_null() {
            let expected: Vec<std::collections::BTreeSet<Arc<str>>> = e["components"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| strings(c).into_iter().map(Into::into).collect())
                .collect();
            assert_eq!(
                object_connected_components(&input),
                expected,
                "{id}: components"
            );
        }
        for (index, row) in e["results"].as_array().unwrap().iter().enumerate() {
            functions.insert(row["function"].as_str().unwrap().to_string());
            let mut selected = serde_json::Map::new();
            for (table, indices) in row["rows"].as_object().unwrap() {
                selected.insert(
                    table.clone(),
                    Value::Array(
                        indices
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|i| e["input"][table][i.as_u64().unwrap() as usize].clone())
                            .collect(),
                    ),
                );
            }
            let mut expected = build(&Value::Object(selected));
            expected.globals = input.globals.clone();
            expected.naive_times = input.naive_times;
            let actual = apply(&input, row);
            assert_eq!(
                actual, expected,
                "{id} scenario {index}: {}",
                row["function"]
            );
            assert_eq!(input, snapshot, "input mutation");
            if row["function"] == "filter_ocel_cc_object" {
                let components = object_connected_components(&input);
                assert_eq!(
                    filter_ocel_cc_object(
                        &input,
                        row["args"][0].as_str().unwrap(),
                        Some(&components)
                    ),
                    actual
                );
            }
        }
    }
    assert_eq!(functions.len(), 15);
}

#[test]
fn empty_input_missing_attributes_and_unknown_objects_are_defined() {
    let empty = Ocel::default();
    assert_eq!(filter_ocel_events(&empty, &[], Selection::Keep), empty);
    assert_eq!(filter_ocel_cc_object(&empty, "absent", None), empty);
    let g = golden("ocel_filters", "objects");
    let log = build(&g.expected["input"]);
    assert!(
        filter_ocel_event_attribute(&log, EventField::Attribute("absent"), &[], Selection::Keep)
            .events
            .is_empty()
    );
    assert_eq!(
        filter_ocel_event_attribute(
            &log,
            EventField::Attribute("absent"),
            &[],
            Selection::Remove
        )
        .events,
        log.events
    );
    assert!(
        filter_ocel_objects(
            &log,
            &["absent"],
            ObjectFilterOptions {
                level: 20,
                ..Default::default()
            }
        )
        .objects
        .is_empty()
    );
}

#[test]
fn allowed_activity_pairs_do_not_collide_on_pm4py_separator() {
    let g = golden("ocel_filters", "object-types-allowed-activities");
    let mut log = build(&g.expected["input"]);
    for e in &mut log.events {
        e.activity = "a@#@#b".into();
    }
    for o in &mut log.objects {
        o.object_type = "c".into();
    }
    let allowed = [(
        "b@#@#c".to_string(),
        ["a".to_string()].into_iter().collect(),
    )]
    .into_iter()
    .collect();
    assert!(
        filter_ocel_object_types_allowed_activities(&log, &allowed)
            .relations
            .is_empty()
    );
}

#[test]
fn lifecycle_endpoints_skip_relations_to_unlisted_objects() {
    let g = golden("ocel_filters", "start-events-per-object-type");
    let mut log = build(&g.expected["input"]);
    assert!(!log.relations.is_empty());
    let object_index = log.object_index()[log.relations[0].object.as_ref()];
    let object_type = log.objects[object_index].object_type.clone();
    assert!(
        !filter_ocel_start_events_per_object_type(&log, &object_type)
            .events
            .is_empty()
    );
    log.objects.clear();
    assert!(
        filter_ocel_start_events_per_object_type(&log, &object_type)
            .events
            .is_empty()
    );
    assert!(
        filter_ocel_end_events_per_object_type(&log, &object_type)
            .events
            .is_empty()
    );
}
