//! Petri net alignments, fitness and precision against pm4py
//! (`fixtures/golden/conformance/alignments-*`).
//!
//! pm4py picks one of several optimal alignments by hash order, so the tests
//! compare costs and fitness, not moves. Every heuristic must find the same
//! optimal cost.

mod common;

use std::collections::HashMap;

use ichnos_conformance::alignments::{
    Aligner, AlignmentOptions, Heuristic, LogAlignment, Move, precision_alignments,
};
use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{Golden, Tolerance, as_f64, cases, compare_close, golden};
use ichnos_model::PetriNet;

use common::{build_net, load_csv_log};

fn close(actual: f64, expected: f64, tol: Tolerance, what: impl std::fmt::Display) {
    if let Err(m) = compare_close(actual, expected, tol) {
        panic!("{what}: {m}");
    }
}

fn alignment_cases() -> Vec<String> {
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| {
            id.starts_with("alignments-")
                && !["dfg", "tree", "edit-distance", "approx", "subset", "decomposed", "discounted"]
                    .iter()
                    .any(|kind| id.starts_with(&format!("alignments-{kind}-")))
        })
        .collect();
    assert_eq!(ids.len(), 8, "expected 8 alignment goldens, found {ids:?}");
    ids
}

fn f64_at(g: &Golden, pointer: &str) -> f64 {
    as_f64(g.expected_at(pointer))
        .unwrap_or_else(|| panic!("{}: {pointer} is not a number", g.case))
}

/// Checks that `moves` is a valid run: the trace events in order, every
/// synchronous move on a transition with the event's label.
fn check_moves(net: &PetriNet, trace: &[&str], moves: &[Move]) {
    let events: Vec<usize> = moves.iter().filter_map(Move::event).collect();
    assert_eq!(events, (0..trace.len()).collect::<Vec<_>>());
    for m in moves {
        if let Move::Sync { event, transition } = *m {
            assert_eq!(
                net.transition(transition).label.as_deref(),
                Some(trace[event])
            );
        }
    }
}

fn check_log(g: &Golden, log: &EventLog, net: &PetriNet, result: &LogAlignment, what: &str) {
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
        let a = result.trace(i).expect("no time limit");
        let cost = e["cost"].as_u64().expect("cost");
        assert_eq!(a.cost, cost, "{} {what}: cost of case {case_id}", g.case);
        assert_eq!(
            a.best_worst_cost,
            e["bwc"].as_u64().expect("bwc"),
            "{} {what}: bwc of case {case_id}",
            g.case
        );
        close(
            a.fitness,
            as_f64(&e["fitness"]).expect("fitness"),
            Tolerance::EXACT,
            format_args!("{} {what}: fitness of case {case_id}", g.case),
        );
        let v = &result.variants;
        let variant = v.iter().find(|v| v.traces.contains(&i)).expect("variant");
        let names: Vec<&str> = v.names(variant).collect();
        check_moves(net, &names, &a.moves);
        let moved: u64 = a
            .moves
            .iter()
            .map(|m| match *m {
                Move::Sync { .. } => 0,
                Move::Log { .. } => 10_000,
                Move::Model { transition } => {
                    if net.transition(transition).is_silent() {
                        1
                    } else {
                        10_000
                    }
                }
            })
            .sum();
        assert_eq!(moved, a.cost, "{} {what}: moves of case {case_id}", g.case);
    }
    let fitness = result.fitness();
    close(
        fitness.log_fitness,
        f64_at(g, "/fitness/log_fitness"),
        Tolerance::METRIC,
        format_args!("{} {what}: log_fitness", g.case),
    );
    close(
        fitness.average_trace_fitness,
        f64_at(g, "/fitness/average_trace_fitness"),
        Tolerance::METRIC,
        format_args!("{} {what}: average_trace_fitness", g.case),
    );
    close(
        fitness.percentage_of_fitting_traces,
        f64_at(g, "/fitness/percentage_of_fitting_traces"),
        Tolerance::METRIC,
        format_args!("{} {what}: percentage_of_fitting_traces", g.case),
    );
}

fn run(heuristic: Heuristic) {
    let keys = EventKeys::default();
    for id in alignment_cases() {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let (net, im, fm) = build_net(g.expected_at("/model"));
        let options = AlignmentOptions::default().heuristic(heuristic).threads(4);
        let aligner = Aligner::new(&net, &im, &fm, options).expect("easy sound net");
        let result = aligner.align_log(&log, &keys).expect("aligned");
        check_log(&g, &log, &net, &result, &format!("{heuristic:?}"));
    }
}

#[test]
fn state_equation_a_star_matches_pm4py() {
    run(Heuristic::StateEquation);
}

#[test]
fn dijkstra_matches_pm4py() {
    run(Heuristic::None);
}

/// pm4py's `get_visible_transitions_eventually_enabled_by_marking` can miss
/// markings reached through silent transitions; ichnos explores them all.
/// The golden holds pm4py's precision with a complete search patched in
/// (`precision_complete_closure`), which ichnos must match, and pm4py's
/// own value, which differs only on `receipt` with its inductive net.
#[test]
fn precision_matches_pm4py() {
    let keys = EventKeys::default();
    for id in alignment_cases() {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let (net, im, fm) = build_net(g.expected_at("/model"));
        let precision = precision_alignments(&log, &net, &im, &fm, &keys).expect("precision");
        close(
            precision,
            f64_at(&g, "/precision_complete_closure"),
            Tolerance::METRIC,
            format_args!("{}: precision", g.case),
        );
        if id != "alignments-receipt-im" {
            close(
                precision,
                f64_at(&g, "/precision"),
                Tolerance::METRIC,
                format_args!("{}: pm4py's own precision", g.case),
            );
        }
    }
}
