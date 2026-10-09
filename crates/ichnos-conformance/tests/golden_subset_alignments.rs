//! Subset and edit-distance alignments against pm4py
//! (`fixtures/golden/conformance/alignments-subset-*`).
//!
//! The exact searches break ties by transition label and name, and the
//! goldens use nets with canonical names, so moves, bounds and counters
//! must match pm4py's exactly.

mod common;

use std::collections::BTreeMap;

use ichnos_conformance::alignments::{
    DeviationCounts, SubsetOptions, SubsetSelection, SubsetSize, SubsetTraceAlignment,
    align_log_subset,
};
use ichnos_core::EventKeys;
use ichnos_golden::{Tolerance, cases, compare_close, golden};
use ichnos_model::{Label, PetriNet};
use serde_json::Value;

use common::{build_net, load_csv_log};

fn options(params: &Value) -> SubsetOptions {
    let selection = match params.get("selection_method").and_then(Value::as_str) {
        None | Some("frequency") => SubsetSelection::Frequency,
        Some("k_medoids") => SubsetSelection::KMedoids { max_iterations: 10 },
        Some(other) => panic!("unknown selection {other}"),
    };
    let size = match (params.get("subset_size"), params.get("subset_fraction")) {
        (Some(n), _) => SubsetSize::Count(n.as_u64().expect("size") as usize),
        (None, Some(f)) => SubsetSize::Fraction(f.as_f64().expect("fraction")),
        (None, None) => SubsetSize::Fraction(0.1),
    };
    SubsetOptions {
        selection,
        size,
        ..SubsetOptions::default()
    }
}

fn counts(map: &BTreeMap<Label, usize>) -> Value {
    Value::Object(
        map.iter()
            .map(|(k, v)| (k.as_str().to_owned(), Value::from(*v)))
            .collect(),
    )
}

fn deviations(d: &DeviationCounts) -> Value {
    serde_json::json!({
        "insertions": counts(&d.insertions),
        "deletions": counts(&d.deletions),
        "synchronous": counts(&d.synchronous),
    })
}

fn moves(net: &PetriNet, a: &SubsetTraceAlignment) -> Value {
    Value::Array(
        a.alignment
            .moves
            .iter()
            .map(|m| {
                let event = m.event().map_or(Value::Null, Value::from);
                let name = m.transition().map_or(Value::Null, |t| {
                    Value::from(net.transition(t).name.as_str())
                });
                Value::Array(vec![event, name])
            })
            .collect(),
    )
}

fn close(actual: f64, expected: &Value, what: &str) {
    let expected = expected.as_f64().expect("number");
    if let Err(m) = compare_close(actual, expected, Tolerance::METRIC) {
        panic!("{what}: {m}");
    }
}

#[test]
fn subset_alignments_match_pm4py() {
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with("alignments-subset-"))
        .collect();
    let keys = EventKeys::default();
    for id in &ids {
        let g = golden("conformance", id);
        let (net, im, fm) = build_net(&g.expected["model"]);
        let log = load_csv_log(&g.fixture("log"));
        for (run, r) in g.expected["runs"].as_object().expect("runs") {
            let result = align_log_subset(&log, &net, &im, &fm, &keys, &options(&r["params"]))
                .expect("subset");
            let expected = r["variants"].as_array().expect("variants");
            assert_eq!(
                result.log.alignments.len(),
                expected.len(),
                "{id} {run}: variants"
            );
            for (v, (a, e)) in result.log.alignments.iter().zip(expected).enumerate() {
                let what = format!("{id} {run} variant {v}");
                let a = a.as_ref().expect("every variant is aligned");
                assert_eq!(moves(&net, a), e["moves"], "{what}: moves");
                let t = &a.alignment;
                assert_eq!(t.cost, e["cost"].as_u64().expect("cost"), "{what}: cost");
                assert_eq!(
                    t.best_worst_cost,
                    e["bwc"].as_u64().expect("bwc"),
                    "{what}: bwc"
                );
                close(t.fitness, &e["fitness"], &format!("{what}: fitness"));
                close(
                    a.fitness_upper_bound,
                    &e["fitness_upper_bound"],
                    &format!("{what}: upper"),
                );
                close(
                    a.approximated_fitness(),
                    &e["approximated_fitness"],
                    &format!("{what}: approximated fitness"),
                );
                assert_eq!(
                    a.lower_bound_cost,
                    e["lower_bound_cost"].as_u64().expect("lower"),
                    "{what}"
                );
                assert_eq!(
                    Some(a.fitness_bounds_guaranteed),
                    e["fitness_bounds_guaranteed"].as_bool(),
                    "{what}: guaranteed"
                );
                assert_eq!(
                    Some(a.selected_exact),
                    e["selected_exact"].as_bool(),
                    "{what}: exact"
                );
                let rep: Vec<&str> = a.representative.iter().map(Label::as_str).collect();
                assert_eq!(
                    serde_json::json!(rep),
                    e["representative"],
                    "{what}: representative"
                );
                assert_eq!(
                    deviations(&a.deviations),
                    e["deviation_counts"],
                    "{what}: deviations"
                );
                assert_eq!(
                    t.visited_states as u64,
                    e["visited"].as_u64().expect("visited"),
                    "{what}"
                );
                assert_eq!(
                    t.queued_states as u64,
                    e["queued"].as_u64().expect("queued"),
                    "{what}"
                );
                assert_eq!(
                    t.traversed_arcs as u64,
                    e["traversed"].as_u64().expect("traversed"),
                    "{what}"
                );
                assert_eq!(Some(a.is_valid), e["is_valid"].as_bool(), "{what}: valid");
                assert_eq!(
                    result.representatives as u64,
                    e["subset_size"].as_u64().expect("subset size"),
                    "{what}: subset size"
                );
            }
            let s = result.summary();
            let es = &r["summary"];
            close(
                s.log_fitness,
                &es["log_fitness"],
                &format!("{id} {run}: log fitness"),
            );
            close(
                s.fitness_lower_bound,
                &es["fitness_lower_bound"],
                &format!("{id} {run}: lower"),
            );
            close(
                s.fitness_upper_bound,
                &es["fitness_upper_bound"],
                &format!("{id} {run}: upper"),
            );
            assert_eq!(
                deviations(&s.deviations),
                es["deviation_counts"],
                "{id} {run}: deviations"
            );
        }
    }
    assert_eq!(
        ids.len(),
        8,
        "expected 8 subset alignment goldens, found {ids:?}"
    );
}
