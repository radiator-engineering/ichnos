//! Token replay against pm4py (`fixtures/golden/conformance/token-replay-*`).
//!
//! The goldens replay on nets with canonical node names, so pm4py's name
//! order and ichnos's agree. The tests compare token counts, fitness and the
//! fired transitions exactly, for pm4py's default options and for each
//! non-default option set the golden lists. They also check ETConformance
//! precision, generalization and the markings `replay_prefix_tbr` reaches.

mod common;

use std::collections::{BTreeMap, HashMap};

use ichnos_conformance::generalization::{generalization, generalization_tbr};
use ichnos_conformance::token_replay::{
    TokenReplayOptions, TokenReplayer, TraceReplay, fitness_token_based_replay,
    precision_token_based_replay, replay_prefix_tbr,
};
use ichnos_core::EventKeys;
use ichnos_golden::{Golden, Tolerance, as_f64, cases, compare_close, golden};
use ichnos_model::{Marking, PetriNet};
use serde_json::Value;

use common::{build_net, load_csv_log};

fn close(actual: f64, expected: f64, tol: Tolerance, what: impl std::fmt::Display) {
    if let Err(m) = compare_close(actual, expected, tol) {
        panic!("{what}: {m}");
    }
}

fn token_replay_cases() -> Vec<String> {
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with("token-replay-"))
        .collect();
    assert_eq!(
        ids.len(),
        8,
        "expected 8 token replay goldens, found {ids:?}"
    );
    ids
}

fn names(net: &PetriNet, ts: impl IntoIterator<Item = ichnos_model::TransitionId>) -> String {
    ts.into_iter()
        .map(|t| net.transition(t).name.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn options_from(params: &Value) -> TokenReplayOptions {
    let mut o = TokenReplayOptions::default();
    for (key, value) in params.as_object().expect("params") {
        let on = value.as_bool().expect("boolean parameter");
        o = match key.as_str() {
            "consider_remaining_in_fitness" => o.consider_remaining_in_fitness(on),
            "try_to_reach_final_marking_through_hidden" => {
                o.try_to_reach_final_marking_through_hidden(on)
            }
            "stop_immediately_unfit" => o.stop_immediately_unfit(on),
            "walk_through_hidden_trans" => o.walk_through_hidden_transitions(on),
            "exhaustive_invisible_exploration" => o.exhaustive_invisible_exploration(on),
            "cleaning_token_flood" => o.cleaning_token_flood(on),
            "consider_activities_not_in_model_in_fitness" => {
                o.consider_activities_not_in_model_in_fitness(on)
            }
            other => panic!("unknown token replay parameter {other}"),
        };
    }
    o
}

/// Compares the counts, fitness and fit flag of `r` with a golden record.
fn check_counts(r: &TraceReplay, e: &Value, what: &str) {
    let count = |key: &str| e[key].as_u64().unwrap_or_else(|| panic!("{what}: {key}"));
    assert_eq!(r.missing_tokens, count("missing"), "{what}: missing");
    assert_eq!(r.consumed_tokens, count("consumed"), "{what}: consumed");
    assert_eq!(r.remaining_tokens, count("remaining"), "{what}: remaining");
    assert_eq!(r.produced_tokens, count("produced"), "{what}: produced");
    assert_eq!(
        r.is_fit,
        e["is_fit"].as_bool().expect("is_fit"),
        "{what}: is_fit"
    );
    close(
        r.fitness,
        as_f64(&e["fitness"]).expect("fitness"),
        Tolerance::EXACT,
        format_args!("{what}: fitness"),
    );
}

fn marking_names(net: &PetriNet, m: &Marking) -> BTreeMap<String, u64> {
    m.iter()
        .map(|(p, n)| (net.place(p).name.clone(), u64::from(n)))
        .collect()
}

fn expected_marking(e: &Value) -> BTreeMap<String, u64> {
    e.as_object()
        .expect("marking")
        .iter()
        .map(|(p, n)| (p.clone(), n.as_u64().expect("tokens")))
        .collect()
}

fn activities(e: &Value) -> Vec<&str> {
    e["activities"]
        .as_array()
        .expect("activities")
        .iter()
        .map(|a| a.as_str().expect("activity"))
        .collect()
}

fn f64_at(g: &Golden, pointer: &str) -> f64 {
    as_f64(g.expected_at(pointer))
        .unwrap_or_else(|| panic!("{}: {pointer} is not a number", g.case))
}

#[test]
fn token_replay_matches_pm4py() {
    let keys = EventKeys::default();
    for id in token_replay_cases() {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let (net, im, fm) = build_net(g.expected_at("/model"));
        let replayer =
            TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).expect("replayer");
        let result = replayer.replay_log(&log, &keys).expect("replay");

        // Per-trace diagnostics, in log order.
        let expected = g.expected_at("/traces").as_array().expect("traces");
        assert_eq!(result.trace_count(), expected.len(), "{id}: trace count");
        let index: HashMap<String, usize> = log
            .traces
            .iter()
            .enumerate()
            .map(|(i, t)| (t.case_id().expect("case id").to_string(), i))
            .collect();
        for e in expected {
            let case_id = e["case_id"].as_str().expect("case id");
            check_counts(
                result.trace(index[case_id]),
                e,
                &format!("{id}: case {case_id}"),
            );
        }

        // Full diagnostics per variant.
        for e in g.expected_at("/variants").as_array().expect("variants") {
            let trace = activities(e);
            let what = format!("{id}: variant {trace:?}");
            let r = replayer.replay(&trace).expect("replay");
            check_counts(&r, e, &what);
            assert_eq!(
                names(&net, r.activated_transitions.iter().copied()),
                e["activated"].as_str().expect("activated"),
                "{what}: activated transitions"
            );
            assert_eq!(
                names(&net, r.transitions_with_problems.iter().copied()),
                e["problems"].as_str().expect("problems"),
                "{what}: transitions with problems"
            );
            assert_eq!(
                marking_names(&net, &r.reached_marking),
                expected_marking(&e["reached"]),
                "{what}: reached marking"
            );
            let mut enabled: Vec<&str> = r
                .enabled_transitions_in_marking
                .iter()
                .map(|&t| net.transition(t).name.as_str())
                .collect();
            enabled.sort_unstable();
            assert_eq!(
                enabled.join(" "),
                e["enabled"].as_str().expect("enabled"),
                "{what}: enabled transitions"
            );
        }

        // Log fitness.
        let fitness = fitness_token_based_replay(&log, &net, &im, &fm, &keys).expect("fitness");
        assert_eq!(fitness, result.fitness());
        for (field, actual) in [
            ("log_fitness", fitness.log_fitness),
            ("average_trace_fitness", fitness.average_trace_fitness),
            (
                "percentage_of_fitting_traces",
                fitness.percentage_of_fitting_traces,
            ),
        ] {
            close(
                actual,
                f64_at(&g, &format!("/fitness/{field}")),
                Tolerance::METRIC,
                format_args!("{id}: {field}"),
            );
        }
    }
}

#[test]
fn precision_and_generalization_match_pm4py() {
    let keys = EventKeys::default();
    for id in token_replay_cases() {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let (net, im, fm) = build_net(g.expected_at("/model"));
        let precision =
            precision_token_based_replay(&log, &net, &im, &fm, &keys).expect("precision");
        close(
            precision,
            f64_at(&g, "/precision"),
            Tolerance::METRIC,
            format_args!("{id}: precision"),
        );
        let general = generalization_tbr(&log, &net, &im, &fm, &keys).expect("generalization");
        close(
            general,
            f64_at(&g, "/generalization"),
            Tolerance::METRIC,
            format_args!("{id}: generalization"),
        );
        let replayer =
            TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).expect("replayer");
        let replay = replayer.replay_log(&log, &keys).expect("replay");
        assert_eq!(generalization(&net, &replay), general, "{id}");
    }
}

