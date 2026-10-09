//! Compares `powl_inductive` with pm4py's `discover_powl`
//! (`fixtures/golden/discovery/powl-*.json`).
//!
//! The models are compared in a canonical form, as the oracle emits them:
//! XOR children sorted, every order closed under transitivity (ichnos'
//! `simplify` closes orders; pm4py's keeps the pairs it was given), and
//! partial-order children sorted by their own form and the forms of their
//! predecessors and successors. The child order of a cut depends on
//! Python's set order in pm4py, and none of these changes alter the
//! language.

// Only the CSV reader is used here.
#[allow(dead_code)]
mod common;

use std::collections::BTreeSet;

use common::load_csv_log;
use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{Error, PowlOptions, PowlVariant, powl_inductive};
use ichnos_golden::{cases, golden};
use ichnos_model::Powl;
use serde_json::{Value, json};

fn describe(p: &Powl) -> Value {
    let children = |c: &[Powl]| c.iter().map(describe).collect::<Vec<_>>();
    match p {
        Powl::Silent => json!({"kind": "silent"}),
        Powl::Activity(l) => json!({"kind": "activity", "label": l.as_str()}),
        Powl::Frequent(_) => panic!("discovery builds no frequent transitions"),
        Powl::Xor(c) => json!({"kind": "xor", "children": children(c)}),
        Powl::Loop(c) => json!({"kind": "loop", "children": children(&c[..])}),
        Powl::PartialOrder(po) => json!({
            "kind": "po",
            "children": children(po.children()),
            "order": po.order().edges().map(|(i, j)| [i, j]).collect::<Vec<_>>(),
        }),
    }
}

/// The oracle's `canonical_powl`: XOR children sorted, orders closed under
/// transitivity, partial-order children sorted by (own form, predecessor
/// forms, successor forms).
fn canonical(v: &Value) -> Value {
    let Some(children) = v.get("children").and_then(Value::as_array) else {
        return v.clone();
    };
    let mut children: Vec<Value> = children.iter().map(canonical).collect();
    let key = |c: &Value| c.to_string();
    match v["kind"].as_str() {
        Some("loop") => json!({"kind": "loop", "children": children}),
        Some("xor") => {
            children.sort_by_cached_key(key);
            json!({"kind": "xor", "children": children})
        }
        Some("po") => {
            let n = children.len();
            let index = |x: &Value| usize::try_from(x.as_u64().expect("index")).expect("fits");
            let mut order: BTreeSet<(usize, usize)> = v["order"]
                .as_array()
                .expect("order")
                .iter()
                .map(|p| (index(&p[0]), index(&p[1])))
                .collect();
            loop {
                let more: Vec<(usize, usize)> = order
                    .iter()
                    .flat_map(|&(i, j)| order.range((j, 0)..(j + 1, 0)).map(move |&(_, k)| (i, k)))
                    .filter(|e| !order.contains(e))
                    .collect();
                if more.is_empty() {
                    break;
                }
                order.extend(more);
            }
            let keys: Vec<String> = children.iter().map(key).collect();
            let near = |i: usize, before: bool| {
                let mut k: Vec<&str> = (0..n)
                    .filter(|&o| order.contains(&if before { (o, i) } else { (i, o) }))
                    .map(|o| keys[o].as_str())
                    .collect();
                k.sort_unstable();
                k
            };
            let mut rank: Vec<usize> = (0..n).collect();
            rank.sort_by_cached_key(|&i| (keys[i].clone(), near(i, true), near(i, false)));
            let mut at = vec![0; n];
            for (new, &old) in rank.iter().enumerate() {
                at[old] = new;
            }
            let mut pairs: Vec<[usize; 2]> = order.iter().map(|&(i, j)| [at[i], at[j]]).collect();
            pairs.sort_unstable();
            let children: Vec<Value> = rank.iter().map(|&i| children[i].clone()).collect();
            json!({"kind": "po", "children": children, "order": pairs})
        }
        other => panic!("unknown kind {other:?}"),
    }
}

