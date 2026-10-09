//! Footprints, reachability graphs and heuristics nets against pm4py's
//! (`fixtures/golden/model`).
//!
//! There is no PNML or PTML reader yet, so each golden file also describes
//! its input model; see `tools/golden/cases/model.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_golden::{Golden, JsonCompare, assert_json_eq, cases, golden};
use ichnos_model::conversion::EdgeNaming;
use ichnos_model::heuristics_net::{
    DEFAULT_AND_MEASURE_THRESH, DEFAULT_LOOP_LENGTH_TWO_THRESH, HeuristicsEdge, HeuristicsNet,
    Matrix,
};
use ichnos_model::petri::ReachabilityOptions;
use ichnos_model::{AcceptingPetriNet, Footprints, Label, Marking, ProcessTree, TreeFootprints};
use serde_json::Value;

mod common;
use common::{build_net, str_field};

fn check(g: &Golden) {
    let model = g.expected_at("/model");
    let expected = g.expected_at("/footprints");
    let actual = match str_field(model, "kind") {
        "petri_net" => {
            let (net, im) = build_net(model);
            let fp = net
                .footprints(&im, ReachabilityOptions::default())
                .expect("bounded net");
            let back: Footprints = serde_json::from_value(expected.clone()).expect("deserialize");
            assert_eq!(back, fp, "{}: round trip", g.case);
            serde_json::to_value(&fp).expect("serialize")
        }
        "process_tree" => {
            let tree: ProcessTree = str_field(model, "tree").parse().expect("tree parses");
            let fp = tree.footprints();
            let back: TreeFootprints =
                serde_json::from_value(expected.clone()).expect("deserialize");
            assert_eq!(back, fp, "{}: round trip", g.case);
            serde_json::to_value(&fp).expect("serialize")
        }
        other => panic!("{}: unknown model kind {other}", g.case),
    };
    assert_json_eq(&actual, expected, &JsonCompare::default());
}

#[test]
fn footprints_match_pm4py() {
    let ids: Vec<String> = cases("model")
        .into_iter()
        .filter(|id| id.starts_with("footprints-"))
        .collect();
    assert_eq!(
        ids.len(),
        14,
        "expected 14 footprint goldens, found {ids:?}"
    );
    for id in ids {
        check(&golden("model", &id));
    }
}

#[test]
fn reachability_graphs_match_pm4py() {
    let ids: Vec<String> = cases("model")
        .into_iter()
        .filter(|id| id.starts_with("reachability-graph-"))
        .collect();
    assert_eq!(
        ids.len(),
        10,
        "expected 10 reachability goldens, found {ids:?}"
    );
    for id in ids {
        let g = golden("model", &id);
        let (net, im) = build_net(g.expected_at("/model"));
        let ts = net
            .to_transition_system(&im, ReachabilityOptions::default(), EdgeNaming::Repr)
            .expect("bounded net");
        // pm4py states are equal when their names are, so compare name sets.
        let states: BTreeSet<&str> = ts.states().map(|(_, s)| s.name.as_str()).collect();
        let edges: BTreeSet<[&str; 3]> = ts
            .edges()
            .map(|(_, e)| {
                [
                    ts.state(e.from()).name.as_str(),
                    e.name.as_str(),
                    ts.state(e.to()).name.as_str(),
                ]
            })
            .collect();
        let actual = serde_json::json!({ "states": states, "edges": edges });
        let expected = serde_json::json!({
            "states": g.expected_at("/states"),
            "edges": g.expected_at("/edges"),
        });
        assert_json_eq(&actual, &expected, &JsonCompare::default());
    }
}

fn as_u64(v: &Value) -> u64 {
    v.as_u64().unwrap_or_else(|| panic!("{v} is not a count"))
}

fn as_f64(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("{v} is not a number"))
}

fn counts(v: &Value) -> BTreeMap<Label, u64> {
    v.as_object()
        .expect("object of counts")
        .iter()
        .map(|(k, n)| (Label::from(k.as_str()), as_u64(n)))
        .collect()
}

fn matrix<T>(v: &Value, value: impl Fn(&Value) -> T) -> Matrix<T> {
    let mut m: Matrix<T> = Matrix::new();
    for row in v.as_array().expect("matrix rows") {
        m.entry(Label::from(row[0].as_str().expect("activity")))
            .or_default()
            .insert(
                Label::from(row[1].as_str().expect("activity")),
                value(&row[2]),
            );
    }
    m
}

fn edges(v: &Value) -> impl Iterator<Item = (Label, HeuristicsEdge)> + '_ {
    v.as_array().expect("edges").iter().map(|e| {
        (
            Label::from(str_field(e, "target")),
            HeuristicsEdge {
                dependency: as_f64(&e["dependency"]),
                frequency: as_u64(&e["frequency"]),
            },
        )
    })
}

