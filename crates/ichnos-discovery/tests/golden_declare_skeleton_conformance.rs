//! DECLARE and log-skeleton conformance against the pm4py oracle. The
//! goldens live in the conformance area; see `tools/golden/cases/conformance.py`.

use std::collections::{BTreeMap, HashMap};

use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{
    DeclareActivities, DeclareCounts, DeclareModel, DeclareTemplate, LogSkeleton,
    SkeletonConformanceOptions, SkeletonConstraint, SkeletonDeviation, SkeletonRelation,
    conformance_declare, conformance_log_skeleton,
};
use ichnos_golden::{Golden, golden};
use ichnos_model::Label;
use serde_json::Value;

fn load(g: &Golden) -> EventLog {
    match g.meta().params["traces"].as_array() {
        Some(traces) => EventLog::from_traces(
            traces
                .iter()
                .map(|row| {
                    let mut trace = Trace::new();
                    for a in row.as_array().unwrap() {
                        let mut event = Event::new();
                        event.attributes.insert("concept:name", a.as_str().unwrap());
                        trace.events.push(event);
                    }
                    trace
                })
                .collect(),
        ),
        None => ichnos_io::read_xes(g.fixture("log"), &Default::default()).unwrap(),
    }
}

fn labels(g: &Golden) -> Vec<Label> {
    g.expected["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| Label::from(a.as_str().unwrap()))
        .collect()
}

fn ints(v: &Value) -> Vec<usize> {
    v.as_str()
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect()
}

fn template(name: &str) -> DeclareTemplate {
    DeclareTemplate::ALL
        .into_iter()
        .find(|t| t.as_str() == name)
        .unwrap_or_else(|| panic!("unknown template {name}"))
}

/// Checks the counts every result shares; the caller checks the deviations.
fn check_counts(
    id: &str,
    trace: usize,
    expected: &Value,
    (no_dev_total, no_constr_total, dev_fitness, is_fit): (usize, usize, f64, bool),
) {
    assert_eq!(
        no_dev_total as u64,
        expected["no_dev_total"].as_u64().unwrap(),
        "{id}: trace {trace}"
    );
    assert_eq!(
        no_constr_total as u64,
        expected["no_constr_total"].as_u64().unwrap(),
        "{id}: trace {trace}"
    );
    // serde_json can read a float one unit in the last place off.
    let wanted = expected["dev_fitness"].as_f64().unwrap();
    assert!(
        (dev_fitness - wanted).abs() < 1e-12,
        "{id}: trace {trace}: {dev_fitness} != {wanted}"
    );
    assert_eq!(
        is_fit,
        expected["is_fit"].as_bool().unwrap(),
        "{id}: trace {trace}"
    );
}

fn check_declare(id: &str) {
    let g = golden("conformance", &format!("declare-{id}"));
    let log = load(&g);
    let labels = labels(&g);
    for check in g.expected["checks"].as_array().unwrap() {
        let rules: Vec<(DeclareTemplate, DeclareActivities)> = check["model"]
            .as_array()
            .unwrap()
            .iter()
            .map(|rule| {
                let mut parts = rule.as_str().unwrap().split(' ');
                let template = template(parts.next().unwrap());
                let acts: Vec<Label> = parts
                    .map(|i| labels[i.parse::<usize>().unwrap()].clone())
                    .collect();
                let args = match acts.as_slice() {
                    [a] => DeclareActivities::Unary(a.clone()),
                    [a, b] => DeclareActivities::Binary(a.clone(), b.clone()),
                    _ => panic!("bad rule {rule}"),
                };
                (template, args)
            })
            .collect();
        let index: HashMap<_, usize> = rules.iter().cloned().zip(0..).collect();
        let mut model = DeclareModel::default();
        for (template, args) in &rules {
            model
                .rules
                .entry(*template)
                .or_default()
                .insert(args.clone(), DeclareCounts::default());
        }
        let actual = conformance_declare(&log, &model, &EventKeys::default()).unwrap();
        let per_trace = ints(&check["per_trace"]);
        let results = check["results"].as_array().unwrap();
        assert_eq!(actual.len(), per_trace.len(), "{id}");
        for (t, (r, &k)) in actual.iter().zip(&per_trace).enumerate() {
            let expected = &results[k];
            let deviations: Vec<usize> = r
                .deviations
                .iter()
                .map(|d| index[&(d.template, d.activities.clone())])
                .collect();
            assert_eq!(
                deviations,
                ints(&expected["deviations"]),
                "{id}: trace {t} {check:?}"
            );
            check_counts(
                id,
                t,
                expected,
                (r.no_dev_total, r.no_constr_total, r.dev_fitness, r.is_fit),
            );
        }
    }
}

fn constraint(name: &str) -> SkeletonConstraint {
    SkeletonConstraint::ALL
        .into_iter()
        .find(|c| c.as_str() == name)
        .unwrap_or_else(|| panic!("unknown constraint {name}"))
}

fn check_skeleton(id: &str) {
    let g = golden("conformance", &format!("log-skeleton-{id}"));
    let log = load(&g);
    let labels = labels(&g);
    let position: BTreeMap<&Label, usize> = labels.iter().zip(0..).collect();
    let label = |i: &str| labels[i.parse::<usize>().unwrap()].clone();
    for check in g.expected["checks"].as_array().unwrap() {
        let m = &check["model"];
        let relation = |name: &str| -> SkeletonRelation {
            m[name]
                .as_str()
                .unwrap()
                .split_whitespace()
                .map(|p| {
                    let (a, b) = p.split_once('>').unwrap();
                    (label(a), label(b))
                })
                .collect()
        };
        let model = LogSkeleton {
            equivalence: relation("equivalence"),
            always_after: relation("always_after"),
            always_before: relation("always_before"),
            never_together: relation("never_together"),
            directly_follows: relation("directly_follows"),
            activity_frequencies: m["activ_freq"]
                .as_str()
                .unwrap()
                .split_whitespace()
                .map(|e| {
                    let (a, counts) = e.split_once(':').unwrap();
                    (
                        label(a),
                        counts.split(',').map(|n| n.parse().unwrap()).collect(),
                    )
                })
                .collect(),
        };
        let mut options = SkeletonConformanceOptions::default();
        if let Some(names) = check["considered"].as_array() {
            options.considered_constraints = names
                .iter()
                .map(|n| constraint(n.as_str().unwrap()))
                .collect();
        }
        let actual =
            conformance_log_skeleton(&log, &model, &EventKeys::default(), &options).unwrap();
        let per_trace = ints(&check["per_trace"]);
        let results = check["results"].as_array().unwrap();
        assert_eq!(actual.len(), per_trace.len(), "{id}");
        for (t, (r, &k)) in actual.iter().zip(&per_trace).enumerate() {
            let expected = &results[k];
            let deviations: Vec<String> = r
                .deviations
                .iter()
                .map(|d| match d {
                    SkeletonDeviation::ActivityFrequency { activity, count } => {
                        format!("activ_freq {}:{count}", position[activity])
                    }
                    SkeletonDeviation::Relation { constraint, pairs } => {
                        let pairs: Vec<String> = pairs
                            .iter()
                            .map(|(a, b)| format!("{}>{}", position[a], position[b]))
                            .collect();
                        format!("{} {}", constraint.as_str(), pairs.join(" "))
                    }
                })
                .collect();
            let wanted: Vec<&str> = expected["deviations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d.as_str().unwrap())
                .collect();
            assert_eq!(deviations, wanted, "{id}: trace {t} {check:?}");
            check_counts(
                id,
                t,
                expected,
                (r.no_dev_total, r.no_constr_total, r.dev_fitness, r.is_fit),
            );
        }
    }
}

macro_rules! cases {
    ($check:ident: $($name:ident => $id:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                $check($id);
            }
        )*
    };
}

cases!(check_declare:
    declare_running_example => "running-example",
    declare_receipt => "receipt",
    declare_reviewing => "reviewing",
    declare_synthetic => "synthetic",
);
cases!(check_skeleton:
    log_skeleton_running_example => "running-example",
    log_skeleton_receipt => "receipt",
    log_skeleton_reviewing => "reviewing",
    log_skeleton_synthetic => "synthetic",
);

#[test]
fn declare_rejects_a_rule_of_the_wrong_arity() {
    let mut model = DeclareModel::default();
    model
        .rules
        .entry(DeclareTemplate::Response)
        .or_default()
        .insert(
            DeclareActivities::Unary(Label::from("a")),
            DeclareCounts::default(),
        );
    let log = EventLog::from_traces(vec![Trace::new()]);
    assert!(conformance_declare(&log, &model, &EventKeys::default()).is_err());
}
