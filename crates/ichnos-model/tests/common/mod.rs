//! Helpers shared by the golden tests.

use std::collections::BTreeMap;

use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::{AcceptingPetriNet, Label, Marking, PetriNet, PlaceId};
use serde_json::Value;

pub fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {v}"))
}

pub fn build_net(model: &Value) -> (PetriNet, Marking) {
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
    let initial = marking(&model["initial_marking"], &places);
    (net, initial)
}

fn marking(v: &Value, places: &BTreeMap<String, PlaceId>) -> Marking {
    v.as_object()
        .expect("a marking")
        .iter()
        .map(|(p, n)| {
            let n = u32::try_from(n.as_u64().expect("token count")).expect("count fits");
            (places[p], n)
        })
        .collect()
}

/// Builds the accepting Petri net a golden file describes, with both
/// markings.
#[allow(dead_code)]
pub fn build_accepting(model: &Value) -> AcceptingPetriNet {
    let (net, initial) = build_net(model);
    let places: BTreeMap<String, PlaceId> =
        net.places().map(|(id, p)| (p.name.clone(), id)).collect();
    let fin = marking(&model["final_marking"], &places);
    AcceptingPetriNet::new(net, initial, fin)
}
