//! Petri net analysis against pm4py's (`fixtures/golden/analysis`).
//!
//! See `tools/golden/cases/analysis.py` for the cases.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_golden::{cases, golden};
use ichnos_model::analysis::{SimplicityVariant, SynchronousProduct};
use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::{AcceptingPetriNet, Label, Marking, PetriNet};
use serde_json::{Value, json};

mod common;
use common::{build_accepting, build_net};

fn node_name(net: &PetriNet, ends: ArcEnds) -> (String, String) {
    match ends {
        ArcEnds::PlaceToTransition(p, t) => {
            (net.place(p).name.clone(), net.transition(t).name.clone())
        }
        ArcEnds::TransitionToPlace(t, p) => {
            (net.transition(t).name.clone(), net.place(p).name.clone())
        }
    }
}

fn marking(net: &PetriNet, m: &Marking) -> BTreeMap<String, u32> {
    m.iter()
        .map(|(p, n)| (net.place(p).name.clone(), n))
        .collect()
}

/// The golden form of a net (`describe_net` in `tools/golden/cases/model.py`),
/// with arcs sorted fully so that parallel arcs compare in any order.
fn describe(apn: &AcceptingPetriNet) -> Value {
    let net = &apn.net;
    let mut places: Vec<&str> = net.places().map(|(_, p)| p.name.as_str()).collect();
    places.sort_unstable();
    let mut transitions: Vec<(&str, Option<&str>)> = net
        .transitions()
        .map(|(_, t)| (t.name.as_str(), t.label.as_ref().map(Label::as_str)))
        .collect();
    transitions.sort_unstable();
    let mut arcs: Vec<Value> = net
        .arcs()
        .map(|(_, a)| {
            let (source, target) = node_name(net, a.ends);
            let kind = match a.kind {
                ArcKind::Normal => "normal",
                ArcKind::Inhibitor => "inhibitor",
                ArcKind::Reset => "reset",
            };
            json!({"source": source, "target": target, "weight": a.weight, "type": kind})
        })
        .collect();
    arcs.sort_by_key(Value::to_string);
    json!({
        "kind": "petri_net",
        "places": places,
        "transitions": transitions
            .iter()
            .map(|(name, label)| json!({"name": name, "label": label}))
            .collect::<Vec<_>>(),
        "arcs": arcs,
        "initial_marking": marking(net, &apn.initial_marking),
        "final_marking": marking(net, &apn.final_marking),
    })
}

/// A golden net description with its arcs sorted as [`describe`] sorts them.
fn normalised(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(arcs) = v["arcs"].as_array_mut() {
        arcs.sort_by_key(Value::to_string);
    }
    v
}

/// The paths where two JSON values differ, at most a few.
fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if out.len() >= 6 || a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for k in keys {
                let null = Value::Null;
                diff(
                    x.get(k).unwrap_or(&null),
                    y.get(k).unwrap_or(&null),
                    &format!("{path}/{k}"),
                    out,
                );
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (x, y)) in x.iter().zip(y).enumerate() {
                diff(x, y, &format!("{path}/{i}"), out);
            }
        }
        _ => out.push(format!("{path}: ours {a} pm4py {b}")),
    }
}

/// Records a mismatch between `actual` and `expected`.
fn check(failures: &mut Vec<String>, what: &str, actual: &Value, expected: &Value) {
    if actual != expected {
        let mut d = Vec::new();
        diff(actual, expected, "", &mut d);
        failures.push(format!("{what}: {}", d.join("; ")));
    }
}

