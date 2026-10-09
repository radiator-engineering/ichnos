//! Alignments against DFGs, process trees and the variants of another log,
//! against pm4py (`fixtures/golden/conformance/alignments-{dfg,tree,edit-distance}-*`).
//!
//! These pm4py methods are not exact searches, and ichnos follows them step
//! for step, so the tests compare costs and fitness exactly. For edit
//! distance, pm4py's pick among equally close model variants depends on
//! Python's string hashing; the golden lists the cost of every pick it can
//! make.

mod common;

use std::collections::HashMap;

use ichnos_conformance::alignments::{
    DfgAligner, EditDistanceAligner, LogAlignment, SequenceAlignment, SequenceMove, TreeAligner,
};
use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{Golden, Tolerance, as_f64, cases, compare_close, golden};
use ichnos_model::{Dfg, ProcessTree};
use serde_json::Value;

use common::load_csv_log;

fn kind_cases(kind: &str, expected: usize) -> Vec<String> {
    let prefix = format!("alignments-{kind}-");
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with(&prefix))
        .collect();
    assert_eq!(ids.len(), expected, "{kind} goldens: {ids:?}");
    ids
}

/// The cost of each move under the method's cost model.
type MoveCost = fn(&SequenceMove) -> u64;

/// Checks every expected trace with `check`, and that the moves explain the
/// trace in order and add up to the cost.
fn check_traces(
    g: &Golden,
    log: &EventLog,
    result: &LogAlignment<SequenceAlignment>,
    move_cost: MoveCost,
    check: impl Fn(&Value, &SequenceAlignment, &str),
) {
    let expected = g.expected_at("/traces").as_array().expect("traces");
    assert_eq!(
        result.trace_count(),
        expected.len(),
        "{}: trace count",
        g.case
    );
    let index: HashMap<String, usize> = log
        .traces
        .iter()
        .enumerate()
        .map(|(i, t)| (t.case_id().expect("case id").to_string(), i))
        .collect();
    for e in expected {
        let case_id = e["case_id"].as_str().expect("case id");
        let i = index[case_id];
        let a = result.trace(i).expect("aligned");
        let what = format!("{} case {case_id}", g.case);
        check(e, a, &what);
        let len = log.traces[i].events.len();
        let events: Vec<usize> = a.moves.iter().filter_map(SequenceMove::event).collect();
        assert_eq!(events, (0..len).collect::<Vec<_>>(), "{what}: events");
        let moved: u64 = a.moves.iter().map(move_cost).sum();
        assert_eq!(moved, a.cost, "{what}: move costs");
    }
}

fn exact(actual: f64, expected: &Value, what: impl std::fmt::Display) {
    let expected = as_f64(expected).expect("number");
    if let Err(m) = compare_close(actual, expected, Tolerance::EXACT) {
        panic!("{what}: {m}");
    }
}

fn standard_cost(m: &SequenceMove) -> u64 {
    match m {
        SequenceMove::Sync { .. } => 0,
        _ => 10_000,
    }
}

fn tree_cost(m: &SequenceMove) -> u64 {
    match m {
        SequenceMove::Sync { .. } | SequenceMove::Model { activity: None } => 0,
        _ => 1,
    }
}

#[test]
fn dfg_alignments_match_pm4py() {
    let keys = EventKeys::default();
    for id in kind_cases("dfg", 3) {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let mut dfg = Dfg::new();
        for e in g.expected_at("/dfg").as_array().expect("dfg") {
            let n = e[2].as_u64().expect("count");
            dfg.add_edge(e[0].as_str().unwrap(), e[1].as_str().unwrap(), n);
        }
        for (a, n) in g.expected_at("/start").as_object().expect("start") {
            dfg.add_start(a.as_str(), n.as_u64().unwrap());
        }
        for (a, n) in g.expected_at("/end").as_object().expect("end") {
            dfg.add_end(a.as_str(), n.as_u64().unwrap());
        }
        let result = DfgAligner::new(&dfg)
            .expect("connected DFG")
            .align_log(&log, &keys)
            .expect("aligned");
        check_traces(&g, &log, &result, standard_cost, |e, a, what| {
            assert_eq!(a.cost, e["cost"].as_u64().unwrap(), "{what}: cost");
            assert_eq!(a.best_worst_cost, e["bwc"].as_u64().unwrap(), "{what}: bwc");
            exact(a.fitness, &e["fitness"], format_args!("{what}: fitness"));
            assert_eq!(
                a.visited_states as u64,
                e["visited"].as_u64().unwrap(),
                "{what}: visited"
            );
            assert_eq!(
                a.closed_states as u64,
                e["closed"].as_u64().unwrap(),
                "{what}: closed"
            );
        });
    }
}

#[test]
fn tree_alignments_match_pm4py() {
    let keys = EventKeys::default();
    for id in kind_cases("tree", 4) {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let tree = ProcessTree::parse(g.expected_at("/tree").as_str().expect("tree"))
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let result = TreeAligner::new(&tree)
            .expect("supported tree")
            .align_log(&log, &keys)
            .expect("aligned");
        check_traces(&g, &log, &result, tree_cost, |e, a, what| {
            assert_eq!(a.cost, e["cost"].as_u64().unwrap(), "{what}: cost");
            exact(a.fitness, &e["fitness"], format_args!("{what}: fitness"));
        });
    }
}

#[test]
fn edit_distance_alignments_match_pm4py() {
    let keys = EventKeys::default();
    for id in kind_cases("edit-distance", 3) {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let model: Vec<Vec<&str>> = g
            .expected_at("/model")
            .as_array()
            .expect("model")
            .iter()
            .map(|t| {
                t.as_array()
                    .expect("variant")
                    .iter()
                    .map(|a| a.as_str().expect("activity"))
                    .collect()
            })
            .collect();
        let result = EditDistanceAligner::new(&model)
            .expect("model traces")
            .align_log(&log, &keys)
            .expect("aligned");
        check_traces(&g, &log, &result, standard_cost, |e, a, what| {
            assert_eq!(a.best_worst_cost, e["bwc"].as_u64().unwrap(), "{what}: bwc");
            let choices: Vec<u64> = e["cost_choices"]
                .as_array()
                .expect("choices")
                .iter()
                .map(|c| c.as_u64().unwrap())
                .collect();
            assert!(
                choices.contains(&a.cost),
                "{what}: cost {} not in {choices:?}",
                a.cost
            );
            if let Some(cost) = e.get("cost") {
                assert_eq!(a.cost, cost.as_u64().unwrap(), "{what}: cost");
                exact(a.fitness, &e["fitness"], format_args!("{what}: fitness"));
            }
        });
    }
}
