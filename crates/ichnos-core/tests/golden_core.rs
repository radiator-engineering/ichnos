//! Compares log conversions and log helpers with pm4py's golden output in
//! `fixtures/golden/core/`. Every case starts from the CSV running example,
//! loaded as the harness loads it.

mod common;

use ichnos_core::artificial::{ARTIFICIAL_END, ARTIFICIAL_START};
use ichnos_core::chrono::SecondsFormat;
use ichnos_core::{
    AttributeValue, Attributes, EventKeys, EventLog, LogEdge, LogNode, SortOrder, Trace,
};
use ichnos_golden::golden;
use serde_json::{Map, Value, json};

fn log() -> EventLog {
    common::load_csv_log("running-example.csv")
}

/// A value as the harness writes it: dates as RFC 3339 UTC with `Z`.
fn to_json(value: &AttributeValue) -> Value {
    match value.plain() {
        AttributeValue::String(s) | AttributeValue::Id(s) => json!(s.as_ref()),
        AttributeValue::Int(v) => json!(v),
        AttributeValue::Float(v) => json!(v),
        AttributeValue::Bool(v) => json!(v),
        AttributeValue::Date(d) => json!(d.to_utc().to_rfc3339_opts(SecondsFormat::AutoSi, true)),
        other => panic!("no JSON form for {other:?}"),
    }
}

fn attributes(attrs: &Attributes) -> Value {
    Value::Object(
        attrs
            .iter()
            .map(|(k, v)| (k.to_string(), to_json(v)))
            .collect::<Map<_, _>>(),
    )
}

fn activities(trace: &Trace) -> Value {
    trace
        .iter()
        .map(|e| to_json(e.get("concept:name").unwrap()))
        .collect()
}

fn case_ids(log: &EventLog) -> Value {
    log.iter().map(|t| to_json(t.case_id().unwrap())).collect()
}

#[test]
fn conversions_match_pm4py() {
    let g = golden("core", "convert-running-example-csv");
    let keys = EventKeys::default();
    let log = log();

    let traces: Vec<Value> = log
        .iter()
        .map(|t| json!({"attributes": attributes(&t.attributes), "activities": activities(t)}))
        .collect();
    assert_eq!(&Value::from(traces), g.expected_at("/traces"));

    let stream = log.to_event_stream(&keys);
    let stream_ids: Vec<Value> = stream
        .events
        .iter()
        .map(|e| to_json(e.get("case:concept:name").unwrap()))
        .collect();
    assert_eq!(&Value::from(stream_ids), g.expected_at("/stream_case_ids"));
    assert_eq!(
        &attributes(&stream.events[0].attributes),
        g.expected_at("/stream_first_event")
    );

    let back = stream.into_event_log(&keys).unwrap();
    assert_eq!(&case_ids(&back), g.expected_at("/round_trip_case_ids"));

    let table = log.to_arrow(&keys).unwrap();
    let columns: Vec<Value> = table
        .schema()
        .fields()
        .iter()
        .map(|f| json!(f.name()))
        .collect();
    assert_eq!(&Value::from(columns), g.expected_at("/dataframe_columns"));
    assert_eq!(&json!(table.num_rows()), g.expected_at("/dataframe_rows"));
}

#[test]
fn artificial_start_end_matches_pm4py() {
    let g = golden("core", "artificial-start-end-running-example-csv");
    let mut log = log();
    log.insert_artificial_start_end(&EventKeys::default(), ARTIFICIAL_START, ARTIFICIAL_END)
        .unwrap();
    let actual: Vec<Value> = log
        .iter()
        .map(|t| {
            let stamps: Vec<Value> = t
                .iter()
                .map(|e| e.get("time:timestamp").map_or(Value::Null, to_json))
                .collect();
            json!({
                "case_id": to_json(t.case_id().unwrap()),
                "activities": activities(t),
                "timestamps": stamps,
            })
        })
        .collect();
    assert_eq!(&Value::from(actual), g.expected_at(""));
}

#[test]
fn projection_matches_pm4py() {
    let g = golden("core", "project-running-example-csv");
    let log = log();
    for key in ["concept:name", "org:resource", "Costs"] {
        let actual: Vec<Vec<Value>> = log
            .project(key)
            .into_iter()
            .map(|t| t.into_iter().map(|v| to_json(v.unwrap())).collect())
            .collect();
        let pointer = format!("/{}", key.replace('~', "~0").replace('/', "~1"));
        assert_eq!(&json!(actual), g.expected_at(&pointer), "{key}");
    }
}