#[test]
fn replay_prefix_matches_pm4py() {
    for id in token_replay_cases() {
        let g = golden("conformance", &id);
        let (net, im, fm) = build_net(g.expected_at("/model"));
        for e in g.expected_at("/prefixes").as_array().expect("prefixes") {
            let prefix: Vec<&str> = e["prefix"]
                .as_array()
                .expect("prefix")
                .iter()
                .map(|a| a.as_str().expect("activity"))
                .collect();
            let reached = replay_prefix_tbr(&prefix, &net, &im, &fm).expect("prefix replay");
            assert_eq!(
                marking_names(&net, &reached),
                expected_marking(&e["reached"]),
                "{id}: prefix {prefix:?}"
            );
        }
    }
}

#[test]
fn token_replay_options_match_pm4py() {
    for id in token_replay_cases() {
        let g = golden("conformance", &id);
        let (net, im, fm) = build_net(g.expected_at("/model"));
        let variants = g.expected_at("/variants").as_array().expect("variants");
        for (name, set) in g.expected_at("/options").as_object().expect("options") {
            let options = options_from(&set["params"]);
            let replayer = TokenReplayer::new(&net, &im, &fm, options).expect("replayer");
            let expected = set["variants"].as_array().expect("variants");
            assert_eq!(expected.len(), variants.len(), "{id} {name}: variants");
            for (v, e) in variants.iter().zip(expected) {
                let trace = activities(v);
                let what = format!("{id} {name}: variant {trace:?}");
                let r = replayer.replay(&trace).expect("replay");
                check_counts(&r, e, &what);
                assert_eq!(
                    names(&net, r.activated_transitions.iter().copied()),
                    e["activated"].as_str().expect("activated"),
                    "{what}: activated transitions"
                );
            }
        }
    }
}
