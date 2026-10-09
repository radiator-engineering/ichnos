//! Behavioural and matrix parity for alpha, alpha+ and classic heuristics.
use ichnos_core::{AttributeValue, EventKeys, EventLog};
use ichnos_discovery::{
    AlphaOptions, AlphaPlusOptions, HeuristicsOptions, heuristics_net, petri_net_alpha,
    petri_net_alpha_plus, petri_net_heuristics,
};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_model::heuristics_net::Matrix;
use ichnos_model::{AcceptingPetriNet, Footprints, HeuristicsNet, Label};
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};

fn matrix<T: serde::Serialize>(m: &Matrix<T>) -> Value {
    json!(
        m.iter()
            .flat_map(|(a, row)| row
                .iter()
                .map(move |(b, v)| json!([a.as_str(), b.as_str(), v])))
            .collect::<Vec<_>>()
    )
}
fn describe(h: &HeuristicsNet) -> Value {
    json!({
        "activities": h.activities.iter().map(Label::as_str).collect::<Vec<_>>(),
        "activities_occurrences": h.activity_occurrences,
        "start_activities": h.start_activities, "end_activities": h.end_activities,
        "dfg": h.dfg.iter().map(|((a,b),v)| json!([a.as_str(),b.as_str(),v])).collect::<Vec<_>>(),
        "dfg_matrix": matrix(&h.dfg_matrix), "dependency_matrix": matrix(&h.dependency_matrix),
        "dfg_window_2_matrix": matrix(&h.dfg_window_2_matrix), "freq_triples_matrix": matrix(&h.freq_triples_matrix),
        "nodes": h.nodes.values().map(|n| {
            let edges = |m: &std::collections::BTreeMap<Label,Vec<ichnos_model::heuristics_net::HeuristicsEdge>>| m.iter().flat_map(|(a,es)| es.iter().map(move |e| json!({"target": a.as_str(), "dependency": e.dependency, "frequency": e.frequency}))).collect::<Vec<_>>();
            json!({"name": n.name.as_str(), "occurrences": n.occurrences,
                "inputs": edges(&n.inputs), "outputs": edges(&n.outputs),
                "and_measures_in": matrix(&n.and_measures_in), "and_measures_out": matrix(&n.and_measures_out), "loop_length_two": n.loop_length_two})
        }).collect::<Vec<_>>()
    })
}
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
fn options(v: &Value) -> HeuristicsOptions {
    let mut o = HeuristicsOptions::default();
    if let Some(x) = v["dependency_thresh"].as_f64() {
        o.dependency_threshold = x;
    }
    if let Some(x) = v["and_measure_thresh"].as_f64() {
        o.and_measure_threshold = x;
    }
    if let Some(x) = v["min_act_count"].as_u64() {
        o.min_activity_count = x;
    }
    if let Some(x) = v["min_dfg_occurrences"].as_u64() {
        o.min_dfg_occurrences = x;
    }
    if let Some(x) = v["dfg_pre_cleaning_noise_thresh"].as_f64() {
        o.dfg_pre_cleaning_noise_threshold = x;
    }
    if let Some(x) = v["loop_length_two_thresh"].as_f64() {
        o.loop_length_two_threshold = x;
    }
    o
}
fn check_case(id: &str) {
    let g = golden("discovery", &format!("classic-miners-{id}"));
    let keys = EventKeys::default().with_activity(g.expected["activity_key"].as_str().unwrap());
    let log = if g.expected["traces"].is_array() {
        let traces: Vec<Vec<String>> =
            serde_json::from_value(g.expected["traces"].clone()).unwrap();
        let mut log = EventLog::default();
        for row in traces {
            let mut trace = ichnos_core::Trace::new();
            for a in row {
                let mut event = ichnos_core::Event::new();
                event.attributes.insert(keys.activity.as_str(), a);
                trace.events.push(event);
            }
            log.traces.push(trace);
        }
        log
    } else if id.ends_with("xes") {
        ichnos_io::read_xes(g.fixture("log"), &Default::default()).unwrap()
    } else {
        ichnos_io::read_csv(g.fixture("log"), &ichnos_io::CsvReadOptions::default()).unwrap()
    };
    let original = log.clone();
    check_model(
        &petri_net_alpha(&log, &keys, &AlphaOptions::default()).unwrap(),
        &g.expected["models"]["alpha"],
        &format!("{id}/alpha"),
    );
    if g.expected["alpha_plus_error"].is_null() {
        for remove in [false, true] {
            let name = if remove {
                "alpha_plus_remove_unconnected"
            } else {
                "alpha_plus"
            };
            let net = petri_net_alpha_plus(
                &log,
                &keys,
                &AlphaPlusOptions {
                    remove_unconnected: remove,
                },
            )
            .unwrap();
            let expected = if let Some(runs) = g.expected["alpha_plus_runs"].as_array() {
                let actual = language(&net, 3);
                &runs
                    .iter()
                    .find(|run| run["models"][name]["language"] == actual)
                    .unwrap_or_else(|| {
                        panic!("{id}/{name}: language matches no pm4py hash-seed run")
                    })["models"][name]
            } else {
                &g.expected["models"][name]
            };
            check_model(&net, expected, &format!("{id}/{name}"));
        }
    } else {
        let shell = petri_net_alpha_plus(&log, &keys, &AlphaPlusOptions::default()).unwrap();
        assert_eq!(shell.net.transition_count(), 0);
        assert_eq!(shell.net.place_count(), 2);
    }
    for (i, v) in g.expected["heuristics"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let o = options(&v["options"]);
        let h = heuristics_net(&log, &keys, &o).unwrap();
        assert_json_eq(&describe(&h), &v["net"], &JsonCompare::default());
        let net = petri_net_heuristics(&log, &keys, &o).unwrap();
        assert_eq!(net, h.to_petri_net());
        check_model(
            &net,
            &g.expected["models"][format!("heuristics_{i}")],
            &format!("{id}/heuristics_{i}"),
        );
    }
    check_model(
        &petri_net_heuristics(&log, &keys, &HeuristicsOptions::default()).unwrap(),
        &g.expected["models"]["heuristics_public"],
        id,
    );
    assert_eq!(log, original, "miners mutated input");
}
macro_rules! classic_case {
    ($name:ident, $id:literal) => {
        #[test]
        fn $name() {
            check_case($id);
        }
    };
}
classic_case!(running_example, "running-example-xes");
classic_case!(receipt, "receipt-xes");
classic_case!(roadtraffic, "roadtraffic100traces-xes");
classic_case!(interleavings_even, "interleavings-receipt_even-csv");
classic_case!(interleavings_odd, "interleavings-receipt_odd-csv");
classic_case!(empty, "empty");
classic_case!(empty_trace, "empty-trace");
classic_case!(self_loop_only, "self-loop-only");
classic_case!(boundaries, "boundaries");
classic_case!(loops, "loops");
classic_case!(parallel, "parallel");
classic_case!(custom_key, "custom-key");
#[test]
fn validates_keys_thresholds_and_boundary_collisions() {
    let keys = EventKeys::default();
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let o = HeuristicsOptions {
            dependency_threshold: value,
            ..Default::default()
        };
        assert!(matches!(
            heuristics_net(&EventLog::default(), &keys, &o),
            Err(ichnos_discovery::Error::HeuristicsThreshold { .. })
        ));
    }
    let mut missing = EventLog::from_trace_strings(["a"], ",", &keys);
    missing.traces[0].events[0]
        .attributes
        .remove("concept:name");
    assert!(matches!(
        petri_net_alpha(&missing, &keys, &Default::default()),
        Err(ichnos_discovery::Error::Core(_))
    ));
    assert!(matches!(
        petri_net_alpha_plus(&missing, &keys, &Default::default()),
        Err(ichnos_discovery::Error::Core(_))
    ));
    assert!(matches!(
        heuristics_net(&missing, &keys, &Default::default()),
        Err(ichnos_discovery::Error::Core(_))
    ));
    let log = EventLog::from_trace_strings(["artificial_start,artificial_end"], ",", &keys);
    let net = petri_net_alpha_plus(&log, &keys, &Default::default()).unwrap();
    let visible: BTreeSet<_> = net
        .net
        .transitions()
        .filter_map(|(_, t)| t.label.as_ref().map(Label::as_str))
        .collect();
    assert_eq!(
        visible,
        BTreeSet::from(["artificial_start", "artificial_end"])
    );
    // Activity-only discovery remains valid with wrong or absent timestamps.
    let mut log = log;
    log.traces[0].events[0].insert("time:timestamp", AttributeValue::from(false));
    log.traces[0].events[1].attributes.remove("time:timestamp");
    heuristics_net(&log, &keys, &Default::default()).unwrap();
}

#[test]
fn cleaned_loop_with_zero_minimum() {
    check_case("cleaned-loop");
}
