use ichnos_core::EventLog;
use ichnos_golden::{JsonCompare, assert_json_eq, cases, fixture_path, golden};
use ichnos_model::{Dfg, ProcessTree};
use ichnos_sim::{
    DfgOptions, GeneratorOptions, Model, PlayOutOptions, generate_process_tree, parse_process_tree,
    play_out, play_out_dfg,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
fn counts(log: &EventLog) -> BTreeMap<Vec<String>, usize> {
    let mut result = BTreeMap::new();
    for t in &log.traces {
        let labels = t
            .events
            .iter()
            .map(|e| e.get("concept:name").unwrap().as_str().unwrap().to_string())
            .collect();
        *result.entry(labels).or_default() += 1;
    }
    result
}
fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual {actual}, expected {expected}, tolerance {tolerance}"
    );
}
#[test]
fn parse_entry_point_matches_goldens() {
    let g = golden("simulation", "parse-trees");
    for row in g.expected.as_array().unwrap() {
        let tree = parse_process_tree(row["input"].as_str().unwrap()).unwrap();
        assert_eq!(tree.to_string(), row["display"].as_str().unwrap());
        let mut leaves: Vec<_> = tree
            .leaves()
            .map(|l| l.label().map(|s| s.to_string()))
            .collect();
        leaves.sort();
        assert_eq!(json!(leaves), row["leaves"]);
    }
    assert!(parse_process_tree("->( 'a' ) junk").is_err());
}
#[test]
fn tree_playout_distributions() {
    for id in cases("simulation")
        .into_iter()
        .filter(|s| s.starts_with("playout-tree-"))
    {
        let g = golden("simulation", &id);
        let e = &g.expected;
        let tree = ProcessTree::parse(e["tree"].as_str().unwrap()).unwrap();
        let n = e["traces"].as_u64().unwrap() as usize;
        let options = PlayOutOptions {
            traces: n,
            ..Default::default()
        };
        let log = play_out(
            Model::Tree(&tree),
            &options,
            &mut ChaCha8Rng::seed_from_u64(1729),
        )
        .unwrap();
        let again = play_out(
            Model::Tree(&tree),
            &options,
            &mut ChaCha8Rng::seed_from_u64(1729),
        )
        .unwrap();
        assert_eq!(log, again);
        assert_eq!(log.traces.len(), n);
        assert!(log.traces.iter().all(|t| t.attributes.is_empty()));
        assert!(
            log.traces
                .iter()
                .flat_map(|t| &t.events)
                .all(|e| e.attributes.len() == 1)
        );
        let c = counts(&log);
        close(
            log.traces.iter().map(|t| t.len()).sum::<usize>() as f64 / n as f64,
            e["mean_length"].as_f64().unwrap(),
            0.08,
        );
        for pair in e["rates"].as_array().unwrap() {
            let trace: Vec<String> = serde_json::from_value(pair[0].clone()).unwrap();
            close(
                *c.get(&trace).unwrap_or(&0) as f64 / n as f64,
                pair[1].as_f64().unwrap(),
                0.025,
            );
        }
        if !id.ends_with("loop") {
            assert_eq!(c.len(), e["rates"].as_array().unwrap().len());
        }
    }
}
#[test]
fn generated_population_distributions() {
    for id in cases("simulation")
        .into_iter()
        .filter(|s| s.starts_with("generate-tree-"))
    {
        let g = golden("simulation", &id);
        let e = &g.expected;
        let params = &e["params"];
        let mut rng = ChaCha8Rng::seed_from_u64(1729);
        let o = GeneratorOptions {
            min: 5,
            mode: 10,
            max: 15,
            operator_weights: ["sequence", "choice", "parallel", "loop", "or"]
                .map(|k| params[k].as_f64().unwrap()),
            ..Default::default()
        };
        let mut sizes = Vec::new();
        let mut operators = BTreeMap::new();
        fn walk(t: &ProcessTree, ops: &mut BTreeMap<String, usize>) {
            if let Some(op) = t.operator() {
                assert_eq!(t.children().len(), 2);
                *ops.entry(op.symbol().to_string()).or_default() += 1;
            }
            for child in t.children() {
                walk(child, ops);
            }
        }
        for _ in 0..1000 {
            let t = generate_process_tree(&o, &mut rng).unwrap();
            t.validate().unwrap();
            sizes.push(t.leaves().filter(|l| l.label().is_some()).count());
            walk(&t, &mut operators);
        }
        assert!(sizes.iter().all(|n| (5..=15).contains(n)));
        close(
            sizes.iter().sum::<usize>() as f64 / 1000.,
            e["visible_mean"].as_f64().unwrap(),
            0.25,
        );
        let total = operators.values().sum::<usize>() as f64;
        for (op, p) in e["operator_rates"].as_object().unwrap() {
            close(
                operators.get(op).copied().unwrap_or(0) as f64 / total,
                p.as_f64().unwrap(),
                0.025,
            );
        }
    }
}
#[test]
fn dfg_variants_and_probabilities_match() {
    for id in cases("simulation")
        .into_iter()
        .filter(|s| s.starts_with("playout-dfg-"))
    {
        let g = golden("simulation", &id);
        let e = &g.expected;
        let mut dfg = Dfg::new();
        for row in e["graph"].as_array().unwrap() {
            dfg.add_edge(
                row[0].as_str().unwrap(),
                row[1].as_str().unwrap(),
                row[2].as_u64().unwrap(),
            );
        }
        for (a, n) in e["starts"].as_object().unwrap() {
            dfg.add_start(a.as_str(), n.as_u64().unwrap());
        }
        for (a, n) in e["ends"].as_object().unwrap() {
            dfg.add_end(a.as_str(), n.as_u64().unwrap());
        }
        let log = play_out_dfg(
            &dfg,
            &DfgOptions {
                max_variants: 20,
                ..Default::default()
            },
        )
        .unwrap();
        let actual:Vec<_>=log.traces.iter().map(|t|json!({"activities":t.events.iter().map(|e|e.get("concept:name").unwrap().as_str().unwrap()).collect::<Vec<_>>(),"probability":t.attributes.get("probability").unwrap().as_f64().unwrap()})).collect();
        assert_json_eq(&json!(actual), &e["variants"], &JsonCompare::default());
    }
}
#[test]
fn petri_playout_distribution() {
    let g = golden("simulation", "playout-petri-running-example");
    let e = &g.expected;
    let net = ichnos_io::pnml::read_pnml(fixture_path("running-example.pnml"), &Default::default())
        .unwrap()
        .model;
    let log = play_out(
        Model::PetriNet(&net),
        &PlayOutOptions {
            traces: 5000,
            max_trace_length: 100,
            ..Default::default()
        },
        &mut ChaCha8Rng::seed_from_u64(1729),
    )
    .unwrap();
    let c = counts(&log);
    let activities: BTreeSet<_> = c.keys().flatten().cloned().collect();
    assert_eq!(json!(activities), e["activities"]);
    close(
        log.traces.iter().map(|t| t.len()).sum::<usize>() as f64 / 5000.,
        e["mean_length"].as_f64().unwrap(),
        0.15,
    );
    for row in e["rates"].as_array().unwrap() {
        let trace: Vec<String> = serde_json::from_value(row[0].clone()).unwrap();
        close(
            *c.get(&trace).unwrap_or(&0) as f64 / 5000.,
            row[1].as_f64().unwrap(),
            0.035,
        );
    }
    let times: Vec<_> = log
        .traces
        .iter()
        .flat_map(|t| &t.events)
        .map(|e| {
            e.get("time:timestamp")
                .unwrap()
                .as_date()
                .unwrap()
                .timestamp()
        })
        .collect();
    assert!(times.windows(2).all(|w| w[1] - w[0] == 1));
}
#[test]
fn rejects_invalid_options_and_bounds() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let tree = ProcessTree::looped(ProcessTree::Tau, ProcessTree::Tau);
    assert!(
        play_out(
            Model::Tree(&tree),
            &PlayOutOptions {
                max_steps: 1,
                traces: 1,
                ..Default::default()
            },
            &mut rng
        )
        .is_err()
    );
    assert!(
        play_out(
            Model::Tree(&ProcessTree::or([ProcessTree::activity("a")])),
            &PlayOutOptions::default(),
            &mut rng
        )
        .is_err()
    );
    let options = GeneratorOptions {
        operator_weights: [0.; 5],
        ..Default::default()
    };
    assert!(generate_process_tree(&options, &mut rng).is_err());
    assert!(play_out_dfg(&Dfg::new(), &DfgOptions::default()).is_err());
    let t = ProcessTree::sequence([ProcessTree::activity("a"), ProcessTree::Tau]);
    let options = PlayOutOptions {
        traces: 0,
        ..Default::default()
    };
    assert!(
        play_out(Model::Tree(&t), &options, &mut rng)
            .unwrap()
            .traces
            .is_empty()
    );
}

