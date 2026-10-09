//! Organizational mining against pm4py goldens (`fixtures/golden/org`).

mod common;

use common::load;
use ichnos_core::EventKeys;
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_org::*;
use serde_json::{Value, json};

const LOGS: [&str; 3] = ["running-example", "receipt", "reviewing"];

fn sna_json(sna: &Sna) -> Value {
    let rows: Vec<Value> = sna
        .connections
        .iter()
        .map(|((a, b), v)| json!({"from": a, "to": b, "value": float(*v)}))
        .collect();
    json!({"directed": sna.directed, "connections": rows})
}

/// A float as the goldens write it: NaN as a string.
fn float(v: f64) -> Value {
    if v.is_nan() { json!("NaN") } else { json!(v) }
}

fn check(case: &str, actual: &Value) {
    let expected: Value = golden("org", case).expected_as();
    assert_json_eq(actual, &expected, &JsonCompare::default());
}

#[test]
fn social_networks_match_pm4py() {
    let keys = EventKeys::default();
    for name in LOGS {
        let log = load(name);
        let sna = discover_handover_of_work_network(&log, 0.0, &keys).unwrap();
        check(&format!("handover-{name}"), &sna_json(&sna));
        let sna = discover_working_together_network(&log, &keys).unwrap();
        check(&format!("working-together-{name}"), &sna_json(&sna));
        if name != "receipt" {
            let sna = discover_activity_based_resource_similarity(&log, &keys).unwrap();
            check(&format!("similarity-{name}"), &sna_json(&sna));
        }
        let sna = discover_subcontracting_network(&log, 2, &keys).unwrap();
        check(&format!("subcontracting-{name}"), &sna_json(&sna));
    }
    for (name, beta) in [("running-example", 0.5), ("receipt", 1.0)] {
        let sna = discover_handover_of_work_network(&load(name), beta, &keys).unwrap();
        check(&format!("handover-{name}-beta"), &sna_json(&sna));
    }
    for name in ["running-example", "receipt"] {
        let sna = discover_subcontracting_network(&load(name), 3, &keys).unwrap();
        check(&format!("subcontracting-{name}-n3"), &sna_json(&sna));
    }
}

#[test]
fn roles_match_pm4py() {
    for name in LOGS {
        let roles = discover_organizational_roles(&load(name), &EventKeys::default()).unwrap();
        let actual: Vec<Value> = roles
            .iter()
            .map(|r| json!({"activities": r.activities, "originator_importance": r.originator_importance}))
            .collect();
        // Order matters: compare as a list.
        check(&format!("roles-{name}"), &json!(actual));
    }
}

fn network_rows<T>(na: &NetworkAnalysis<T>, value: impl Fn(&T) -> Value) -> Value {
    let rows: Vec<Value> = na
        .iter()
        .flat_map(|((a, b), edges)| {
            edges
                .iter()
                .map(move |(e, v)| (a, b, e, v))
                .collect::<Vec<_>>()
        })
        .map(|(a, b, e, v)| json!({"source": a, "target": b, "edge": e, "value": value(v)}))
        .collect();
    json!(rows)
}

fn sorted_seconds(v: &[f64]) -> Value {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    Value::Array(v.into_iter().map(float).collect())
}

#[test]
fn network_analyses_match_pm4py() {
    let default = NetworkAnalysisOptions::default();
    let running = load("running-example");
    let counts = |log, options: &NetworkAnalysisOptions| {
        network_rows(&discover_network_analysis(log, options), |c| json!(c))
    };
    check(
        "network-analysis-running-example",
        &counts(&running, &default),
    );
    check(
        "network-analysis-running-example-performance",
        &network_rows(
            &discover_network_analysis_performance(&running, &default),
            |v| sorted_seconds(v),
        ),
    );
    let target = NetworkAnalysisOptions {
        edge_reference: EdgeReference::Target,
        ..NetworkAnalysisOptions::default()
    };
    check(
        "network-analysis-running-example-target-edge",
        &counts(&running, &target),
    );
    let by_resource = NetworkAnalysisOptions {
        out_column: "org:resource".to_owned(),
        in_column: "org:resource".to_owned(),
        node_column_source: "concept:name".to_owned(),
        node_column_target: "concept:name".to_owned(),
        edge_column: "case:concept:name".to_owned(),
        ..NetworkAnalysisOptions::default()
    };
    check(
        "network-analysis-running-example-by-resource",
        &counts(&running, &by_resource),
    );
    check(
        "network-analysis-receipt",
        &counts(&load("receipt"), &default),
    );
}

#[test]
fn missing_resources_are_errors() {
    let mut log = load("running-example");
    log.traces[0].events[1].attributes = Default::default();
    let err = discover_handover_of_work_network(&log, 0.0, &EventKeys::default()).unwrap_err();
    assert!(err.to_string().contains("org:resource"), "{err}");
}

#[test]
fn subcontracting_needs_a_window() {
    let log = load("running-example");
    let keys = EventKeys::default();
    let err = discover_subcontracting_network(&log, 0, &keys).unwrap_err();
    assert!(matches!(err, Error::InvalidOption(_)), "{err}");
    // A window of one has no resource in between: an empty network.
    let sna = discover_subcontracting_network(&log, 1, &keys).unwrap();
    assert!(sna.connections.is_empty());
}
