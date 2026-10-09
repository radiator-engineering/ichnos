//! Public pm4py oracle comparisons with lossless label-index encoding.

use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{
    DeclareActivities, DeclareCounts, DeclareModel, DeclareOptions, DeclareTemplate, LogSkeleton,
    LogSkeletonOptions, declare, log_skeleton,
};
use ichnos_golden::{Golden, golden};
use ichnos_model::Label;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn template(name: &str) -> DeclareTemplate {
    DeclareTemplate::ALL
        .into_iter()
        .find(|t| t.as_str() == name)
        .unwrap_or_else(|| panic!("unknown template {name}"))
}

fn declare_options(v: &Value) -> DeclareOptions {
    let mut options = DeclareOptions::default();
    if let Some(names) = v["allowed_templates"].as_array() {
        options.allowed_templates = names
            .iter()
            .map(|n| template(n.as_str().unwrap()))
            .collect();
    }
    options.considered_activities = v["considered_activities"].as_array().map(|names| {
        names
            .iter()
            .map(|n| Label::from(n.as_str().unwrap()))
            .collect()
    });
    options.min_support_ratio = v["min_support_ratio"].as_f64();
    options.min_confidence_ratio = v["min_confidence_ratio"].as_f64();
    if let Some(multiplier) = v["auto_selection_multiplier"].as_f64() {
        options.auto_selection_multiplier = multiplier;
    }
    options
}

fn load(g: &Golden) -> (EventLog, EventKeys) {
    let keys = EventKeys::default().with_activity(
        g.meta().params["activity_key"]
            .as_str()
            .unwrap_or("concept:name"),
    );
    let log = if let Some(traces) = g.meta().params["traces"].as_array() {
        let traces = traces
            .iter()
            .map(|row| {
                let mut trace = Trace::new();
                for label in row.as_array().unwrap() {
                    let mut event = Event::new();
                    event
                        .attributes
                        .insert(keys.activity.as_str(), label.as_str().unwrap());
                    trace.events.push(event);
                }
                trace
            })
            .collect();
        EventLog::from_traces(traces)
    } else {
        let path = g.fixture("log");
        if path.extension().unwrap() == "xes" {
            ichnos_io::read_xes(path, &Default::default()).unwrap()
        } else {
            ichnos_io::read_csv(path, &Default::default()).unwrap()
        }
    };
    (log, keys)
}

fn expected_skeleton(v: &Value, labels: &[Label]) -> LogSkeleton {
    let relations = |name| {
        v[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    labels[row[0].as_u64().unwrap() as usize].clone(),
                    labels[row[1].as_u64().unwrap() as usize].clone(),
                )
            })
            .collect()
    };
    LogSkeleton {
        equivalence: relations("equivalence"),
        always_after: relations("always_after"),
        always_before: relations("always_before"),
        never_together: relations("never_together"),
        directly_follows: relations("directly_follows"),
        activity_frequencies: v["activ_freq"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    labels[row[0].as_u64().unwrap() as usize].clone(),
                    row[1]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_u64().unwrap())
                        .collect(),
                )
            })
            .collect(),
    }
}

fn expected_declare(v: &Value, labels: &[Label]) -> DeclareModel {
    let rules = v
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, rows)| {
            let template = template(name);
            let rules = rows
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    let row = row.as_array().unwrap();
                    let activity = |i: usize| labels[row[i].as_u64().unwrap() as usize].clone();
                    let args = if template.is_unary() {
                        assert_eq!(row.len(), 3);
                        DeclareActivities::Unary(activity(0))
                    } else {
                        assert_eq!(row.len(), 4);
                        DeclareActivities::Binary(activity(0), activity(1))
                    };
                    let counts = DeclareCounts {
                        support: row[row.len() - 2].as_u64().unwrap(),
                        confidence: row[row.len() - 1].as_u64().unwrap(),
                    };
                    (args, counts)
                })
                .collect();
            (template, rules)
        })
        .collect();
    DeclareModel { rules }
}