/// The heuristics net the golden describes, with its connections but
/// without the node measures, which the test recomputes.
fn build_heuristics_net(d: &Value) -> HeuristicsNet {
    let mut h = HeuristicsNet {
        activities: d["activities"]
            .as_array()
            .expect("activities")
            .iter()
            .map(|a| Label::from(a.as_str().expect("activity")))
            .collect(),
        activity_occurrences: counts(&d["activities_occurrences"]),
        start_activities: d["start_activities"]
            .as_array()
            .expect("starts")
            .iter()
            .map(counts)
            .collect(),
        end_activities: d["end_activities"]
            .as_array()
            .expect("ends")
            .iter()
            .map(counts)
            .collect(),
        dfg_matrix: matrix(&d["dfg_matrix"], as_u64),
        dependency_matrix: matrix(&d["dependency_matrix"], as_f64),
        freq_triples_matrix: matrix(&d["freq_triples_matrix"], as_u64),
        ..HeuristicsNet::default()
    };
    for node in d["nodes"].as_array().expect("nodes") {
        let name = Label::from(str_field(node, "name"));
        h.node_mut(&name).occurrences = as_u64(&node["occurrences"]);
        for (to, e) in edges(&node["outputs"]) {
            h.add_output_connection(&name, &to, e);
        }
        for (from, e) in edges(&node["inputs"]) {
            h.add_input_connection(&name, &from, e);
        }
    }
    h
}

/// The converted net in the golden's form: places, and per transition its
/// label with the names of its input and output places.
fn describe_converted(apn: &AcceptingPetriNet) -> Value {
    let net = &apn.net;
    let names = |ps: Vec<ichnos_model::PlaceId>| {
        let mut v: Vec<&str> = ps.into_iter().map(|p| net.place(p).name.as_str()).collect();
        v.sort_unstable();
        v
    };
    let mut places: Vec<&str> = net.places().map(|(_, p)| p.name.as_str()).collect();
    places.sort_unstable();
    let mut transitions: Vec<(&str, Vec<&str>, Vec<&str>)> = net
        .transitions()
        .map(|(id, t)| {
            (
                t.label.as_deref().unwrap_or(""),
                names(net.preset(id).collect()),
                names(net.postset(id).collect()),
            )
        })
        .collect();
    transitions.sort();
    let marking = |m: &Marking| -> BTreeMap<&str, u32> {
        m.iter()
            .map(|(p, n)| (net.place(p).name.as_str(), n))
            .collect()
    };
    serde_json::json!({
        "places": places,
        "transitions": transitions
            .into_iter()
            .map(|(label, pre, post)| serde_json::json!({
                "label": (!label.is_empty()).then_some(label),
                "preset": pre,
                "postset": post,
            }))
            .collect::<Vec<_>>(),
        "arcs": net.arc_count(),
        "initial_marking": marking(&apn.initial_marking),
        "final_marking": marking(&apn.final_marking),
    })
}

#[test]
fn heuristics_nets_match_pm4py() {
    let ids: Vec<String> = cases("model")
        .into_iter()
        .filter(|id| id.starts_with("heuristics-net-"))
        .collect();
    assert_eq!(
        ids.len(),
        4,
        "expected 4 heuristics net goldens, found {ids:?}"
    );
    for id in ids {
        let g = golden("model", &id);
        let d = g.expected_at("/heuristics_net");
        let mut h = build_heuristics_net(d);

        // pm4py computes the node measures before it adds the connections
        // of length-two loops, which have dependency 0.
        let mut before = h.clone();
        for node in before.nodes.values_mut() {
            node.outputs
                .retain(|_, es| es.iter().any(|e| e.dependency != 0.0));
            node.inputs
                .retain(|_, es| es.iter().any(|e| e.dependency != 0.0));
        }
        // Nodes that only length-two loop connections bring in are added
        // after the measures too, so they get none.
        let loop_only: Vec<Label> = before
            .nodes
            .iter()
            .filter(|(_, n)| n.outputs.is_empty() && n.inputs.is_empty())
            .map(|(name, _)| name.clone())
            .collect();
        if loop_only.len() < before.nodes.len() {
            for name in &loop_only {
                before.nodes.remove(name);
            }
        }
        before.calculate_node_measures(DEFAULT_AND_MEASURE_THRESH, DEFAULT_LOOP_LENGTH_TWO_THRESH);
        for node in d["nodes"].as_array().expect("nodes") {
            let name = Label::from(str_field(node, "name"));
            let Some(ours) = before.nodes.get(&name) else {
                continue;
            };
            let actual = serde_json::json!({
                "and_measures_in": flatten(&ours.and_measures_in),
                "and_measures_out": flatten(&ours.and_measures_out),
                "loop_length_two": ours.loop_length_two,
            });
            let expected = serde_json::json!({
                "and_measures_in": node["and_measures_in"],
                "and_measures_out": node["and_measures_out"],
                "loop_length_two": node["loop_length_two"],
            });
            assert_json_eq(&actual, &expected, &JsonCompare::default());
            let target = h.nodes.get_mut(&name).expect("node");
            target.and_measures_in = ours.and_measures_in.clone();
            target.and_measures_out = ours.and_measures_out.clone();
            target.loop_length_two = ours.loop_length_two.clone();
        }

        let actual = describe_converted(&h.to_petri_net());
        assert_json_eq(
            &actual,
            g.expected_at("/petri_net"),
            &JsonCompare::default(),
        );
    }
}

fn flatten(m: &Matrix<f64>) -> Vec<(&Label, &Label, f64)> {
    m.iter()
        .flat_map(|(a, row)| row.iter().map(move |(b, &v)| (a, b, v)))
        .collect()
}
