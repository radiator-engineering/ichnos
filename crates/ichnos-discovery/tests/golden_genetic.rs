use ichnos_core::{AttributeValue, Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{
    Error, GeneticMatrix, GeneticOptions, discover_genetic, genetic_matrix_fitness,
};
use ichnos_golden::golden;
use ichnos_model::{AcceptingPetriNet, Footprints, Label};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
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

fn input(g: &ichnos_golden::Golden, name: &str) -> (EventLog, EventKeys) {
    let keys = EventKeys::default().with_activity(
        g.meta().params["activity_key"]
            .as_str()
            .unwrap_or("concept:name"),
    );
    let log = if let Some(traces) = g.meta().params["traces"].as_array() {
        let mut log = EventLog::default();
        for (ti, row) in traces.iter().enumerate() {
            let mut trace = Trace::new();
            trace.attributes.insert("concept:name", ti.to_string());
            for (i, label) in row.as_array().unwrap().iter().enumerate() {
                let mut event = Event::new();
                event
                    .attributes
                    .insert(keys.activity.as_str(), label.as_str().unwrap());
                event.attributes.insert(
                    keys.timestamp.as_str(),
                    AttributeValue::Date(format!("2020-01-01T00:00:{i:02}+00:00").parse().unwrap()),
                );
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
    (log, keys)
}
fn matrix(value: &Value) -> GeneticMatrix {
    let map = |v: &Value| {
        v.as_object()
            .unwrap()
            .iter()
            .map(|(label, bindings)| {
                (
                    Label::from(label.as_str()),
                    bindings
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|set| {
                            set.as_array()
                                .unwrap()
                                .iter()
                                .map(|label| Label::from(label.as_str().unwrap()))
                                .collect()
                        })
                        .collect(),
                )
            })
            .collect()
    };
    GeneticMatrix {
        activities: value["activities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|label| Label::from(label.as_str().unwrap()))
            .collect(),
        inputs: map(&value["inputs"]),
        outputs: map(&value["outputs"]),
    }
}
fn check_matrix(name: &str) {
    let g = golden("discovery", &format!("genetic-matrix-{name}"));
    let (log, keys) = input(&g, name);
    let before = log.clone();
    let m = matrix(&g.expected["matrix"]);
    check_model(&m.to_petri_net().unwrap(), &g.expected["model"], name);
    let fitness = genetic_matrix_fitness(&log, &keys, &m).unwrap();
    assert!(
        (fitness - g.expected["fitness"].as_f64().unwrap()).abs() < 1e-9,
        "{name}: fitness {fitness}, expected {}",
        g.expected["fitness"]
    );
    assert_eq!(log, before);
}
fn check_public(name: &str) {
    let g = golden("discovery", &format!("genetic-public-{name}"));
    let (log, keys) = input(&g, name);
    let options = GeneticOptions {
        population_size: 4,
        generations: 1,
        ..Default::default()
    };
    let result = discover_genetic(&log, &keys, &options).unwrap();
    check_model(&result.model, &g.expected["model"], name);
    assert!((result.fitness - g.expected["fitness"].as_f64().unwrap()).abs() < 1e-9);
    assert_eq!(result.history, vec![result.fitness]);
}
macro_rules! matrices { ($($test:ident=>$name:literal),*)=>{$(#[test] fn $test(){check_matrix($name);})*}; }
matrices!(running_example=>"running-example-xes",receipt=>"receipt-xes",roadtraffic=>"roadtraffic100traces-xes",even=>"interleavings-receipt_even-csv",odd=>"interleavings-receipt_odd-csv",sequence=>"sequence",parallel=>"parallel",loops=>"loop",silent=>"silent",custom_key=>"custom-key",grouped=>"grouped");
#[test]
fn public_sequence() {
    check_public("sequence");
}
#[test]
fn public_single() {
    check_public("single");
}
#[test]
fn public_loop() {
    check_public("loop");
}
#[test]
fn seeded_search_and_validation() {
    let g = golden("discovery", "genetic-matrix-parallel");
    let (log, keys) = input(&g, "parallel");
    let before = log.clone();
    let options = GeneticOptions {
        population_size: 8,
        generations: 8,
        elitism_min_sample: 3,
        mutation_rate: 1.0,
        seed: 42,
        ..Default::default()
    };
    let a = discover_genetic(&log, &keys, &options).unwrap();
    let b = discover_genetic(&log, &keys, &options).unwrap();
    assert_eq!(a.matrix, b.matrix);
    assert_eq!(a.history, b.history);
    assert!(a.history.iter().all(|v| (0.0..=1.0).contains(v)));
    assert!(a.history.len() <= 8);
    assert_eq!(a.fitness, *a.history.last().unwrap());
    assert_eq!(log, before);
    let mut custom = log.clone();
    for trace in &mut custom.traces {
        for event in &mut trace.events {
            let label = event.attributes.remove(&keys.activity).unwrap();
            event.attributes.insert("work", label);
        }
    }
    let custom_keys = keys.clone().with_activity("work");
    let c = discover_genetic(&custom, &custom_keys, &options).unwrap();
    assert_eq!(a.matrix, c.matrix);
    assert_eq!(a.history, c.history);
    for rate in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(matches!(
            discover_genetic(
                &log,
                &keys,
                &GeneticOptions {
                    mutation_rate: rate,
                    ..options.clone()
                }
            ),
            Err(Error::InvalidOption(_))
        ));
    }
    assert!(matches!(
        discover_genetic(
            &log,
            &keys,
            &GeneticOptions {
                population_size: 1,
                ..options.clone()
            }
        ),
        Err(Error::InvalidOption(_))
    ));
    let empty = discover_genetic(&EventLog::default(), &keys, &options).unwrap();
    assert_eq!(language(&empty.model, 3)["accepted"], json!([[]]));
    let mut bad = log.clone();
    bad.traces[0].events[0].attributes.remove(&keys.timestamp);
    assert!(matches!(
        discover_genetic(&bad, &keys, &options),
        Err(Error::Core(_))
    ));
    let invalid = GeneticMatrix {
        activities: vec![Label::from("a")],
        inputs: BTreeMap::new(),
        outputs: BTreeMap::from([(Label::from("a"), vec![BTreeSet::from([Label::from("b")])])]),
    };
    assert!(matches!(
        invalid.to_petri_net(),
        Err(Error::InvalidOption(_))
    ));
}

#[test]
fn stagnation_and_timestamp_sorting() {
    let g = golden("discovery", "genetic-public-loop");
    let (log, keys) = input(&g, "loop");
    let options = GeneticOptions {
        population_size: 4,
        generations: 8,
        mutation_rate: 1.0,
        ..Default::default()
    };
    let result = discover_genetic(&log, &keys, &options).unwrap();
    assert_eq!(
        result.history,
        vec![g.expected["fitness"].as_f64().unwrap(); 2]
    );
    let g = golden("discovery", "genetic-public-sequence");
    let (mut log, keys) = input(&g, "sequence");
    for trace in &mut log.traces {
        trace.events.swap(0, 1);
    }
    let before = log.clone();
    let result = discover_genetic(&log, &keys, &options).unwrap();
    check_model(&result.model, &g.expected["model"], "timestamp-sort");
    assert_eq!(result.history, vec![1.0]);
    assert_eq!(log, before);
    let mut missing = log.clone();
    missing.traces[0].events[1]
        .attributes
        .remove(&keys.activity);
    assert!(matches!(
        discover_genetic(&missing, &keys, &options),
        Err(Error::Core(_))
    ));
}
