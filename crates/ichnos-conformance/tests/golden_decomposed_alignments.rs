//! Decomposed alignments against pm4py
//! (`fixtures/golden/conformance/alignments-decomposed-*`).
//!
//! pm4py orders components and breaks ties in each component's search by
//! object hashes, so the goldens record every cost and alignment seen over a
//! set of hash seeds. The ichnos search breaks ties differently, and a tie
//! can change which components merge. So the test requires ichnos's cost to
//! be one of pm4py's, its `bwc` and fitness to match, and its log moves to
//! replay the trace. It reports how many alignments match one of pm4py's
//! exactly, or up to the order of moves, without requiring either.

mod common;

use ichnos_conformance::alignments::{DecomposedAligner, DecomposedOptions, Move};
use ichnos_golden::{cases, golden};
use ichnos_model::PetriNet;
use serde_json::Value;

use common::build_net;

fn pair(net: &PetriNet, trace: &[String], m: &Move) -> Value {
    let log = m.event().map_or(Value::Null, |e| Value::from(trace[e].as_str()));
    let model = m.transition().map_or(Value::Null, |t| {
        net.transition(t)
            .label
            .as_ref()
            .map_or(Value::Null, |l| Value::from(l.as_str()))
    });
    Value::Array(vec![log, model])
}

/// Whether two alignments list the same moves, in any order.
fn same_moves(expected: &Value, actual: &Value) -> bool {
    let sorted = |v: &Value| {
        let mut rows: Vec<String> = v
            .as_array()
            .expect("alignment")
            .iter()
            .map(Value::to_string)
            .collect();
        rows.sort();
        rows
    };
    sorted(expected) == sorted(actual)
}

#[test]
fn decomposed_alignments_match_pm4py() {
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with("alignments-decomposed-"))
        .collect();
    let (mut exact, mut multiset, mut total) = (0, 0, 0);
    for id in &ids {
        let g = golden("conformance", id);
        let (net, im, fm) = build_net(&g.expected["model"]);
        let aligner =
            DecomposedAligner::new(&net, &im, &fm, DecomposedOptions::default()).expect("aligner");
        for (v, e) in g.expected["variants"].as_array().expect("variants").iter().enumerate() {
            let what = format!("{id} variant {v}");
            let trace: Vec<String> = serde_json::from_value(e["trace"].clone()).expect("trace");
            let a = aligner.align(&trace).expect("align").expect("aligned");
            let costs = e["cost_choices"].as_array().expect("costs");
            assert!(
                costs.iter().any(|c| c.as_u64() == Some(a.cost)),
                "{what}: cost {} not in {costs:?}",
                a.cost
            );
            let bwc = e["bwc"].as_u64().expect("bwc");
            assert_eq!(a.best_worst_cost, bwc, "{what}: bwc");
            let denominator = (bwc / 10000) as f64;
            let fitness = if denominator == 0.0 {
                0.0
            } else {
                1.0 - (a.cost / 10000) as f64 / denominator
            };
            assert_eq!(a.fitness, fitness, "{what}: fitness");
            let events: Vec<usize> = a.moves.iter().filter_map(Move::event).collect();
            assert_eq!(events, (0..trace.len()).collect::<Vec<_>>(), "{what}: log moves");

            let moves = Value::Array(a.moves.iter().map(|m| pair(&net, &trace, m)).collect());
            let choices = e["alignment_choices"].as_array().expect("alignments");
            total += 1;
            if choices.contains(&moves) {
                exact += 1;
            }
            if choices.iter().any(|c| same_moves(c, &moves)) {
                multiset += 1;
            }
        }
    }
    eprintln!("{total} variants: {exact} exact, {multiset} same moves in another order");
    assert_eq!(ids.len(), 8, "expected 8 decomposed alignment goldens, found {ids:?}");
}
