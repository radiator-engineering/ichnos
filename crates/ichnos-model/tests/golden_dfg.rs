//! DFG filters against pm4py's (`fixtures/golden/dfg`).
//!
//! Each golden file holds the DFG of a log and the result of every filter
//! over a parameter grid; see `tools/golden/cases/dfg.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_golden::{cases, golden};
use ichnos_model::Label;
use ichnos_model::dfg::{ActivityCounts, Dfg};
use serde_json::{Map, Value, json};

fn counts(v: &Value) -> BTreeMap<Label, u64> {
    v.as_object()
        .expect("an activity map")
        .iter()
        .map(|(a, n)| (Label::from(a.as_str()), n.as_u64().expect("a count")))
        .collect()
}

fn read_dfg(v: &Value) -> (Dfg, ActivityCounts) {
    let mut dfg = Dfg::new();
    for (edge, n) in v["edges"].as_object().expect("edges") {
        let (a, b) = edge.split_once(" -> ").expect("an `a -> b` key");
        dfg.add_edge(a, b, n.as_u64().expect("a count"));
    }
    dfg.start_activities = counts(&v["start_activities"]);
    dfg.end_activities = counts(&v["end_activities"]);
    (dfg, counts(&v["activities_count"]))
}

fn edges(dfg: &Dfg) -> Value {
    let map: Map<String, Value> = dfg
        .graph
        .iter()
        .map(|((a, b), n)| (format!("{a} -> {b}"), json!(n)))
        .collect();
    Value::Object(map)
}

fn activities(c: &BTreeMap<Label, u64>) -> Value {
    let map: Map<String, Value> = c.iter().map(|(a, n)| (a.to_string(), json!(n))).collect();
    Value::Object(map)
}

fn describe((dfg, counts): &(Dfg, ActivityCounts)) -> Value {
    json!({
        "edges": edges(dfg),
        "start_activities": activities(&dfg.start_activities),
        "end_activities": activities(&dfg.end_activities),
        "activities_count": activities(counts),
    })
}

#[test]
fn dfg_filters_match_pm4py() {
    let ids = cases("dfg");
    assert_eq!(ids.len(), 2, "expected 2 DFG goldens, found {ids:?}");
    for id in ids {
        let g = golden("dfg", &id);
        let (dfg, ac) = read_dfg(g.expected_at("/input"));
        let results = g.expected_at("/results").as_array().expect("results");
        assert!(!results.is_empty());
        for r in results {
            let filter = r["filter"].as_str().expect("filter name");
            let param = &r["param"];
            let keep_all = r["keep_all_activities"].as_bool().unwrap_or(false);
            let number = || param.as_f64().expect("a numeric parameter");
            let activity = || param.as_str().expect("an activity parameter");
            let actual = match filter {
                "activities_percentage" => {
                    describe(&dfg.filter_activities_percentage(&ac, number()))
                }
                "paths_percentage" => {
                    describe(&dfg.filter_paths_percentage(&ac, number(), keep_all))
                }
                "keep_connected" => describe(&dfg.filter_keep_connected(&ac, number(), keep_all)),
                "to_activity" => describe(&dfg.filter_to_activity(&ac, activity()).expect("ok")),
                "from_activity" => {
                    describe(&dfg.filter_from_activity(&ac, activity()).expect("ok"))
                }
                "contain_activity" => {
                    describe(&dfg.filter_contain_activity(&ac, activity()).expect("ok"))
                }
                "clean_noise" => {
                    json!({"edges": edges(&dfg.clean_noise(number(), &BTreeSet::new()))})
                }
                other => panic!("unknown filter {other}"),
            };
            let context = format!("{id}: {filter}({param}, keep_all={keep_all})");
            assert_eq!(actual, r["output"], "{context}");
        }
    }
}
