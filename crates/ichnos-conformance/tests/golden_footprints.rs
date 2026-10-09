//! Footprint conformance and `check_is_fitting` against pm4py
//! (`fixtures/golden/conformance/footprints-*`).
//!
//! Each golden checks a log against a canonical net and, for the
//! inductive-miner cases, against the tree too. The tests compare the model
//! and log footprints, the deviations of the whole log and of each variant,
//! fitness, precision, the `log_model` violations and whether each variant
//! fits.

mod common;

use std::collections::BTreeSet;

use ichnos_conformance::footprints::{
    FootprintsDeviations, LogFootprints, ModelFootprints, conformance_diagnostics_footprints,
    conformance_diagnostics_footprints_log, fitness_footprints, fitness_footprints_log,
    footprint_violations, precision_footprints,
};
use ichnos_conformance::token_replay::{check_is_fitting, check_is_fitting_tree};
use ichnos_core::{EventKeys, EventLog};
use ichnos_golden::{Golden, Tolerance, as_f64, cases, compare_close, golden};
use ichnos_model::footprints::LabelPair;
use ichnos_model::{Label, ProcessTree};
use serde_json::Value;

use common::{build_net, load_csv_log};

fn close(actual: f64, expected: &Value, what: impl std::fmt::Display) {
    let expected = as_f64(expected).unwrap_or_else(|| panic!("{what}: not a number"));
    if let Err(m) = compare_close(actual, expected, Tolerance::METRIC) {
        panic!("{what}: {m}");
    }
}

fn footprint_cases() -> Vec<String> {
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with("footprints-"))
        .collect();
    assert_eq!(
        ids.len(),
        11,
        "expected 11 footprint goldens, found {ids:?}"
    );
    ids
}

fn labels(e: &Value) -> BTreeSet<Label> {
    e.as_array()
        .expect("labels")
        .iter()
        .map(|l| Label::new(l.as_str().expect("label")))
        .collect()
}

fn pairs(e: &Value) -> BTreeSet<LabelPair> {
    e.as_array()
        .expect("pairs")
        .iter()
        .map(|p| {
            let label = |i: usize| Label::new(p[i].as_str().expect("label"));
            (label(0), label(1))
        })
        .collect()
}

fn check_model(model: &ModelFootprints, e: &Value, what: &str) {
    assert_eq!(model.sequence, pairs(&e["sequence"]), "{what}: sequence");
    assert_eq!(model.parallel, pairs(&e["parallel"]), "{what}: parallel");
    assert_eq!(
        model.start_activities,
        labels(&e["start_activities"]),
        "{what}: start activities"
    );
    assert_eq!(
        model.end_activities,
        e.get("end_activities").map(labels),
        "{what}: end activities"
    );
    assert_eq!(
        model.activities_always_happening,
        e.get("activities_always_happening").map(labels),
        "{what}: activities always happening"
    );
    assert_eq!(
        model.min_trace_length,
        e.get("min_trace_length")
            .map(|n| n.as_u64().expect("length") as usize),
        "{what}: min trace length"
    );
}

fn check_log_footprints(fp: &LogFootprints, e: &Value, what: &str) {
    let dfg: Vec<(String, String, u64)> = fp
        .dfg
        .iter()
        .map(|((a, b), n)| (a.to_string(), b.to_string(), *n))
        .collect();
    let expected: Vec<(String, String, u64)> = e["dfg"]
        .as_array()
        .expect("dfg")
        .iter()
        .map(|t| {
            (
                t[0].as_str().expect("from").to_owned(),
                t[1].as_str().expect("to").to_owned(),
                t[2].as_u64().expect("count"),
            )
        })
        .collect();
    assert_eq!(dfg, expected, "{what}: dfg");
    assert_eq!(fp.sequence, pairs(&e["sequence"]), "{what}: sequence");
    assert_eq!(fp.parallel, pairs(&e["parallel"]), "{what}: parallel");
    assert_eq!(
        fp.activities,
        labels(&e["activities"]),
        "{what}: activities"
    );
    assert_eq!(
        fp.start_activities,
        labels(&e["start_activities"]),
        "{what}: start activities"
    );
    assert_eq!(
        fp.end_activities,
        labels(&e["end_activities"]),
        "{what}: end activities"
    );
    assert_eq!(
        fp.min_trace_length as u64,
        e["min_trace_length"].as_u64().expect("length"),
        "{what}: min trace length"
    );
}

