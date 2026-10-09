use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{
    Error, TransitionAbstraction, TransitionDirection, TransitionDiscovery,
    TransitionSystemOptions, TransitionView, discover_transition_system, transition_system,
};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use serde_json::{Value, json};
fn view(value: &TransitionView) -> Value {
    match value {
        TransitionView::Sequence(labels) => {
            json!(labels.iter().map(ToString::to_string).collect::<Vec<_>>())
        }
        TransitionView::Set(labels) => {
            json!(labels.iter().map(ToString::to_string).collect::<Vec<_>>())
        }
        TransitionView::Multiset(counts) => json!(
            counts
                .iter()
                .map(|(label, count)| json!([label.to_string(), count]))
                .collect::<Vec<_>>()
        ),
    }
}
fn describe(result: &TransitionDiscovery) -> Value {
    let positions = |values: &[ichnos_discovery::TransitionEvent]| {
        values
            .iter()
            .map(|event| [event.trace, event.event])
            .collect::<Vec<_>>()
    };
    let mut states = Vec::new();
    for (id, state) in result.system.states() {
        assert!(!state.incoming().is_empty() || !state.outgoing().is_empty());
        let data = result.state_data.get(&id).cloned().unwrap_or_default();
        states.push(json!({"view":view(&result.views[&id]),"incoming":positions(&data.incoming),"outgoing":positions(&data.outgoing)}));
    }
    states.sort_by_key(|row| row["view"].to_string());
    let mut edges = Vec::new();
    for (id, edge) in result.system.edges() {
        edges.push(json!({"from":view(&result.views[&edge.from()]),"to":view(&result.views[&edge.to()]),"label":edge.name,"events":positions(result.edge_data.get(&id).map(Vec::as_slice).unwrap_or(&[]))}));
    }
    edges.sort_by_key(|row| {
        (
            row["from"].to_string(),
            row["label"].as_str().unwrap().to_owned(),
            row["to"].to_string(),
        )
    });
    json!({"states":states,"edges":edges})
}
fn check(name: &str) {
    let g = golden("discovery", &format!("transition-system-{name}"));
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
        let option = &run["options"];
        let options = TransitionSystemOptions {
            direction: if option["direction"] == "forward" {
                TransitionDirection::Forward
            } else {
                TransitionDirection::Backward
            },
            view: match option["view"].as_str().unwrap() {
                "sequence" => TransitionAbstraction::Sequence,
                "set" => TransitionAbstraction::Set,
                _ => TransitionAbstraction::Multiset,
            },
            window: option["window"].as_u64().unwrap() as usize,
            include_data: option["include_data"].as_bool().unwrap(),
        };
        let result = discover_transition_system(&log, &keys, &options).unwrap();
        assert_json_eq(&describe(&result), &run["graph"], &JsonCompare::default());
        assert_eq!(
            transition_system(&log, &keys, &options).unwrap(),
            result.system
        );
        if options.include_data {
            assert_eq!(
                result.edge_data.values().map(Vec::len).sum::<usize>(),
                log.traces
                    .iter()
                    .map(|trace| trace.events.len())
                    .sum::<usize>()
            );
        }
    }
    assert_eq!(log, before);
}
macro_rules! cases { ($($test:ident=>$name:literal),*)=>{$(#[test]fn $test(){check($name);})*}; }
cases!(running_example=>"running-example-xes",receipt=>"receipt-xes",roadtraffic=>"roadtraffic100traces-xes",even=>"interleavings-receipt_even-csv",odd=>"interleavings-receipt_odd-csv",empty=>"empty",empty_traces=>"empty-traces",views=>"views",single=>"single",custom_key=>"custom-key");
#[test]
fn positional_errors_and_extreme_window() {
    let mut trace = Trace::new();
    trace.events.push(Event::new());
    let mut log = EventLog {
        traces: vec![trace],
        ..Default::default()
    };
    assert!(matches!(
        transition_system(&log, &Default::default(), &Default::default()),
        Err(Error::Core(_))
    ));
    log.traces[0].events[0]
        .attributes
        .insert("concept:name", "a");
    for direction in [TransitionDirection::Forward, TransitionDirection::Backward] {
        let options = TransitionSystemOptions {
            window: usize::MAX,
            direction,
            ..Default::default()
        };
        let result = discover_transition_system(&log, &Default::default(), &options).unwrap();
        assert_eq!(result.system.state_count(), 2);
        assert_eq!(result.system.edge_count(), 1);
    }
}
