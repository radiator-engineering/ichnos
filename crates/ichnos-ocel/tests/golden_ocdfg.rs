//! pm4py `discover_ocdfg` goldens (area `ocel_discovery`).
use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, NaiveDate};
use ichnos_golden::{JsonCompare, assert_json_eq, cases, golden};
use ichnos_ocel::{
    EventObject, Ocdfg, OcdfgActivities, OcdfgActivity, OcdfgDurations, OcdfgError, OcdfgOptions,
    Ocel, OcelEvent, OcelObject, discover_ocdfg,
};
use ichnos_stats::time::BusinessHours;
use serde_json::{Value, json};

fn text(v: &Value) -> Arc<str> {
    v.as_str().expect("string").into()
}

fn rows<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v[key].as_array().expect(key)
}

/// The events, objects and relations of a golden's tables. The OC-DFG
/// reads nothing else.
fn build(v: &Value) -> Ocel {
    Ocel {
        events: rows(v, "events")
            .iter()
            .map(|e| OcelEvent {
                id: text(&e["id"]),
                activity: text(&e["activity"]),
                timestamp: DateTime::parse_from_rfc3339(e["timestamp"].as_str().unwrap()).unwrap(),
                attributes: Default::default(),
            })
            .collect(),
        objects: rows(v, "objects")
            .iter()
            .map(|o| OcelObject {
                id: text(&o["id"]),
                object_type: text(&o["type"]),
                attributes: Default::default(),
            })
            .collect(),
        relations: rows(v, "relations")
            .iter()
            .map(|r| EventObject {
                event: text(&r["event"]),
                object: text(&r["object"]),
                qualifier: r["qualifier"].as_str().map(Into::into),
            })
            .collect(),
        ..Ocel::default()
    }
}

fn activity(a: &OcdfgActivity) -> Value {
    json!({
        "events": a.events.iter().map(|x| &**x).collect::<Vec<_>>(),
        "unique_objects": a.unique_objects.iter().map(|x| &**x).collect::<Vec<_>>(),
        "total_objects": a.total_objects.iter().map(|(e, o)| [&**e, &**o]).collect::<Vec<_>>(),
    })
}

fn activities(part: &OcdfgActivities) -> Value {
    part.iter()
        .map(|(a, occ)| (a.to_string(), activity(occ)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

fn by_type(part: &BTreeMap<Arc<str>, OcdfgActivities>) -> Value {
    part.iter()
        .map(|(ot, acts)| (ot.to_string(), activities(acts)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The OC-DFG in the golden's layout (see `cases/ocel_discovery.py`).
fn describe(d: &Ocdfg) -> Value {
    let edges: serde_json::Map<_, _> = d
        .edges
        .iter()
        .map(|(ot, edges)| {
            let rows = edges
                .iter()
                .map(|((a, b), e)| {
                    json!([&**a, &**b, {
                        "event_couples": e.event_couples.iter().map(|(x, y)| [&**x, &**y]).collect::<Vec<_>>(),
                        "unique_objects": e.unique_objects.iter().map(|x| &**x).collect::<Vec<_>>(),
                        "total_objects": e.total_objects.iter().map(|(x, y, o)| [&**x, &**y, &**o]).collect::<Vec<_>>(),
                        "event_couples_durations": e.event_couples_durations,
                        "total_objects_durations": e.total_objects_durations,
                    }])
                })
                .collect::<Vec<_>>();
            (ot.to_string(), Value::Array(rows))
        })
        .collect();
    json!({
        "activities": d.activities.iter().map(|x| &**x).collect::<Vec<_>>(),
        "object_types": d.object_types.iter().map(|x| &**x).collect::<Vec<_>>(),
        "activities_indep": activities(&d.activities_indep),
        "activities_ot": by_type(&d.activities_ot),
        "start_activities": by_type(&d.start_activities),
        "end_activities": by_type(&d.end_activities),
        "edges": edges,
    })
}

/// pm4py's work schedule for a case, as ichnos-stats business hours.
fn business_hours(id: &str) -> Option<BusinessHours> {
    match id {
        "ocdfg-synthetic-business-hours" => Some(BusinessHours::default()),
        "ocdfg-synthetic-business-slots" => Some(BusinessHours {
            slots: vec![
                (7 * 3600, 12 * 3600),
                (12 * 3600 + 1, 17 * 3600),
                (10 * 3600, 13 * 3600),
            ],
            ..BusinessHours::default()
        }),
        "ocdfg-synthetic-holiday" => Some(BusinessHours {
            non_working_dates: [NaiveDate::from_ymd_opt(2024, 1, 8).unwrap()].into(),
            ..BusinessHours::default()
        }),
        _ => None,
    }
}

fn options(id: &str) -> OcdfgOptions {
    let durations = if id == "ocdfg-synthetic-no-performance" {
        OcdfgDurations::Skip
    } else if let Some(schedule) = business_hours(id) {
        OcdfgDurations::Custom(Arc::new(
            move |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| {
                schedule.seconds_between(a, b).expect("valid schedule")
            },
        ))
    } else {
        OcdfgDurations::Elapsed
    };
    OcdfgOptions { durations }
}

#[test]
fn discover_ocdfg_goldens() {
    let ids = cases("ocel_discovery");
    assert_eq!(ids.len(), 17, "{ids:?}");
    for id in ids {
        let g = golden("ocel_discovery", &id);
        let log = build(&g.expected["input"]);
        let ocdfg = discover_ocdfg(&log, &options(&id)).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_json_eq(
            &describe(&ocdfg),
            &g.expected["expected"],
            &JsonCompare::default(),
        );
    }
}

#[test]
fn unknown_relation_ends_are_errors() {
    let event = OcelEvent {
        id: "e1".into(),
        activity: "a".into(),
        timestamp: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap(),
        attributes: Default::default(),
    };
    let object = OcelObject {
        id: "o1".into(),
        object_type: "order".into(),
        attributes: Default::default(),
    };
    let relation = |event: &str, object: &str| EventObject {
        event: event.into(),
        object: object.into(),
        qualifier: None,
    };
    let mut log = Ocel {
        events: vec![event],
        objects: vec![object],
        relations: vec![relation("e1", "o1"), relation("e2", "o1")],
        ..Ocel::default()
    };
    let options = OcdfgOptions::default();
    assert_eq!(
        discover_ocdfg(&log, &options),
        Err(OcdfgError::UnknownEvent("e2".into()))
    );
    log.relations[1] = relation("e1", "o2");
    assert_eq!(
        discover_ocdfg(&log, &options),
        Err(OcdfgError::UnknownObject("o2".into()))
    );
}
