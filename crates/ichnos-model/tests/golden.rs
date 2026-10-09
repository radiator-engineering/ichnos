//! Footprints of the fixture models against pm4py's (`fixtures/golden/model`).
//!
//! There is no PNML or PTML reader yet, so each golden file also describes
//! its input model; see `tools/golden/cases/model.py`.

use std::collections::BTreeMap;

use ichnos_golden::{Golden, JsonCompare, assert_json_eq, cases, golden};
use ichnos_model::petri::{ArcEnds, ArcKind, ReachabilityOptions};
use ichnos_model::{Footprints, Label, Marking, PetriNet, ProcessTree, TreeFootprints};
use serde_json::Value;

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {v}"))
}

fn build_net(model: &Value) -> (PetriNet, Marking) {
    let mut net = PetriNet::new("golden");
    let mut places = BTreeMap::new();
    for p in model["places"].as_array().expect("places") {
        let name = p.as_str().expect("place name");
        places.insert(name.to_owned(), net.add_place(name));
    }
    let mut transitions = BTreeMap::new();
    for t in model["transitions"].as_array().expect("transitions") {
        let name = str_field(t, "name");
        let label = t["label"].as_str().map(Label::from);
        transitions.insert(name.to_owned(), net.add_transition(name, label));
    }
    for a in model["arcs"].as_array().expect("arcs") {
        let (source, target) = (str_field(a, "source"), str_field(a, "target"));
        let ends = match (places.get(source), transitions.get(target)) {
            (Some(&p), Some(&t)) => ArcEnds::PlaceToTransition(p, t),
            _ => ArcEnds::TransitionToPlace(transitions[source], places[target]),
        };
        let kind = match str_field(a, "type") {
            "normal" => ArcKind::Normal,
            "inhibitor" => ArcKind::Inhibitor,
            "reset" => ArcKind::Reset,
            other => panic!("unknown arc type {other}"),
        };
        let weight = u32::try_from(a["weight"].as_u64().expect("weight")).expect("weight fits");
        net.add_arc(ends, weight, kind).expect("valid arc");
    }
    let initial = model["initial_marking"]
        .as_object()
        .expect("initial marking")
        .iter()
        .map(|(p, n)| {
            let n = u32::try_from(n.as_u64().expect("token count")).expect("count fits");
            (places[p], n)
        })
        .collect();
    (net, initial)
}

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
