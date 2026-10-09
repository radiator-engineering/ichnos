use ichnos_io::*;
use ichnos_model::{
    Marking, PetriNet, ProcessTree,
    petri::{ArcEnds, ArcKind, ReachabilityOptions},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

// pm4py's default footprint algorithm uses ClassicSemantics even for special
// arcs. Match that oracle on a copy; imported arc kinds are checked separately.
fn classic_footprints(document: &PnmlDocument) -> Option<Value> {
    let original = &document.model.net;
    let mut net = PetriNet::new(&original.name);
    let places: BTreeMap<_, _> = original
        .places()
        .map(|(id, p)| (id, net.add_place(&p.name)))
        .collect();
    let transitions: BTreeMap<_, _> = original
        .transitions()
        .map(|(id, t)| (id, net.add_transition(&t.name, t.label.clone())))
        .collect();
    for (_, arc) in original.arcs() {
        let ends = match arc.ends {
            ArcEnds::PlaceToTransition(p, t) => {
                ArcEnds::PlaceToTransition(places[&p], transitions[&t])
            }
            ArcEnds::TransitionToPlace(t, p) => {
                ArcEnds::TransitionToPlace(transitions[&t], places[&p])
            }
        };
        net.add_arc(ends, arc.weight, ArcKind::Normal).unwrap();
    }
    let initial: Marking = document
        .model
        .initial_marking
        .iter()
        .map(|(p, n)| (places[&p], n))
        .collect();
    net.footprints(
        &initial,
        ReachabilityOptions {
            max_markings: 10000,
        },
    )
    .ok()
    .map(|fp| serde_json::to_value(fp).unwrap())
}

fn pnml_summary(document: &PnmlDocument, unbounded: bool) -> Value {
    let net = &document.model.net;
    let footprints = if unbounded {
        None
    } else {
        classic_footprints(document)
    };
    let status = if unbounded {
        "unbounded"
    } else if footprints.is_none() {
        "state_space_limit"
    } else {
        "complete"
    };
    let mut stochastic: Vec<Value> = document
        .stochastic
        .iter()
        .map(|(id, info)| {
            json!({
                "label": net.transition(*id).label,
                "distribution_type": info.distribution_type,
                "priority": info.priority, "weight": info.weight,
            })
        })
        .collect();
    stochastic.sort_by_key(|value| serde_json::to_string(value).unwrap());
    json!({"model_kind": "petri_net", "places": net.places().count(),
        "transitions": net.transitions().count(), "arcs": net.arcs().count(),
        "silent_transitions": net.transitions().filter(|(_,t)| t.is_silent()).count(),
        "inhibitor_arcs": net.arcs().filter(|(_,a)| a.kind==ArcKind::Inhibitor).count(),
        "reset_arcs": net.arcs().filter(|(_,a)| a.kind==ArcKind::Reset).count(),
        "initial_tokens": document.model.initial_marking.iter().map(|(_,n)| u64::from(n)).sum::<u64>(),
        "final_tokens": document.model.final_marking.iter().map(|(_,n)| u64::from(n)).sum::<u64>(),
        "stochastic": stochastic, "footprints": footprints, "footprints_status": status})
}

fn tree_counts(tree: &ProcessTree) -> (usize, usize, usize) {
    let mut counts = (
        1,
        usize::from(tree.label().is_some()),
        usize::from(tree.is_tau()),
    );
    for child in tree.children() {
        let (n, a, s) = tree_counts(child);
        counts.0 += n;
        counts.1 += a;
        counts.2 += s;
    }
    counts
}

#[test]
fn every_model_fixture_matches_oracle_and_round_trips() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut checked = 0;
    for entry in fs::read_dir(root.join("fixtures/golden/io")).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("model-")
        {
            continue;
        }
        let golden: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let fixture = golden["meta"]["fixtures"]["model"].as_str().unwrap();
        let actual = match Path::new(fixture).extension().unwrap().to_str().unwrap() {
            "pnml" => {
                let document = read_pnml(root.join(fixture), &Default::default())
                    .unwrap_or_else(|e| panic!("{fixture}: {e}"));
                let mut xml = Vec::new();
                write_pnml_to_writer(&document, &mut xml, &Default::default()).unwrap();
                let back = read_pnml_from_reader(xml.as_slice(), &Default::default()).unwrap();
                assert_eq!(document, back, "{fixture}: PNML round trip");
                pnml_summary(&document, fixture.contains("inh_res_nets/"))
            }
            "ptml" => {
                let tree = read_ptml(root.join(fixture), &Default::default())
                    .unwrap_or_else(|e| panic!("{fixture}: {e}"));
                let mut xml = Vec::new();
                write_ptml_to_writer(&tree, &mut xml, &Default::default()).unwrap();
                assert_eq!(
                    tree,
                    read_ptml_from_reader(xml.as_slice(), &Default::default()).unwrap(),
                    "{fixture}: PTML round trip"
                );
                let (nodes, activities, silent) = tree_counts(&tree);
                json!({"model_kind":"process_tree","nodes":nodes,"activity_nodes":activities,"silent_nodes":silent,"footprints":tree.footprints()})
            }
            "dfg" => {
                let dfg = read_dfg(root.join(fixture), &Default::default()).unwrap();
                let mut text = Vec::new();
                write_dfg_to_writer(&dfg, &mut text, &Default::default()).unwrap();
                assert_eq!(
                    dfg,
                    read_dfg_from_reader(text.as_slice(), &Default::default()).unwrap(),
                    "{fixture}: DFG round trip"
                );
                let parallel: Vec<_> = dfg
                    .graph
                    .keys()
                    .filter(|(a, b)| dfg.graph.contains_key(&(b.clone(), a.clone())))
                    .collect();
                let sequence: Vec<_> = dfg
                    .graph
                    .keys()
                    .filter(|(a, b)| !dfg.graph.contains_key(&(b.clone(), a.clone())))
                    .collect();
                json!({"model_kind":"dfg","edges":dfg.graph.len(),"edge_frequency":dfg.graph.values().sum::<u64>(),
                    "start_activities":dfg.start_activities,"end_activities":dfg.end_activities,
                    "frequencies":dfg.graph.iter().map(|((a,b),n)|json!([a,b,n])).collect::<Vec<_>>(),
                    "footprints":{"activities":dfg.edge_activities(),"start_activities":dfg.infer_start_activities(),"end_activities":dfg.infer_end_activities(),"parallel":parallel,"sequence":sequence}})
            }
            _ => unreachable!(),
        };
        assert_eq!(actual, golden["expected"], "{fixture}: oracle");
        checked += 1;
    }
    assert_eq!(
        checked, 30,
        "every bounded PNML, PTML and DFG fixture is covered"
    );
}