#[test]
fn explicit_weights_final_markings_and_silent_bounds() {
    use ichnos_model::petri::{AcceptingPetriNet, Marking, PetriNet};
    let mut net = PetriNet::new("weighted choice");
    let start = net.add_place("start");
    let end = net.add_place("end");
    let a = net.add_transition("a", Some("a"));
    let b = net.add_transition("b", Some("b"));
    for t in [a, b] {
        net.add_input_arc(start, t).unwrap();
        net.add_output_arc(t, end).unwrap();
    }
    let mut initial = Marking::new();
    initial.set(start, 1);
    let mut final_marking = Marking::new();
    final_marking.set(end, 1);
    let model = AcceptingPetriNet::new(net, initial, final_marking);
    let options = PlayOutOptions {
        traces: 4000,
        require_final: true,
        transition_weights: [(a, 3.), (b, 1.)].into_iter().collect(),
        ..Default::default()
    };
    let log = play_out(
        Model::PetriNet(&model),
        &options,
        &mut ChaCha8Rng::seed_from_u64(42),
    )
    .unwrap();
    assert!(log.traces.iter().all(|t| t.len() == 1));
    let count = counts(&log);
    close(count[&vec!["a".into()]] as f64 / 4000., 0.75, 0.035);
    let mut net = PetriNet::new("silent cycle");
    let p = net.add_place("p");
    let q = net.add_place("q");
    let tau = net.add_transition("tau", None::<&str>);
    net.add_input_arc(p, tau).unwrap();
    net.add_output_arc(tau, p).unwrap();
    let mut initial = Marking::new();
    initial.set(p, 1);
    let mut final_marking = Marking::new();
    final_marking.set(q, 1);
    let model = AcceptingPetriNet::new(net, initial, final_marking);
    assert!(
        play_out(
            Model::PetriNet(&model),
            &PlayOutOptions {
                traces: 1,
                max_steps: 3,
                ..Default::default()
            },
            &mut ChaCha8Rng::seed_from_u64(42)
        )
        .is_err()
    );
}