fn report(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Known differences from pm4py, as `(case and output, our value, pm4py's
/// value)`. `docs/parity.md` lists them as behaviour changes.
///
/// - pm4py truncates GLPK's optimum 3.9999999999963 to 3; ichnos rounds.
const KNOWN: &[(&str, &str, &str)] = &[
    (
        "sync-running-example-3 /extended_marking_equation",
        "4",
        "3",
    ),
    (
        "sync-running-example-3 /extended_marking_equation_split",
        "4",
        "3",
    ),
];

/// Checks a value, allowing a [`KNOWN`] difference.
fn check_known(failures: &mut Vec<String>, what: &str, actual: &Value, expected: &Value) {
    match KNOWN.iter().find(|(w, ..)| *w == what) {
        Some(&(_, ours, theirs)) => {
            if (actual.to_string(), expected.to_string()) != (ours.to_owned(), theirs.to_owned()) {
                failures.push(format!(
                    "{what}: expected the known difference ours {ours} pm4py {theirs}, got ours {actual} pm4py {expected}"
                ));
            }
        }
        None => check(failures, what, actual, expected),
    }
}

/// The soundness messages with the place list of the s-component coverage
/// message left out. The uniform invariants come from linear programs with
/// many optimal vertices, and pm4py's solver (HiGHS through scipy) and
/// ichnos's (`microlp`) pick different ones, so on `big_wf_net` the two
/// list different uncovered places. Both find some.
fn without_uncovered_places(messages: &Value) -> Value {
    const PREFIX: &str = "The following places are not covered by an s-component:";
    messages
        .as_array()
        .expect("messages")
        .iter()
        .map(|m| match m.as_str() {
            Some(m) if m.starts_with(PREFIX) => json!(PREFIX),
            _ => m.clone(),
        })
        .collect()
}

fn net_cases() -> Vec<String> {
    let ids: Vec<String> = cases("analysis")
        .into_iter()
        .filter(|id| id.starts_with("net-"))
        .collect();
    assert_eq!(ids.len(), 22, "expected 22 net goldens, found {ids:?}");
    ids
}

#[test]
fn workflow_net_checks_match_pm4py() {
    let mut failures = Vec::new();
    for id in net_cases() {
        let g = golden("analysis", &id);
        let apn = build_accepting(g.expected_at("/model"));
        let actual = json!(apn.net.is_workflow_net());
        check(&mut failures, &id, &actual, g.expected_at("/workflow_net"));
    }
    report(&failures);
}

#[test]
fn soundness_matches_pm4py() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for id in net_cases() {
        let g = golden("analysis", &id);
        let expected = g.expected_at("/soundness");
        if expected.is_null() {
            continue;
        }
        let apn = build_accepting(g.expected_at("/model"));
        let report = apn.check_soundness().expect("woflan");
        let mut actual = json!({"sound": report.sound, "messages": report.messages});
        let mut expected = expected.clone();
        if id == "net-big-wf-net" {
            actual["messages"] = without_uncovered_places(&actual["messages"]);
            expected["messages"] = without_uncovered_places(&expected["messages"]);
        }
        check(&mut failures, &id, &actual, &expected);
        let actual = json!(apn.is_sound().expect("woflan"));
        check_known(
            &mut failures,
            &format!("{id} is_sound"),
            &actual,
            g.expected_at("/is_sound"),
        );
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} soundness goldens");
    report(&failures);
}

#[test]
fn simplicity_matches_pm4py() {
    let mut failures = Vec::new();
    for id in net_cases() {
        let g = golden("analysis", &id);
        let apn = build_accepting(g.expected_at("/model"));
        for (key, variant) in [
            ("arc_degree", SimplicityVariant::ArcDegree),
            ("extended_cardoso", SimplicityVariant::ExtendedCardoso),
            ("extended_cyclomatic", SimplicityVariant::ExtendedCyclomatic),
        ] {
            let Some(expected) = g.expected_at(&format!("/simplicity/{key}")).as_f64() else {
                continue;
            };
            let actual = apn.simplicity(variant).expect("simplicity");
            if (actual - expected).abs() >= 1e-9 {
                failures.push(format!("{id} {key}: ours {actual} pm4py {expected}"));
            }
        }
    }
    report(&failures);
}

#[test]
fn maximal_decompositions_match_pm4py() {
    let mut failures = Vec::new();
    for id in net_cases() {
        let g = golden("analysis", &id);
        let apn = build_accepting(g.expected_at("/model"));
        let mut expected: Vec<Value> = g
            .expected_at("/decomposition")
            .as_array()
            .expect("components")
            .iter()
            .map(normalised)
            .collect();
        expected.sort_by_key(Value::to_string);
        // The golden iterates transitions in name order, so pm4py keeps the
        // last by name among transitions that share a label, as ichnos does.
        let mut actual: Vec<Value> = apn.maximal_decomposition().iter().map(describe).collect();
        actual.sort_by_key(Value::to_string);
        check(&mut failures, &id, &json!(actual), &json!(expected));
    }
    report(&failures);
}

