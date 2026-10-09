//! pm4py Declare prefix diagnostics, including all eighteen automata.
use ichnos_core::{AttributeValue, Event, EventKeys, chrono::DateTime};
use ichnos_discovery::declare::DeclareCounts;
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_stream::{
    DeclareActivities, DeclareEventTime, DeclareMissingPolicy, DeclareModel, DeclareTemplate,
    Error, LiveEventStream, StreamingDeclareCase, StreamingDeclareConformance,
    StreamingDeclareOptions, XesEventReader,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

fn compare(actual: &Value, expected: &Value) {
    assert_json_eq(
        actual,
        expected,
        &JsonCompare {
            unordered_arrays: false,
            ..Default::default()
        },
    );
}

fn model(spec: &Value) -> DeclareModel {
    let mut model = DeclareModel::default();
    for rule in spec.as_array().unwrap() {
        let template = DeclareTemplate::ALL
            .into_iter()
            .find(|t| t.as_str() == rule[0].as_str().unwrap())
            .unwrap();
        let args = rule[1].as_array().unwrap();
        let activities = if args.len() == 1 {
            DeclareActivities::Unary(args[0].as_str().unwrap().into())
        } else {
            DeclareActivities::Binary(
                args[0].as_str().unwrap().into(),
                args[1].as_str().unwrap().into(),
            )
        };
        model.rules.entry(template).or_default().insert(
            activities,
            DeclareCounts {
                support: 7,
                confidence: 5,
            },
        );
    }
    model
}

fn attribute(v: &AttributeValue) -> Value {
    match v.plain() {
        AttributeValue::String(v) | AttributeValue::Id(v) => json!(v.as_ref()),
        AttributeValue::Int(v) => json!(v),
        AttributeValue::Float(v) => json!(v),
        AttributeValue::Bool(v) => json!(v),
        AttributeValue::Date(v) => {
            let timestamp = v
                .with_timezone(&ichnos_core::chrono::Utc)
                .to_rfc3339_opts(ichnos_core::chrono::SecondsFormat::Micros, false);
            json!({ "date": timestamp })
        }
        _ => panic!("unsupported fixture timestamp"),
    }
}

fn case_state(case: &StreamingDeclareCase) -> Value {
    let mut constraints: Vec<_> = case
        .constraints_state
        .iter()
        .map(|(c, s)| {
            let args: Vec<_> = match &c.activities {
                DeclareActivities::Unary(a) => vec![a.to_string()],
                DeclareActivities::Binary(a, b) => vec![a.to_string(), b.to_string()],
            };
            (c.template.as_str(), args, s.as_str())
        })
        .collect();
    constraints.sort();
    json!({
        "events": case.events,
        "deviations": case.deviations,
        "constraints_state": constraints,
    })
}

fn summary(rows: Vec<Value>) -> Value {
    let indices: BTreeSet<_> = [0, 1, rows.len().saturating_sub(1)]
        .into_iter()
        .filter(|&i| i < rows.len())
        .collect();
    let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&rows).unwrap()));
    let samples: Vec<_> = indices.into_iter().map(|i| json!([i, rows[i]])).collect();
    json!({
        "count": rows.len(),
        "sha256": digest,
        "samples": samples,
    })
}

fn state(algo: &StreamingDeclareConformance, compact: bool) -> Value {
    let snapshot = algo.get();
    let cases = Value::Object(
        snapshot
            .cases
            .iter()
            .map(|(c, data)| {
                assert_eq!(algo.get_status(c), Some(data.clone()));
                (c.clone(), case_state(data))
            })
            .collect(),
    );
    let history: Vec<_> = snapshot
        .deviations_per_time
        .iter()
        .map(|d| {
            let time = match &d.time {
                DeclareEventTime::EventIndex(n) => json!(n),
                DeclareEventTime::Attribute(v) => attribute(v),
            };
            json!([time, d.deviations])
        })
        .collect();
    let cases = if compact {
        summary(
            cases
                .as_object()
                .unwrap()
                .iter()
                .map(|(c, v)| json!([c, v]))
                .collect(),
        )
    } else {
        cases
    };
    let history = if compact {
        summary(history)
    } else {
        json!(history)
    };
    json!({
        "total_events_processed": snapshot.total_events_processed,
        "total_deviations": snapshot.total_deviations,
        "deviations_per_time": history,
        "cases": cases,
    })
}

fn event(row: &Value) -> Event {
    let mut event = Event::new();
    for (key, value) in row.as_object().unwrap() {
        if key == "time:timestamp" && value.is_string() {
            event.insert(
                key.as_str(),
                DateTime::parse_from_rfc3339(value.as_str().unwrap()).unwrap(),
            );
        } else if let Some(s) = value.as_str() {
            event.insert(key.as_str(), s);
        } else {
            event.insert(key.as_str(), value.as_i64().unwrap());
        }
    }
    event
}

