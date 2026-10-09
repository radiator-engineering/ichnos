//! Approximate Petri net alignments against pm4py
//! (`fixtures/golden/conformance/alignments-approx-*`).
//!
//! The approximate methods break ties by transition label and name, and the
//! goldens use nets with canonical names, so the moves must match pm4py's
//! exactly, as must the costs, fitness and search counters.

mod common;

use ichnos_conformance::alignments::{
    ApproximateAligner, ApproximateAlignment, ApproximateOptions, Approximation,
    ApproximationReport, FixedHorizon, Move, SlidingWindow,
};
use ichnos_golden::{cases, golden};
use ichnos_model::PetriNet;
use serde_json::Value;

use common::build_net;

fn usize_at(v: &Value, key: &str) -> usize {
    v[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} is not a count: {v}")) as usize
}

fn opt_usize(v: &Value, key: &str) -> Option<usize> {
    v.get(key).map(|x| x.as_u64().expect("count") as usize)
}

/// The method of a run in the golden's `runs`, from pm4py's variant name
/// and parameters.
fn method(name: &str, params: &Value) -> Approximation {
    if name.starts_with("tandem_repeats") {
        Approximation::TandemRepeats
    } else if name.starts_with("sliding_window") {
        let d = SlidingWindow::default();
        Approximation::SlidingWindow(SlidingWindow {
            window_size: opt_usize(params, "window_size").unwrap_or(d.window_size),
            max_candidates: opt_usize(params, "max_candidates").unwrap_or(d.max_candidates),
            max_post_model_moves: opt_usize(params, "max_post_model_moves"),
        })
    } else if name.starts_with("fixed_horizon") {
        let d = FixedHorizon::default();
        Approximation::FixedHorizon(FixedHorizon {
            horizon: opt_usize(params, "horizon").unwrap_or(d.horizon),
            min_progress: opt_usize(params, "min_progress").unwrap_or(d.min_progress),
            max_horizon: opt_usize(params, "max_horizon"),
            max_prefix_states: opt_usize(params, "max_prefix_states")
                .unwrap_or(d.max_prefix_states),
            max_iterations: opt_usize(params, "max_iterations"),
        })
    } else {
        panic!("unknown run {name}")
    }
}

/// pm4py's moves: `[event index or null, transition name or null]`.
fn moves(net: &PetriNet, a: &ApproximateAlignment) -> Value {
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

fn check(net: &PetriNet, a: &ApproximateAlignment, e: &Value, what: &str) {
    assert_eq!(moves(net, a), e["moves"], "{what}: moves");
    let t = &a.alignment;
    assert_eq!(t.cost, e["cost"].as_u64().expect("cost"), "{what}: cost");
    assert_eq!(
        a.standard_cost,
        e["standard_cost"].as_u64().expect("standard cost"),
        "{what}: standard cost"
    );
    assert_eq!(
        t.best_worst_cost,
        e["bwc"].as_u64().expect("bwc"),
        "{what}: bwc"
    );
    assert_eq!(
        t.fitness,
        e["fitness"].as_f64().expect("fitness"),
        "{what}: fitness"
    );
    assert_eq!(
        a.is_valid,
        e["is_valid"].as_bool().expect("is_valid"),
        "{what}: is_valid"
    );
    assert_eq!(t.visited_states, usize_at(e, "visited"), "{what}: visited");
    assert_eq!(t.queued_states, usize_at(e, "queued"), "{what}: queued");
    assert_eq!(
        t.traversed_arcs,
        usize_at(e, "traversed"),
        "{what}: traversed"
    );
    match &a.report {
        ApproximationReport::TandemRepeats {
            reduced_trace_length,
            tandem_repeats,
            removed_events,
            model_loop_expansions,
        } => {
            assert_eq!(
                *reduced_trace_length,
                usize_at(e, "reduced_trace_length"),
                "{what}"
            );
            assert_eq!(*tandem_repeats, usize_at(e, "tandem_repeats"), "{what}");
            assert_eq!(*removed_events, usize_at(e, "removed_events"), "{what}");
            assert_eq!(
                *model_loop_expansions,
                usize_at(e, "model_loop_expansions"),
                "{what}"
            );
        }
        ApproximationReport::SlidingWindow {
            window_count,
            retained_candidates,
            fallback_used,
        } => {
            assert_eq!(*window_count, usize_at(e, "window_count"), "{what}");
            assert_eq!(
                serde_json::to_value(retained_candidates).expect("json"),
                e["retained_candidates"],
                "{what}: retained candidates"
            );
            assert_eq!(Some(*fallback_used), e["fallback_used"].as_bool(), "{what}");
        }
        ApproximationReport::FixedHorizon {
            committed_horizons,
            fallback,
        } => {
            assert_eq!(
                serde_json::to_value(committed_horizons).expect("json"),
                e["committed_horizons"],
                "{what}: committed horizons"
            );
            assert_eq!(t.lp_solved, usize_at(e, "lp_solved"), "{what}: LPs solved");
            assert_eq!(
                fallback.map(|f| f.as_str()),
                e["fallback_reason"].as_str(),
                "{what}: fallback"
            );
        }
    }
}

#[test]
fn approximate_alignments_match_pm4py() {
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with("alignments-approx-"))
        .collect();
    for id in &ids {
        let g = golden("conformance", id);
        let (net, im, fm) = build_net(&g.expected["model"]);
        let runs = g.expected["runs"].as_object().expect("runs");
        for (run, params) in runs {
            let options = ApproximateOptions::new(method(run, params));
            let aligner = ApproximateAligner::new(&net, &im, &fm, options).expect("aligner");
            for (v, e) in g.expected["variants"]
                .as_array()
                .expect("variants")
                .iter()
                .enumerate()
            {
                let trace: Vec<&str> = e["trace"]
                    .as_array()
                    .expect("trace")
                    .iter()
                    .map(|a| a.as_str().expect("activity"))
                    .collect();
                let what = format!("{id} {run} variant {v}");
                let result = aligner.align(&trace).expect("align");
                match (&result, &e[run.as_str()]) {
                    (None, Value::Null) => {}
                    (Some(a), expected @ Value::Object(_)) => check(&net, a, expected, &what),
                    (got, expected) => panic!("{what}: got {got:?}, expected {expected}"),
                }
            }
        }
    }
    assert_eq!(
        ids.len(),
        8,
        "expected 8 approximate alignment goldens, found {ids:?}"
    );
}

#[test]
fn moves_are_valid_runs() {
    let g = golden("conformance", "alignments-approx-running-example-im");
    let (net, im, fm) = build_net(&g.expected["model"]);
    let trace = [
        "register request",
        "decide",
        "decide",
        "decide",
        "pay compensation",
    ];
    for method in [
        Approximation::TandemRepeats,
        Approximation::SlidingWindow(SlidingWindow {
            window_size: 2,
            ..SlidingWindow::default()
        }),
        Approximation::FixedHorizon(FixedHorizon::default()),
    ] {
        let aligner = ApproximateAligner::new(&net, &im, &fm, ApproximateOptions::new(method))
            .expect("aligner");
        let a = aligner.align(&trace).expect("align").expect("no limits");
        assert!(a.is_valid);
        let events: Vec<usize> = a.alignment.moves.iter().filter_map(Move::event).collect();
        assert_eq!(events, (0..trace.len()).collect::<Vec<_>>());
    }
}