#[test]
fn implicit_place_reduction_matches_pm4py() {
    let mut failures = Vec::new();
    for id in net_cases() {
        let g = golden("analysis", &id);
        let mut apn = build_accepting(g.expected_at("/model"));
        apn.reduce_implicit_places().expect("murata");
        let expected = normalised(g.expected_at("/implicit_places"));
        check(&mut failures, &id, &describe(&apn), &expected);
    }
    report(&failures);
}

#[test]
fn invisible_reduction_and_enabled_transitions_match_pm4py() {
    let mut failures = Vec::new();
    for id in net_cases() {
        let g = golden("analysis", &id);
        let mut apn = build_accepting(g.expected_at("/model"));
        let mut enabled: Vec<&str> = apn
            .net
            .enabled_transitions(&apn.initial_marking)
            .into_iter()
            .map(|t| apn.net.transition(t).name.as_str())
            .collect();
        enabled.sort_unstable();
        let enabled = json!(enabled);
        check(
            &mut failures,
            &format!("{id} enabled"),
            &enabled,
            g.expected_at("/enabled"),
        );
        apn.net.apply_simple_reduction();
        let expected = normalised(g.expected_at("/invisibles"));
        check(
            &mut failures,
            &format!("{id} invisibles"),
            &describe(&apn),
            &expected,
        );
    }
    report(&failures);
}

#[test]
fn marking_equation_matches_pm4py() {
    let mut failures = Vec::new();
    for id in net_cases() {
        let g = golden("analysis", &id);
        let apn = build_accepting(g.expected_at("/model"));
        let h = apn.solve_marking_equation(|_| 1.0).expect("lp");
        check(
            &mut failures,
            &id,
            &json!(h),
            g.expected_at("/marking_equation"),
        );
    }
    report(&failures);
}

#[test]
fn synchronous_products_match_pm4py() {
    let ids: Vec<String> = cases("analysis")
        .into_iter()
        .filter(|id| id.starts_with("sync-"))
        .collect();
    assert_eq!(ids.len(), 9, "expected 9 sync goldens, found {ids:?}");
    let mut failures = Vec::new();
    for id in ids {
        let g = golden("analysis", &id);
        let model = build_accepting(g.expected_at("/model"));
        let trace: Vec<Label> = g
            .expected_at("/trace")
            .as_array()
            .expect("trace")
            .iter()
            .map(|a| Label::from(a.as_str().expect("activity")))
            .collect();
        let sp = SynchronousProduct::new(&trace, &model);
        let mut actual = describe(&sp.net);
        // pm4py's `describe_sync` leaves out the arc type and the kind.
        let obj = actual.as_object_mut().expect("object");
        obj.remove("kind");
        let arcs = obj["arcs"].as_array_mut().expect("arcs");
        for arc in arcs.iter_mut() {
            arc.as_object_mut().expect("arc").remove("type");
        }
        arcs.sort_by_key(Value::to_string);
        check(
            &mut failures,
            &id,
            &actual,
            &normalised(g.expected_at("/sync_net")),
        );

        for (key, split) in [
            ("/extended_marking_equation", None),
            ("/extended_marking_equation_split", Some(&[1][..])),
        ] {
            let h = sp.solve_extended_marking_equation(split).expect("ilp");
            check_known(
                &mut failures,
                &format!("{id} {key}"),
                &json!(h),
                g.expected_at(key),
            );
        }
    }
    report(&failures);
}

#[test]
fn generated_markings_match_pm4py() {
    let g = golden("analysis", "generate-marking");
    let (net, _) = build_net(g.expected_at("/model"));
    let place = g.expected_at("/single/place").as_str().expect("place");
    let single = net.marking_from_names([(place, 1)]).expect("marking");
    assert_eq!(
        json!(marking(&net, &single)),
        *g.expected_at("/single/marking")
    );
    let counts: Vec<(&str, u32)> = g
        .expected_at("/counts/places")
        .as_object()
        .expect("counts")
        .iter()
        .map(|(p, n)| {
            (
                p.as_str(),
                u32::try_from(n.as_u64().expect("count")).expect("fits"),
            )
        })
        .collect();
    let m = net.marking_from_names(counts).expect("marking");
    assert_eq!(json!(marking(&net, &m)), *g.expected_at("/counts/marking"));
    assert!(net.marking_from_names([("no such place", 1)]).is_err());
}