fn check_deviations(d: &FootprintsDeviations, e: &Value, what: &str) {
    assert_eq!(d.footprints, pairs(&e["footprints"]), "{what}: footprints");
    assert_eq!(
        d.start_activities,
        labels(&e["start_activities"]),
        "{what}: start activities"
    );
    assert_eq!(
        d.end_activities,
        labels(&e["end_activities"]),
        "{what}: end activities"
    );
    let always = e
        .get("activities_always_happening")
        .map(labels)
        .unwrap_or_default();
    assert_eq!(
        d.activities_always_happening, always,
        "{what}: activities always happening"
    );
    assert_eq!(
        d.min_length_fit,
        e["min_length_fit"].as_bool().expect("min_length_fit"),
        "{what}: min length fit"
    );
    assert_eq!(
        d.is_footprints_fit,
        e["is_footprints_fit"].as_bool().expect("is_footprints_fit"),
        "{what}: is footprints fit"
    );
}

/// Checks one model kind of a golden. `fits` tells whether a variant fits.
fn check_model_kind(
    g: &Golden,
    log: &EventLog,
    model: &ModelFootprints,
    kind: &str,
    fits: impl Fn(&[&str]) -> bool,
) {
    let id = &g.case;
    let keys = EventKeys::default();
    let e = g.expected_at(&format!("/checks/{kind}"));
    let what = format!("{id} {kind}");
    check_model(model, &e["footprints"], &format!("{what}: model"));

    let log_fp = LogFootprints::of_log(log, &keys).expect("log footprints");
    let whole = conformance_diagnostics_footprints_log(log, &keys, model).expect("log check");
    check_deviations(&whole, &e["log"], &format!("{what}: log"));

    // Per variant, and per trace through its variant.
    let variants: Vec<Vec<&str>> = g
        .expected_at("/variants")
        .as_array()
        .expect("variants")
        .iter()
        .map(|v| {
            v.as_array()
                .expect("variant")
                .iter()
                .map(|a| a.as_str().expect("activity"))
                .collect()
        })
        .collect();
    let expected = e["variants"].as_array().expect("variant checks");
    assert_eq!(expected.len(), variants.len(), "{what}: variants");
    let per_trace = conformance_diagnostics_footprints(log, &keys, model).expect("trace check");
    let log_variants = log.variants(&keys).expect("variants");
    assert_eq!(log_variants.len(), variants.len(), "{what}: variants");
    for ((trace, ev), v) in variants.iter().zip(expected).zip(log_variants.iter()) {
        let vw = format!("{what}: variant {trace:?}");
        assert_eq!(
            log_variants.names(v).collect::<Vec<_>>(),
            *trace,
            "{vw}: order"
        );
        let d = FootprintsDeviations::of_trace(&LogFootprints::of_trace(trace), model);
        check_deviations(&d, ev, &vw);
        for &t in &v.traces {
            assert_eq!(per_trace[t], d, "{vw}: trace {t}");
        }
    }

    let fitness = fitness_footprints(log, &keys, model).expect("fitness");
    close(
        fitness.percentage_of_fitting_traces,
        &e["fitness"]["perc_fit_traces"],
        format_args!("{what}: percentage of fitting traces"),
    );
    close(
        fitness.log_fitness,
        &e["fitness"]["log_fitness"],
        format_args!("{what}: log fitness"),
    );
    close(
        fitness_footprints_log(log, &keys, model).expect("fitness"),
        &e["fitness_log"],
        format_args!("{what}: whole-log fitness"),
    );
    let precision = precision_footprints(log, &keys, model).expect("precision");
    close(
        precision,
        &e["precision"],
        format_args!("{what}: precision"),
    );
    close(
        precision,
        &e["precision_log"],
        format_args!("{what}: whole-log precision"),
    );

    for (name, strict) in [("loose", false), ("strict", true)] {
        assert_eq!(
            footprint_violations(&log_fp, model, strict),
            pairs(&e["violations"][name]),
            "{what}: {name} violations"
        );
    }

    let fitting = e["fitting"].as_array().expect("fitting");
    for (trace, expected) in variants.iter().zip(fitting) {
        assert_eq!(
            fits(trace),
            expected.as_bool().expect("fitting"),
            "{what}: check_is_fitting {trace:?}"
        );
    }
}

#[test]
fn footprints_match_pm4py() {
    for id in footprint_cases() {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        check_log_footprints(
            &LogFootprints::of_log(&log, &EventKeys::default()).expect("log footprints"),
            g.expected_at("/log_footprints"),
            &format!("{id}: log footprints"),
        );

        let (net, im, fm) = build_net(g.expected_at("/model"));
        let model = ModelFootprints::of_net(&net, &im).expect("net footprints");
        check_model_kind(&g, &log, &model, "net", |trace| {
            check_is_fitting(trace, &net, &im, &fm).expect("check_is_fitting")
        });

        if let Some(tree) = g.expected.get("tree") {
            let tree = ProcessTree::parse(tree.as_str().expect("tree")).expect("parse tree");
            let model = ModelFootprints::of_tree(&tree);
            check_model_kind(&g, &log, &model, "tree", |trace| {
                check_is_fitting_tree(trace, &tree).expect("check_is_fitting")
            });
        }
    }
}