fn check(name: &str) {
    let g = golden("stream", &format!("declare-{name}"));
    let model = model(&g.expected["model"]);
    let mut algo = StreamingDeclareConformance::new(&model, Default::default()).unwrap();
    let p = &g.meta().params;
    let compact = p["events"].is_null();
    let events: Vec<Event> = if compact {
        XesEventReader::open(g.fixture("log"), Default::default())
            .unwrap()
            .collect::<ichnos_stream::Result<_>>()
            .unwrap()
    } else {
        p["events"].as_array().unwrap().iter().map(event).collect()
    };
    let before = events.clone();
    let points: BTreeSet<_> = g.expected["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["at"].as_u64().unwrap() as usize)
        .collect();
    let mut snapshots = vec![json!({
        "at": 0,
        "state": state(&algo, compact),
    })];
    for (i, event) in events.iter().enumerate() {
        algo.push(event).unwrap();
        if points.contains(&(i + 1)) {
            snapshots.push(json!({
                "at": i + 1,
                "state": state(&algo, compact),
            }));
        }
    }
    compare(&json!(snapshots), &g.expected["snapshots"]);
    let live_algo = Rc::new(RefCell::new(
        StreamingDeclareConformance::new(&model, Default::default()).unwrap(),
    ));
    let mut stream = LiveEventStream::new();
    stream.register(live_algo.clone());
    if let Some(event) = events.first() {
        stream.append(event.clone()).unwrap();
    }
    stream.start().unwrap();
    for event in events.iter().skip(1) {
        stream.append(event.clone()).unwrap();
    }
    stream.stop().unwrap();
    compare(&state(&live_algo.borrow(), compact), &g.expected["live"]);
    assert_eq!(algo.get(), live_algo.borrow().get());
    assert_eq!(events, before);
}

macro_rules! cases {
    ($($id:ident => $name:literal),* $(,)?) => {
        $(
            #[test]
            fn $id() {
                check($name);
            }
        )*
    };
}

cases! {
    running => "running-example",
    receipt => "receipt",
    traffic => "roadtraffic100traces",
    all_templates => "all-templates",
    pending => "pending",
    interleaved => "interleaved",
    missing => "missing",
    timestamps => "timestamps",
    empty => "empty",
    self_pairs => "self-pairs",
    empty_model => "empty-model",
    special_labels => "special-labels",
}

#[test]
fn rejects_wrong_arity_and_exposes_explicit_missing_policies() {
    let mut m = DeclareModel::default();
    m.rules
        .entry(DeclareTemplate::Response)
        .or_default()
        .insert(DeclareActivities::Unary("A".into()), Default::default());
    assert!(StreamingDeclareConformance::new(&m, Default::default()).is_err());
    m.rules.clear();
    m.rules.entry(DeclareTemplate::Absence).or_default().insert(
        DeclareActivities::Binary("A".into(), "B".into()),
        Default::default(),
    );
    assert!(StreamingDeclareConformance::new(&m, Default::default()).is_err());
    let m = model(&json!([["absence", ["A"]]]));
    let options = StreamingDeclareOptions {
        missing: DeclareMissingPolicy::Reject,
        ..Default::default()
    };
    let mut a = StreamingDeclareConformance::new(&m, options).unwrap();
    let event: Event = [("case:concept:name", "c"), ("concept:name", "A")]
        .into_iter()
        .collect();
    a.push(&event).unwrap();
    let before = a.get();
    assert!(matches!(
        a.push(&Event::new()),
        Err(Error::MissingField { event: 1, .. })
    ));
    assert_eq!(a.get(), before);
    let mut a = StreamingDeclareConformance::new(
        &m,
        StreamingDeclareOptions {
            missing: DeclareMissingPolicy::Ignore,
            ..Default::default()
        },
    )
    .unwrap();
    a.push(&Event::new()).unwrap();
    assert_eq!(a.skipped_events(), 1);
    assert_eq!(a.get().total_events_processed, 0);
    assert!(a.get().cases.is_empty());
}

#[test]
fn custom_keys_remove_case_and_clear_history_preserve_totals() {
    let m = model(&json!([["absence", ["A"]]]));
    let options = StreamingDeclareOptions {
        keys: EventKeys::default()
            .with_case_id("case")
            .with_activity("task")
            .with_timestamp("when"),
        ..Default::default()
    };
    let mut a = StreamingDeclareConformance::new(&m, options).unwrap();
    let event: Event = [("case", "c"), ("task", "A"), ("when", "typed-time")]
        .into_iter()
        .collect();
    a.push(&event).unwrap();
    assert_eq!(a.get().total_deviations, 1);
    assert!(matches!(
        a.get().deviations_per_time[0].time,
        DeclareEventTime::Attribute(_)
    ));
    assert!(a.remove_case("c"));
    assert!(!a.remove_case("c"));
    assert!(a.get_status("c").is_none());
    assert_eq!(a.get().total_deviations, 1);
    a.push(&event).unwrap();
    assert_eq!(a.get().total_deviations, 2);
    assert_eq!(a.get_status("c").unwrap().events, 1);
    a.clear_history();
    assert!(a.get().deviations_per_time.is_empty());
    assert_eq!(a.get().total_events_processed, 2);
    assert_eq!(a.get_status("c").unwrap().deviations, 1);
    a.push(&Event::new()).unwrap();
    assert_eq!(
        a.get().deviations_per_time[0].time,
        DeclareEventTime::EventIndex(3)
    );
}