/// The options of an oracle variant name (`POWL_VARIANTS` in
/// `tools/golden/cases/discovery.py`).
fn options(variant: &str) -> PowlOptions {
    let dynamic = |order_frequency_ratio| PowlVariant::DynamicClustering {
        order_frequency_ratio,
    };
    match variant {
        "tree" => PowlOptions::new(PowlVariant::Tree),
        "maximal" => PowlOptions::new(PowlVariant::Maximal),
        "bruteforce" => PowlOptions::new(PowlVariant::BruteForce),
        "dynamic" => PowlOptions::new(dynamic(1.0)),
        "dynamic-ratio08" => PowlOptions::new(dynamic(0.8)),
        "maximal-filter03" => {
            PowlOptions::new(PowlVariant::Maximal).with_filtering_weight_factor(0.3)
        }
        "tree-filter03" => PowlOptions::new(PowlVariant::Tree).with_filtering_weight_factor(0.3),
        _ => panic!("unknown variant {variant}"),
    }
}

/// A log of the given traces, which may be empty.
fn traces_log(traces: &[Vec<String>], keys: &EventKeys) -> EventLog {
    let traces = traces
        .iter()
        .enumerate()
        .map(|(i, activities)| {
            let mut trace = Trace::with_case_id(i.to_string());
            for a in activities {
                let mut event = Event::new();
                event.insert(keys.activity.as_str(), a.as_str());
                trace.events.push(event);
            }
            trace
        })
        .collect();
    EventLog::from_traces(traces)
}

#[test]
fn powl_discovery_matches_pm4py() {
    let ids: Vec<String> = cases("discovery")
        .into_iter()
        .filter(|c| c.starts_with("powl-"))
        .collect();
    assert!(ids.len() > 40, "only {} POWL cases", ids.len());
    let keys = EventKeys::default();
    for id in ids {
        let g = golden("discovery", &id);
        let params = g.meta().params;
        let variant = params["variant"].as_str().expect("variant");
        let log = match params.get("traces") {
            Some(t) => traces_log(
                &serde_json::from_value::<Vec<Vec<String>>>(t.clone()).unwrap(),
                &keys,
            ),
            None => load_csv_log(&g.fixture("log")),
        };
        let model = powl_inductive(&log, &keys, &options(variant)).unwrap();
        assert_eq!(
            canonical(&describe(&model)),
            canonical(g.expected_at("/powl")),
            "{id}: {model}"
        );
    }
}

#[test]
fn canonical_form_ignores_child_order() {
    let parse = |s: &str| canonical(&describe(&Powl::parse(s).unwrap()));
    assert_eq!(parse("X ( a, b )"), parse("X ( b, a )"));
    assert_eq!(
        parse("PO=(nodes={a, b, c}, order={a-->b, b-->c})"),
        parse("PO=(nodes={c, b, a}, order={b-->c, a-->b, a-->c})")
    );
    assert_ne!(
        parse("PO=(nodes={a, b, c}, order={a-->b})"),
        parse("PO=(nodes={a, b, c}, order={b-->a})")
    );
}

#[test]
fn out_of_range_options_are_errors() {
    let keys = EventKeys::default();
    let log = EventLog::from_trace_strings(["a,b"], ",", &keys);
    let bad = [
        PowlOptions::default().with_filtering_weight_factor(1.0),
        PowlOptions::default().with_filtering_weight_factor(-0.1),
        PowlOptions::new(PowlVariant::DynamicClustering {
            order_frequency_ratio: 0.5,
        }),
        PowlOptions::new(PowlVariant::DynamicClustering {
            order_frequency_ratio: 1.1,
        }),
    ];
    for options in bad {
        assert!(matches!(
            powl_inductive(&log, &keys, &options),
            Err(Error::InvalidOption(_))
        ));
    }
}