#[test]
fn duplicate_label_growth_is_bounded() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let options = GeneratorOptions {
        min: 10,
        mode: 10,
        max: 10,
        duplicate: 0.5,
        ..Default::default()
    };
    let mut duplicated = false;
    for _ in 0..50 {
        let tree = generate_process_tree(&options, &mut rng).unwrap();
        tree.validate().unwrap();
        let visible = tree.leaves().filter(|l| l.label().is_some()).count();
        duplicated |= tree.activities().len() < visible;
    }
    assert!(duplicated);
    let options = GeneratorOptions {
        duplicate: 1.,
        ..options
    };
    assert!(generate_process_tree(&options, &mut rng).is_ok());
}

#[test]
fn final_marking_stop_and_superset_goldens() {
    use ichnos_model::petri::{AcceptingPetriNet, Marking, PetriNet};
    let g = golden("simulation", "playout-petri-final-stop");
    for row in g.expected.as_array().unwrap() {
        let mut net = PetriNet::new("final with outgoing transition");
        let p = net.add_place("p");
        let q = net.add_place("q");
        let a = net.add_transition("a", Some("a"));
        net.add_input_arc(p, a).unwrap();
        net.add_output_arc(a, q).unwrap();
        let mut im = Marking::new();
        im.set(p, row["tokens"].as_u64().unwrap() as u32);
        let mut fm = Marking::new();
        fm.set(p, 1);
        let model = AcceptingPetriNet::new(net, im, fm);
        let options = PlayOutOptions {
            traces: 5000,
            max_trace_length: 3,
            require_final: row["require_final"].as_bool().unwrap(),
            final_marking_leq: row["leq"].as_bool().unwrap(),
            ..Default::default()
        };
        let result = play_out(
            Model::PetriNet(&model),
            &options,
            &mut ChaCha8Rng::seed_from_u64(1729),
        );
        if row["traces"].as_u64().unwrap() == 0 {
            assert!(matches!(
                result,
                Err(ichnos_sim::Error::Limit("Petri-net attempts"))
            ));
            continue;
        }
        let log = result.unwrap();
        assert_eq!(log.traces.len(), 5000);
        let observed = counts(&log);
        for pair in row["rates"].as_array().unwrap() {
            let sequence: Vec<String> = serde_json::from_value(pair[0].clone()).unwrap();
            close(
                observed.get(&sequence).copied().unwrap_or(0) as f64 / 5000.,
                pair[1].as_f64().unwrap(),
                0.035,
            );
        }
        assert_eq!(observed.len(), row["rates"].as_array().unwrap().len());
    }
}

#[test]
fn unrestricted_petri_traces_do_not_use_attempt_limit() {
    use ichnos_model::petri::{AcceptingPetriNet, Marking, PetriNet};
    let net = AcceptingPetriNet::new(PetriNet::new("empty"), Marking::new(), Marking::new());
    let options = PlayOutOptions {
        traces: 150_000,
        max_attempts: 1,
        ..Default::default()
    };
    assert_eq!(
        play_out(
            Model::PetriNet(&net),
            &options,
            &mut ChaCha8Rng::seed_from_u64(1)
        )
        .unwrap()
        .traces
        .len(),
        150_000
    );
}
