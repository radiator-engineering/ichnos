//! Compares the footprints of logs, traces, DFGs and POWL models with
//! pm4py's `discover_footprints` (`fixtures/golden/discovery/footprints-*`);
//! see `tools/golden/cases/discovery.py`.

// Only the CSV reader is used here.
#[allow(dead_code)]
mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::load_csv_log;
use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{
    DfgOptions, LogFootprints, PowlOptions, TraceFootprints, dfg, dfg_footprints, log_footprints,
    powl_inductive, trace_footprints,
};
use ichnos_golden::{Golden, cases, golden};
use ichnos_model::footprints::LabelPair;
use ichnos_model::{Footprints, Label, Powl, PowlFootprints};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const LOGS: [&str; 6] = [
    "running-example-csv",
    "receipt-csv",
    "roadtraffic100traces-csv",
    "interleavings-receipt_even-csv",
    "interleavings-receipt_odd-csv",
    "reviewing-csv",
];

fn labels(s: &BTreeSet<Label>) -> Value {
    json!(s.iter().map(Label::as_str).collect::<Vec<_>>())
}

fn pairs(s: &BTreeSet<LabelPair>) -> Value {
    json!(
        s.iter()
            .map(|(a, b)| [a.as_str(), b.as_str()])
            .collect::<Vec<_>>()
    )
}

fn counts(m: &BTreeMap<LabelPair, u64>) -> Value {
    json!(
        m.iter()
            .map(|((a, b), n)| json!([[a.as_str(), b.as_str()], n]))
            .collect::<Vec<_>>()
    )
}

fn shared(fp: &Footprints) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("activities".into(), labels(&fp.activities));
    m.insert("start_activities".into(), labels(&fp.start_activities));
    m.insert("sequence".into(), pairs(&fp.sequence));
    m.insert("parallel".into(), pairs(&fp.parallel));
    m
}

fn log_json(fp: &LogFootprints) -> Value {
    let mut m = shared(&fp.footprints);
    m.insert("dfg".into(), counts(&fp.dfg));
    m.insert("end_activities".into(), labels(&fp.end_activities));
    m.insert("min_trace_length".into(), json!(fp.min_trace_length));
    Value::Object(m)
}

fn trace_json(fp: &TraceFootprints) -> Value {
    let mut m = shared(&fp.footprints);
    m.insert("dfg".into(), counts(&fp.dfg));
    m.insert("end_activities".into(), labels(&fp.end_activities));
    m.insert("min_trace_length".into(), json!(fp.min_trace_length));
    m.insert(
        "trace".into(),
        json!(fp.trace.iter().map(Label::as_str).collect::<Vec<_>>()),
    );
    Value::Object(m)
}

fn powl_json(fp: &PowlFootprints) -> Value {
    let mut m = shared(&fp.footprints);
    m.insert("end_activities".into(), labels(&fp.end_activities));
    m.insert(
        "activities_always_happening".into(),
        labels(&fp.activities_always_happening),
    );
    m.insert("skippable".into(), json!(fp.skippable));
    m.insert("min_trace_length".into(), json!(fp.min_trace_length));
    Value::Object(m)
}

/// How many distinct per-trace footprints a golden keeps in full
/// (`FOOTPRINTS_TRACE_ROWS` in the generator).
const TRACE_ROWS: usize = 20;

/// Groups the per-trace footprints as the generator does: one entry per
/// distinct JSON form with its `count`, sorted by that form. Then checks the
/// first [`TRACE_ROWS`] entries, the totals and the SHA-256 of the rest.
fn check_traces(g: &Golden, case: &str, fps: &[TraceFootprints]) {
    let mut groups: BTreeMap<String, (Value, u64)> = BTreeMap::new();
    for fp in fps {
        let v = trace_json(fp);
        groups.entry(v.to_string()).or_insert((v, 0)).1 += 1;
    }
    let rows: Vec<Value> = groups
        .into_values()
        .map(|(mut v, n)| {
            v.as_object_mut()
                .expect("object")
                .insert("count".into(), json!(n));
            v
        })
        .collect();
    let expected = g.expected_at("/traces");
    assert_eq!(
        json!(fps.len()),
        expected["traces"],
        "{case}: number of traces"
    );
    assert_eq!(
        json!(rows.len()),
        expected["groups"],
        "{case}: number of distinct trace footprints"
    );
    let split = rows.len().min(TRACE_ROWS);
    assert_eq!(
        Value::Array(rows[..split].to_vec()),
        expected["first"],
        "{case}: first trace footprints"
    );
    let rest = serde_json::to_vec(&Value::Array(rows[split..].to_vec())).unwrap();
    assert_eq!(
        json!(format!("{:x}", Sha256::digest(rest))),
        expected["rest_sha256"],
        "{case}: remaining trace footprints"
    );
}

#[test]
fn log_footprints_match_pm4py() {
    let keys = EventKeys::default();
    for log_id in LOGS {
        let case = format!("footprints-log-{log_id}");
        let g = golden("discovery", &case);
        let log = load_csv_log(&g.fixture("log"));
        let entire = log_footprints(&log, &keys).unwrap();
        assert_eq!(
            log_json(&entire),
            *g.expected_at("/entire"),
            "{case}: entire"
        );
        check_traces(&g, &case, &trace_footprints(&log, &keys).unwrap());
        let d = dfg_footprints(&dfg(&log, &keys, &DfgOptions::default()).unwrap());
        let mut fp = shared(&d.footprints);
        fp.insert("end_activities".into(), labels(&d.end_activities));
        assert_eq!(Value::Object(fp), *g.expected_at("/dfg"), "{case}: dfg");
        let model = powl_inductive(&log, &keys, &PowlOptions::default()).unwrap();
        assert_eq!(
            powl_json(&model.footprints()),
            *g.expected_at("/powl"),
            "{case}: powl"
        );
    }
}

#[test]
fn footprints_of_logs_with_empty_traces_match_pm4py() {
    let case = "footprints-log-synthetic-emptytraces";
    let g = golden("discovery", case);
    let keys = EventKeys::default();
    let traces: Vec<Vec<String>> =
        serde_json::from_value(g.meta().params["traces"].clone()).unwrap();
    let log = EventLog::from_traces(
        traces
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
            .collect(),
    );
    let entire = log_footprints(&log, &keys).unwrap();
    assert_eq!(
        log_json(&entire),
        *g.expected_at("/entire"),
        "{case}: entire"
    );
    check_traces(&g, case, &trace_footprints(&log, &keys).unwrap());
}

#[test]
fn powl_footprints_match_pm4py() {
    let ids: Vec<String> = cases("discovery")
        .into_iter()
        .filter(|c| c.starts_with("footprints-powl-"))
        .collect();
    assert!(ids.len() > 10, "only {} POWL footprint cases", ids.len());
    for id in ids {
        let g = golden("discovery", &id);
        let text = g.meta().params["text"].as_str().expect("text").to_owned();
        let model = Powl::parse(&text).unwrap();
        assert_eq!(
            powl_json(&model.footprints()),
            *g.expected_at("/powl"),
            "{id}: powl"
        );
        // Null when pm4py turns a choice or loop of two silent steps into a
        // frequent transition, which ichnos keeps as it is.
        if g.expected_at("/frequent").is_null() {
            continue;
        }
        let frequent = model.simplify_using_frequent_transitions();
        assert_eq!(
            powl_json(&frequent.footprints()),
            *g.expected_at("/frequent"),
            "{id}: frequent ({frequent})"
        );
    }
}
