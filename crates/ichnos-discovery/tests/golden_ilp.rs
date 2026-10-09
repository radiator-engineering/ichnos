use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{Error, IlpActivity, IlpOptions, petri_net_ilp};
use ichnos_golden::golden;
use ichnos_model::{AcceptingPetriNet, Footprints, Label};
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};
fn language(net: &AcceptingPetriNet, depth: usize) -> Value {
    let initial = (net.initial_marking.clone(), Vec::<String>::new());
    let mut seen = BTreeSet::from([initial.clone()]);
    let mut todo = VecDeque::from([initial]);
    let mut prefixes = BTreeSet::from([Vec::<String>::new()]);
    let mut accepted = BTreeSet::new();
    while let Some((m, word)) = todo.pop_front() {
        if m == net.final_marking {
            accepted.insert(word.clone());
        }
        for t in net.net.enabled_transitions(&m) {
            let mut next_word = word.clone();
            if let Some(label) = &net.net.transition(t).label {
                next_word.push(label.to_string());
            }
            if next_word.len() > depth {
                continue;
            }
            let next = (net.net.fire(t, &m).unwrap(), next_word.clone());
            prefixes.insert(next_word);
            if seen.insert(next.clone()) {
                assert!(seen.len() <= 100000, "language state cap");
                todo.push_back(next);
            }
        }
    }
    json!({"depth":depth,"prefixes":prefixes,"accepted":accepted})
}
fn check_model(net: &AcceptingPetriNet, expected: &Value, context: &str) {
    for marking in [&net.initial_marking, &net.final_marking] {
        assert!(
            marking.iter().all(|(p, _)| net.net.contains_place(p)),
            "{context}: orphan marking"
        );
    }
    let actual_language = language(net, 3);
    for field in ["prefixes", "accepted"] {
        let actual: BTreeSet<Vec<String>> =
            serde_json::from_value(actual_language[field].clone()).unwrap();
        let expected: BTreeSet<Vec<String>> =
            serde_json::from_value(expected["language"][field].clone()).unwrap();
        assert!(
            actual == expected,
            "{context}/{field}: missing {:?}, extra {:?}",
            expected.difference(&actual).take(12).collect::<Vec<_>>(),
            actual.difference(&expected).take(12).collect::<Vec<_>>()
        );
    }
    let actual = net.net.footprints(
        &net.initial_marking,
        ichnos_model::petri::ReachabilityOptions {
            max_markings: 10000,
        },
    );
    if expected["footprints"]["status"] == "complete" {
        let footprints: Footprints =
            serde_json::from_value(expected["footprints"]["value"].clone()).unwrap();
        assert_eq!(actual.unwrap(), footprints, "{context}: footprints");
    } else {
        assert!(actual.is_err(), "{context}: expected state space cap");
    }
}

fn check(name: &str) {
    let g = golden("discovery", &format!("ilp-miner-{name}"));
    let keys = EventKeys::default().with_activity(
        g.meta().params["activity_key"]
            .as_str()
            .unwrap_or("concept:name"),
    );
    let log = if let Some(traces) = g.meta().params["traces"].as_array() {
        let mut log = EventLog::default();
        for row in traces {
            let mut trace = Trace::new();
            for label in row.as_array().unwrap() {
                let mut event = Event::new();
                event
                    .attributes
                    .insert(keys.activity.as_str(), label.as_str().unwrap());
                trace.events.push(event);
            }
            log.traces.push(trace);
        }
        log
    } else if name.ends_with("xes") {
        ichnos_io::read_xes(g.fixture("log"), &Default::default()).unwrap()
    } else {
        ichnos_io::read_csv(g.fixture("log"), &Default::default()).unwrap()
    };
    let before = log.clone();
    let activity = |v: &Value| match v.as_str().unwrap() {
        "▶" => IlpActivity::Start,
        "■" => IlpActivity::End,
        label => IlpActivity::Activity(Label::from(label)),
    };
    for row in g.expected["runs"].as_array().unwrap() {
        let options = IlpOptions {
            alpha: row["alpha"].as_f64().unwrap(),
            causal_relation: g.meta().params["causal"].as_array().map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| (activity(&pair[0]), activity(&pair[1])))
                    .collect()
            }),
        };
        let net = petri_net_ilp(&log, &keys, &options).unwrap();
        if row["error"].is_string() {
            assert_eq!(
                language(&net, 3),
                json!({"depth":3,"prefixes":[[]],"accepted":[[]]})
            );
        } else {
            check_model(&net, &row["model"], name);
        }
    }
    assert_eq!(log, before);
}
macro_rules! cases { ($($test:ident=>$name:literal),*)=>{$(#[test] fn $test(){check($name);})*}; }
cases!(running_example=>"running-example-xes",receipt=>"receipt-xes",roadtraffic=>"roadtraffic100traces-xes",even=>"interleavings-receipt_even-csv",odd=>"interleavings-receipt_odd-csv",empty=>"empty",empty_traces=>"empty-traces",sequence=>"sequence",parallel=>"parallel",loops=>"loops",weighted=>"weighted",custom_key=>"custom-key",causal=>"causal");
#[test]
fn typed_options_and_boundary_collisions() {
    for alpha in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(matches!(
            petri_net_ilp(
                &EventLog::default(),
                &Default::default(),
                &IlpOptions {
                    alpha,
                    ..Default::default()
                }
            ),
            Err(Error::InvalidOption(_))
        ));
    }
    let mut trace = Trace::new();
    for label in ["▶", "■"] {
        let mut event = Event::new();
        event.attributes.insert("concept:name", label);
        trace.events.push(event);
    }
    let log = EventLog {
        traces: vec![trace],
        ..Default::default()
    };
    let net = petri_net_ilp(&log, &Default::default(), &Default::default()).unwrap();
    assert!(
        language(&net, 3)["accepted"]
            .as_array()
            .unwrap()
            .contains(&json!(["▶", "■"]))
    );
    assert!(
        net.net
            .transitions()
            .filter_map(|(_, t)| t.label.as_ref())
            .any(|label| label.as_str() == "▶")
    );
    assert!(matches!(
        petri_net_ilp(
            &log,
            &Default::default(),
            &IlpOptions {
                causal_relation: Some(BTreeSet::from([(
                    IlpActivity::Start,
                    IlpActivity::Activity(Label::from("absent"))
                )])),
                ..Default::default()
            }
        ),
        Err(Error::InvalidOption(_))
    ));
    let mut missing = log.clone();
    missing.traces[0].events[0]
        .attributes
        .remove("concept:name");
    assert!(matches!(
        petri_net_ilp(&missing, &Default::default(), &Default::default()),
        Err(Error::Core(_))
    ));
}