fn check_case(id: &str) {
    let g = golden("discovery", &format!("skeleton-declare-{id}"));
    let (log, keys) = load(&g);
    let original = log.clone();
    let labels: Vec<_> = g.expected["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| Label::from(a.as_str().unwrap()))
        .collect();
    for v in g.expected["skeleton"].as_array().unwrap() {
        let noise_threshold = v["noise"].as_f64().unwrap();
        let actual = log_skeleton(&log, &keys, &LogSkeletonOptions { noise_threshold }).unwrap();
        assert_eq!(
            actual,
            expected_skeleton(&v["model"], &labels),
            "{id}: skeleton noise={noise_threshold}"
        );
    }
    for v in g.expected["declare"].as_array().unwrap() {
        let actual = declare(&log, &keys, &declare_options(&v["options"])).unwrap();
        assert_eq!(
            actual,
            expected_declare(&v["model"], &labels),
            "{id}: DECLARE {}",
            v["options"]
        );
        for (template, rules) in &actual.rules {
            assert!(!rules.is_empty());
            for (args, counts) in rules {
                assert_eq!(
                    template.is_unary(),
                    matches!(args, DeclareActivities::Unary(_))
                );
                assert!(
                    counts.confidence <= counts.support
                        && counts.support > 0
                        && counts.support <= log.len() as u64
                );
            }
        }
    }
    assert_eq!(log, original, "input mutated");
}

macro_rules! golden_case {
    ($name:ident,$id:literal) => {
        #[test]
        fn $name() {
            check_case($id);
        }
    };
}
golden_case!(running_example, "running-example-xes");
golden_case!(receipt, "receipt-xes");
golden_case!(roadtraffic, "roadtraffic100traces-xes");
golden_case!(interleavings_even, "interleavings-receipt_even-csv");
golden_case!(interleavings_odd, "interleavings-receipt_odd-csv");
golden_case!(empty, "empty");
golden_case!(empty_traces, "empty-traces");
golden_case!(repeated, "repeated");
golden_case!(weighted, "weighted");
golden_case!(projection, "projection");
golden_case!(frequency_ties, "frequency-ties");
golden_case!(frequency_ties_reversed, "frequency-ties-reversed");
golden_case!(custom_key, "custom-key");

#[test]
fn typed_errors_and_empty_label_arguments() {
    let keys = EventKeys::default();
    let mut log = EventLog::from_trace_strings(["a,b"], ",", &keys);
    log.traces[0].events[0].attributes.remove("concept:name");
    assert!(matches!(
        log_skeleton(&log, &keys, &Default::default()),
        Err(ichnos_discovery::Error::Core(_))
    ));
    assert!(matches!(
        declare(&log, &keys, &Default::default()),
        Err(ichnos_discovery::Error::Core(_))
    ));
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(matches!(
            log_skeleton(
                &EventLog::default(),
                &keys,
                &LogSkeletonOptions {
                    noise_threshold: value
                }
            ),
            Err(ichnos_discovery::Error::NoiseThreshold(_))
        ));
        let options = DeclareOptions {
            min_support_ratio: Some(value),
            ..Default::default()
        };
        assert!(matches!(
            declare(&EventLog::default(), &keys, &options),
            Err(ichnos_discovery::Error::DeclareThreshold {
                option: "min_support_ratio",
                ..
            })
        ));
    }
    let log = EventLog::from_trace_strings(["a,"], ",", &keys);
    let options = DeclareOptions {
        allowed_templates: BTreeSet::from([DeclareTemplate::Response]),
        min_support_ratio: Some(0.0),
        min_confidence_ratio: Some(0.0),
        ..Default::default()
    };
    let model = declare(&log, &keys, &options).unwrap();
    assert_eq!(
        model.rules[&DeclareTemplate::Response],
        BTreeMap::from([
            (
                DeclareActivities::Binary(Label::from("a"), Label::from("")),
                DeclareCounts {
                    support: 1,
                    confidence: 1
                }
            ),
            (
                DeclareActivities::Binary(Label::from(""), Label::from("a")),
                DeclareCounts {
                    support: 1,
                    confidence: 0
                }
            ),
        ])
    );
}
