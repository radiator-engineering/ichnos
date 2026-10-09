use chrono::{DateTime, FixedOffset};
use ichnos_core::{Event, EventLog, Trace};
use ichnos_ml::*;
use ichnos_ocel::{EventObject, Ocel, OcelEvent, OcelObject};

fn at(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

fn event(activity: &str, time: &str) -> Event {
    let mut e = Event::new();
    e.insert("concept:name", activity);
    e.insert("time:timestamp", at(time));
    e
}

fn log() -> EventLog {
    let mut a = Trace::with_case_id("a");
    a.events.push(event("x", "2024-01-01T00:00:00Z"));
    a.events.push(event("y", "2024-01-01T00:00:10Z"));
    let mut b = Trace::with_case_id("b");
    b.events.push(event("y", "2024-01-02T00:00:00Z"));
    EventLog::from_traces(vec![a, b])
}

#[test]
fn target_padding_fills_with_zeros() {
    let options = TargetOptions {
        padding: Some(None),
        ..TargetOptions::default()
    };
    let (t, classes) = extract_target_vector(&log(), TargetVariant::NextTime, &options).unwrap();
    assert_eq!(classes, ["@@next_time"]);
    assert_eq!(
        t,
        TargetVector::Times(vec![vec![10.0, 0.0], vec![0.0, 0.0]])
    );
    let options = TargetOptions {
        padding: Some(Some(3)),
        ..TargetOptions::default()
    };
    let (t, classes) =
        extract_target_vector(&log(), TargetVariant::NextActivity, &options).unwrap();
    assert_eq!(classes, ["x", "y"]);
    let TargetVector::Activities(t) = t else {
        panic!()
    };
    assert_eq!(t[0], [vec![0.0, 1.0], vec![0.0, 0.0], vec![0.0, 0.0]]);
    assert_eq!(t[1].len(), 3);
}

#[test]
fn empty_logs_give_empty_results() {
    let empty = EventLog::default();
    let (t, _) = extract_target_vector(
        &empty,
        TargetVariant::RemainingTime,
        &TargetOptions::default(),
    )
    .unwrap();
    assert_eq!(t, TargetVector::Times(Vec::new()));
    let rows = extract_temporal_features_dataframe(&empty, &TemporalOptions::default()).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn temporal_rejects_a_zero_width() {
    let options = TemporalOptions {
        grouper_freq: GrouperFreq::Hours(0),
        ..TemporalOptions::default()
    };
    assert!(matches!(
        extract_temporal_features_dataframe(&log(), &options),
        Err(Error::InvalidOption(_))
    ));
}

#[test]
fn trace_features_use_the_default_for_absent_activities() {
    let options = TraceFeatureOptions {
        extras: TraceExtras {
            first_last_activity_index: true,
            times_from_first_occurrence: true,
            ..TraceExtras::default()
        },
        ..TraceFeatureOptions::default()
    };
    let table = trace_features(&log(), &options).unwrap();
    assert_eq!(table.case_ids, ["a", "b"]);
    assert_eq!(table.column("startToFirstOcc@@y").unwrap(), [10.0, 0.0]);
    assert_eq!(table.column("firstIndexAct@@x").unwrap(), [0.0, -1.0]);
    let options = TraceFeatureOptions {
        default_not_present: Some(-5.0),
        ..options
    };
    let table = trace_features(&log(), &options).unwrap();
    assert_eq!(table.column("startToFirstOcc@@x").unwrap(), [0.0, -5.0]);
    assert_eq!(table.column("firstIndexAct@@x").unwrap(), [0.0, -5.0]);
}

#[test]
fn trace_features_need_numeric_attributes_in_every_trace() {
    let options = TraceFeatureOptions {
        num_ev_attr: vec!["cost".to_owned()],
        ..TraceFeatureOptions::default()
    };
    assert!(matches!(
        trace_features(&log(), &options),
        Err(Error::MissingEventAttribute { .. })
    ));
}

#[test]
fn ocel_features_reject_events_without_objects() {
    let mut ocel = Ocel::new();
    for (id, act) in [("e1", "create"), ("e2", "ship")] {
        ocel.events.push(OcelEvent {
            id: id.into(),
            activity: act.into(),
            timestamp: at("2024-01-01T00:00:00Z"),
            attributes: Default::default(),
        });
    }
    ocel.objects.push(OcelObject {
        id: "o1".into(),
        object_type: "order".into(),
        attributes: Default::default(),
    });
    ocel.relations.push(EventObject {
        event: "e1".into(),
        object: "o1".into(),
        qualifier: None,
    });
    assert!(matches!(
        extract_ocel_features(&ocel, "order", &OcelFeatureOptions::default()),
        Err(Error::EventWithoutObjects(e)) if e == "e2"
    ));
}