#[test]
fn set_classifier_matches_pm4py() {
    let g = golden("core", "set-classifier-running-example-csv");
    for (name, classifier) in [
        ("activity_and_costs", &["concept:name", "Costs"][..]),
        ("resource", &["org:resource"][..]),
    ] {
        let mut log = log();
        log.insert_classifier_attribute(classifier, "@@classifier")
            .unwrap();
        let actual: Vec<Vec<Value>> = log
            .project("@@classifier")
            .into_iter()
            .map(|t| t.into_iter().map(|v| to_json(v.unwrap())).collect())
            .collect();
        assert_eq!(&json!(actual), g.expected_at(&format!("/{name}")), "{name}");
    }
}

#[test]
fn hof_matches_pm4py() {
    let g = golden("core", "hof-running-example-csv");
    let log = log();

    let long = log.filter_traces(|t| t.len() > 5);
    assert_eq!(&case_ids(&long), g.expected_at("/filter_log_longer_than_5"));

    let mut sorted = log.clone();
    sorted.sort_traces_by_key(Trace::len, SortOrder::Descending);
    assert_eq!(
        &case_ids(&sorted),
        g.expected_at("/sort_log_by_length_reverse")
    );
    let mut ascending = log.clone();
    ascending.sort_traces_by_key(Trace::len, SortOrder::Ascending);
    assert_eq!(&case_ids(&ascending), g.expected_at("/sort_log_by_length"));

    let first = &log.traces[0];
    let no_pete = first
        .filter_events(|e| e.get("org:resource").and_then(AttributeValue::as_str) != Some("Pete"));
    assert_eq!(
        json!({"attributes": attributes(&no_pete.attributes), "activities": activities(&no_pete)}),
        *g.expected_at("/filter_trace_without_pete")
    );

    let mut by_name = first.clone();
    by_name.sort_events_by_key(
        |e| e.get("concept:name").unwrap().to_string(),
        SortOrder::Descending,
    );
    assert_eq!(
        &activities(&by_name),
        g.expected_at("/sort_trace_by_activity_reverse/activities")
    );
    // Behaviour change: pm4py's sort_trace drops the trace attributes; ichnos keeps them.
    assert_eq!(
        g.expected_at("/sort_trace_by_activity_reverse/attributes"),
        &json!({})
    );
    assert_eq!(by_name.attributes, first.attributes);
}

#[test]
fn graph_matches_pm4py() {
    let g = golden("core", "networkx-running-example-csv");
    let graph = log()
        .to_graph(true, &["creator"], &["org:resource", "Costs"])
        .unwrap();

    let mut nodes: Vec<(String, &str)> = graph
        .node_weights()
        .map(|n| {
            let kind = match n {
                LogNode::Case { .. } => "CASE",
                LogNode::Event { .. } => "EVENT",
                LogNode::Attribute(_) => "ATTRIBUTE_NODE",
            };
            (n.id(), kind)
        })
        .collect();
    nodes.sort();
    let nodes: Vec<Value> = nodes
        .into_iter()
        .map(|(id, kind)| json!({"id": id, "type": kind}))
        .collect();
    assert_eq!(&Value::from(nodes), g.expected_at("/nodes"));

    let mut edges: Vec<(String, String, Value)> = graph
        .edge_indices()
        .map(|e| {
            let (s, t) = graph.edge_endpoints(e).unwrap();
            let (kind, name) = match &graph[e] {
                LogEdge::BelongsTo => ("BELONGS_TO", Value::Null),
                LogEdge::DirectlyFollows => ("DF", Value::Null),
                LogEdge::Attribute(key) => ("ATTRIBUTE_EDGE", json!(key.as_ref())),
            };
            let (source, target) = (graph[s].id(), graph[t].id());
            let record = json!({"source": source, "target": target, "type": kind, "name": name});
            (source, target, record)
        })
        .collect();
    edges.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let edges: Vec<Value> = edges.into_iter().map(|(_, _, r)| r).collect();
    assert_eq!(&Value::from(edges), g.expected_at("/edges"));
}

#[test]
fn rebase_matches_pm4py() {
    let g = golden("core", "rebase-running-example-csv");
    let rebased = log()
        .rebase(&EventKeys::default().with_case_id("org:resource"))
        .unwrap();
    let actual: Vec<Value> = rebased
        .iter()
        .map(|t| json!({"case_id": to_json(t.case_id().unwrap()), "activities": activities(t)}))
        .collect();
    assert_eq!(&Value::from(actual), g.expected_at(""));
}

#[test]
fn sample_sizes_match_pm4py() {
    // pm4py samples with Python's `random`, so only the sizes can match.
    let g = golden("core", "sample-running-example-csv");
    let log = log();
    let stream = log.to_event_stream(&EventKeys::default());
    let actual = json!({
        "cases_3": log.sample_cases(3, 0).len(),
        "cases_100": log.sample_cases(100, 0).len(),
        "events_10": stream.sample_events(10, 0).len(),
        "events_100": stream.sample_events(100, 0).len(),
    });
    assert_eq!(&actual, g.expected_at(""));
}
