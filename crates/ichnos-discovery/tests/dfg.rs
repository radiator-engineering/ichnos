use ichnos_core::{AttributeValue, Event, EventKeys, EventLog, Position, Trace, chrono::DateTime};
use ichnos_discovery::dfg::BusinessHours;
use ichnos_discovery::*;

fn event(activity: &str, time: &str) -> Event {
    let mut e = Event::new();
    e.attributes.insert("concept:name", activity);
    e.attributes.insert(
        "time:timestamp",
        AttributeValue::Date(DateTime::parse_from_rfc3339(time).unwrap()),
    );
    e
}
#[test]
fn boundaries_and_extreme_window_need_no_timestamps() {
    let keys = EventKeys::default();
    let mut log = EventLog::from_trace_strings(["a,b,a", "one"], ",", &keys);
    for e in log.traces.iter_mut().flat_map(|t| &mut t.events) {
        e.attributes.remove(&keys.timestamp);
    }
    log.traces.push(Trace::new());
    let graph = dfg(
        &log,
        &keys,
        &DfgOptions {
            window: usize::MAX,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(graph.graph.is_empty());
    assert_eq!(graph.start_activities.values().sum::<u64>(), 2);
    assert_eq!(graph.end_activities.values().sum::<u64>(), 2);
    assert_eq!(derive_minimum_self_distance(&log, &keys).unwrap()["a"], 1);
    log.traces[0].events[1].attributes.remove(&keys.activity);
    assert!(matches!(
        dfg(&log, &keys, &Default::default()),
        Err(Error::Core(ichnos_core::Error::MissingAttribute {
            position: Position::Event { trace: 0, event: 1 },
            ..
        }))
    ));
}
#[test]
fn elapsed_gaps_preserve_nanoseconds_and_offsets() {
    let log = EventLog {
        traces: vec![Trace {
            attributes: Default::default(),
            events: vec![
                event("a", "2024-01-01T02:00:00.000000001+02:00"),
                event("b", "2024-01-01T00:00:00.000000002+00:00"),
                event("c", "2024-01-01T00:00:00.000000001+00:00"),
            ],
        }],
        ..Default::default()
    };
    let graph = performance_dfg(&log, &Default::default(), &Default::default()).unwrap();
    assert_eq!(graph.graph[&("a".into(), "b".into())].mean, 1e-9);
    assert_eq!(graph.graph[&("b".into(), "c".into())].mean, 0.0);
}
#[test]
fn timestamp_errors_keep_original_positions() {
    let mut log = EventLog::from_trace_strings(["a,b"], ",", &Default::default());
    log.traces[0].events[0].attributes.remove("time:timestamp");
    assert!(matches!(
        performance_dfg(&log, &Default::default(), &Default::default()),
        Err(Error::Core(ichnos_core::Error::MissingAttribute {
            position: Position::Event { trace: 0, event: 0 },
            ..
        }))
    ));
    log.traces[0].events[0] = event("a", "2024-01-01T00:00:00Z");
    log.traces[0].events[1]
        .attributes
        .insert("time:timestamp", "invalid");
    assert!(matches!(
        performance_dfg(&log, &Default::default(), &Default::default()),
        Err(Error::Core(ichnos_core::Error::AttributeType {
            position: Position::Event { trace: 0, event: 1 },
            ..
        }))
    ));
    assert!(matches!(
        eventually_follows_graph(&log, &Default::default(), &Default::default()),
        Err(Error::Core(ichnos_core::Error::AttributeType {
            position: Position::Event { trace: 0, event: 1 },
            ..
        }))
    ));
}
#[test]
fn singleton_performance_and_invalid_schedule() {
    let mut log = EventLog::from_trace_strings(["one"], ",", &Default::default());
    log.traces[0].events[0].attributes.remove("time:timestamp");
    assert!(
        performance_dfg(&log, &Default::default(), &Default::default())
            .unwrap()
            .graph
            .is_empty()
    );
    let options = PerformanceDfgOptions {
        business_hours: Some(BusinessHours {
            slots: vec![(2, 1)],
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(matches!(
        performance_dfg(&EventLog::default(), &Default::default(), &options),
        Err(Error::InvalidOption(_))
    ));
}
