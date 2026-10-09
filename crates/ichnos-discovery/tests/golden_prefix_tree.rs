use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{Error, PrefixTree, PrefixTreeOptions, prefix_tree};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use serde_json::{Value, json};
fn describe(tree: &PrefixTree) -> Value {
    let mut paths = vec![Vec::<String>::new(); tree.nodes.len()];
    let mut rows = Vec::new();
    for (i, node) in tree.nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            assert!(parent < i);
            assert_eq!(tree.nodes[parent].depth + 1, node.depth);
            paths[i] = paths[parent].clone();
            paths[i].push(node.label.as_ref().unwrap().to_string());
            assert_eq!(tree.nodes[parent].children[node.label.as_ref().unwrap()], i);
        } else {
            assert_eq!(i, 0);
            assert!(node.label.is_none());
            assert_eq!(node.depth, 0);
        }
        rows.push(json!({
            "path": paths[i],
            "depth": node.depth,
            "final": node.final_node,
            "children": node.children.keys().map(ToString::to_string).collect::<Vec<_>>(),
        }));
    }
    rows.sort_by_key(|row| serde_json::from_value::<Vec<String>>(row["path"].clone()).unwrap());
    json!(rows)
}

fn check(name: &str) {
    let g = golden("discovery", &format!("prefix-tree-{name}"));
    let keys = EventKeys::default().with_activity(
        g.meta().params["activity_key"]
            .as_str()
            .unwrap_or("concept:name"),
    );
    let log = if let Some(rows) = g.meta().params["traces"].as_array() {
        EventLog {
            traces: rows
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
                .collect(),
            ..Default::default()
        }
    } else if name.ends_with("xes") {
        ichnos_io::read_xes(g.fixture("log"), &Default::default()).unwrap()
    } else {
        ichnos_io::read_csv(g.fixture("log"), &Default::default()).unwrap()
    };
    let before = log.clone();
    for run in g.expected["runs"].as_array().unwrap() {
        let options = PrefixTreeOptions {
            max_path_length: run["limit"].as_u64().map(|v| v as usize),
        };
        let tree = prefix_tree(&log, &keys, &options).unwrap();
        assert_json_eq(&describe(&tree), &run["nodes"], &JsonCompare::default());
    }
    assert_eq!(log, before);
}

macro_rules! cases {
    ($($test:ident => $name:literal),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                check($name);
            }
        )*
    };
}

cases!(
    running_example => "running-example-xes",
    receipt => "receipt-xes",
    roadtraffic => "roadtraffic100traces-xes",
    even => "interleavings-receipt_even-csv",
    odd => "interleavings-receipt_odd-csv",
    empty => "empty",
    empty_traces => "empty-traces",
    prefixes => "prefixes",
    loops => "loops",
    custom_key => "custom-key",
);

#[test]
fn typed_errors_and_deep_arena() {
    let mut event = Event::new();
    let mut trace = Trace::new();
    trace.events.push(event.clone());
    let mut log = EventLog {
        traces: vec![trace],
        ..Default::default()
    };
    assert!(matches!(
        prefix_tree(&log, &Default::default(), &Default::default()),
        Err(Error::Core(_))
    ));
    event.attributes.insert("concept:name", "a");
    log.traces[0].events = vec![event; 10000];
    let tree = prefix_tree(&log, &Default::default(), &Default::default()).unwrap();
    assert_eq!(tree.nodes.len(), 10001);
    assert_eq!(tree.nodes.last().unwrap().depth, 10000);
    assert!(tree.nodes.last().unwrap().final_node);
}
