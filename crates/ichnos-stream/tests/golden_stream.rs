//! Complete native stream contents by digest, plus typed samples and DFG snapshots.

use ichnos_core::{AttributeValue, Attributes, Event, EventKeys, EventStream, Trace};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_stream::{
    Collector, CsvEventReader, LiveEventStream, LiveTraceStream, StreamingDfgDiscovery,
    StreamingDfgOptions, StreamingDfgResult, TraceIterator, XesEventReader, XesTraceReader,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

fn attribute(value: &AttributeValue) -> Value {
    match value {
        AttributeValue::String(v) | AttributeValue::Id(v) => json!(v.as_ref()),
        AttributeValue::Int(v) => json!(v),
        AttributeValue::Float(v) => json!(v),
        AttributeValue::Bool(v) => json!(v),
        AttributeValue::Date(v) => {
            json!({"date": v.with_timezone(&ichnos_core::chrono::Utc).to_rfc3339_opts(ichnos_core::chrono::SecondsFormat::Micros, false)})
        }
        AttributeValue::List(v) => {
            json!({"list": v.iter().map(|(k,v)| json!([k.as_ref(), attribute(v)])).collect::<Vec<_>>()})
        }
        AttributeValue::Container(v) => json!({"container": attributes(v)}),
        AttributeValue::Meta(v) => {
            json!({"value": attribute(&v.value), "meta": attributes(&v.meta)})
        }
    }
}

fn attributes(attrs: &Attributes) -> Value {
    Value::Object(
        attrs
            .iter()
            .map(|(k, v)| (k.to_string(), attribute(v)))
            .collect(),
    )
}

fn trace(trace: &Trace) -> Value {
    json!({"attributes": attributes(&trace.attributes), "events": trace.events.iter().map(|e| attributes(&e.attributes)).collect::<Vec<_>>()})
}

fn summary(rows: Vec<Value>) -> Value {
    let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&rows).unwrap()));
    let indices: BTreeSet<_> = [0, 1, rows.len().saturating_sub(1)]
        .into_iter()
        .filter(|&i| i < rows.len())
        .collect();
    json!({"count": rows.len(), "sha256": hash, "samples": indices.into_iter().map(|i| json!([i,rows[i]])).collect::<Vec<_>>()})
}

fn model(result: &StreamingDfgResult) -> Value {
    let counts = |map: &ichnos_model::dfg::ActivityCounts| {
        Value::Object(map.iter().map(|(k, n)| (k.to_string(), json!(n))).collect())
    };
    json!({"edges": result.dfg.graph.iter().map(|((a,b),n)| json!([a.to_string(),b.to_string(),n])).collect::<Vec<_>>(),
        "activities": counts(&result.activities), "starts": counts(&result.dfg.start_activities), "ends": counts(&result.dfg.end_activities)})
}

fn compare(actual: &Value, expected: &Value) {
    assert_json_eq(
        actual,
        expected,
        &JsonCompare {
            unordered_arrays: false,
            ..Default::default()
        },
    );
}

fn json_event(row: &Value) -> Event {
    let mut event = Event::new();
    for (key, value) in row.as_object().unwrap() {
        event.insert(key.as_str(), value.as_str().unwrap());
    }
    event
}

fn dfg(name: &str) {
    let g = golden("stream", &format!("dfg-{name}"));
    let events: Vec<Event> = if let Some(rows) = g.meta().params["events"].as_array() {
        rows.iter().map(json_event).collect()
    } else {
        XesEventReader::open(g.fixture("log"), Default::default())
            .unwrap()
            .collect::<ichnos_stream::Result<_>>()
            .unwrap()
    };
    let keys = EventKeys::default()
        .with_activity(
            g.meta().params["activity_key"]
                .as_str()
                .unwrap_or("concept:name"),
        )
        .with_case_id(
            g.meta().params["case_key"]
                .as_str()
                .unwrap_or("case:concept:name"),
        );
    let options = StreamingDfgOptions {
        keys,
        ..Default::default()
    };
    let mut algorithm = StreamingDfgDiscovery::new(options.clone());
    let mut seen = 0;
    for snapshot in g.expected["snapshots"].as_array().unwrap() {
        let at = snapshot["at"].as_u64().unwrap() as usize;
        for event in &events[seen..at] {
            algorithm.push(event).unwrap();
        }
        seen = at;
        compare(&model(algorithm.get()), &snapshot["model"]);
    }
    let live_algorithm = Rc::new(RefCell::new(StreamingDfgDiscovery::new(options)));
    let collector = Rc::new(RefCell::new(Collector::<Event>::new()));
    let removed = Rc::new(RefCell::new(Collector::<Event>::new()));
    let mut live = LiveEventStream::new();
    live.register(live_algorithm.clone());
    let collector_id = live.register(collector.clone());
    assert_eq!(live.register(collector.clone()), collector_id);
    let id = live.register(removed.clone());
    for event in events.iter().take(2) {
        live.append(event.clone()).unwrap();
    }
    assert!(collector.borrow().get().is_empty());
    assert!(live.deregister(id));
    let mut states = vec![format!("{:?}", live.state()).to_lowercase()];
    live.start().unwrap();
    states.push(format!("{:?}", live.state()).to_lowercase());
    for event in events.iter().skip(2) {
        live.append(event.clone()).unwrap();
    }
    live.stop().unwrap();
    assert!(!live.append(Event::new()).unwrap());
    states.push(format!("{:?}", live.state()).to_lowercase());
    compare(&json!(states), &g.expected["states"]);
    compare(
        &model(live_algorithm.borrow().get()),
        &g.expected["live_model"],
    );
    compare(
        &summary(
            collector
                .borrow()
                .get()
                .iter()
                .map(|e| attributes(&e.attributes))
                .collect(),
        ),
        &g.expected["collected"],
    );
    assert_eq!(
        removed.borrow().get().len(),
        g.expected["removed_count"].as_u64().unwrap() as usize
    );
}

struct TemporaryInput(PathBuf);

impl TemporaryInput {
    fn new(text: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ichnos-stream-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, text).unwrap();
        Self(path)
    }
}

impl Drop for TemporaryInput {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}

fn event_adapter(
    item: ichnos_stream::Result<Event>,
    params: &Value,
) -> Option<ichnos_stream::Result<Event>> {
    match item {
        Err(error) => Some(Err(error)),
        Ok(mut event) => {
            if params["transform"] == true {
                event.insert("copied", "yes");
            }
            if params["filter_activity"].as_str().is_some_and(|name| {
                event.get("concept:name").and_then(AttributeValue::as_str) == Some(name)
            }) {
                None
            } else {
                Some(Ok(event))
            }
        }
    }
}

fn importer(name: &str) {
    let g = golden("stream", name);
    let meta = g.meta();
    let input = meta.params["xml"]
        .as_str()
        .or_else(|| meta.params["csv_text"].as_str())
        .map(TemporaryInput::new);
    let path = input
        .as_ref()
        .map(|input| input.0.clone())
        .unwrap_or_else(|| g.fixture("log"));
    match g.meta().params["kind"].as_str().unwrap() {
        "traces" => {
            let mut reader = XesTraceReader::open(path, Default::default()).unwrap();
            let first = reader
                .by_ref()
                .filter(|item| {
                    item.as_ref().ok().is_none_or(|trace| {
                        meta.params["filter_case"].as_str().is_none_or(|name| {
                            trace
                                .attributes
                                .get("concept:name")
                                .and_then(AttributeValue::as_str)
                                != Some(name)
                        })
                    })
                })
                .collect::<ichnos_stream::Result<Vec<_>>>()
                .unwrap();
            compare(
                &summary(first.iter().map(trace).collect()),
                &g.expected["items"],
            );
            reader.reset().unwrap();
            let second = reader
                .by_ref()
                .filter(|item| {
                    item.as_ref().ok().is_none_or(|trace| {
                        meta.params["filter_case"].as_str().is_none_or(|name| {
                            trace
                                .attributes
                                .get("concept:name")
                                .and_then(AttributeValue::as_str)
                                != Some(name)
                        })
                    })
                })
                .collect::<ichnos_stream::Result<Vec<_>>>()
                .unwrap();
            compare(
                &summary(second.iter().map(trace).collect()),
                &g.expected["reset"],
            );
            reader.reset().unwrap();
            let collector = Rc::new(RefCell::new(Collector::<Trace>::new()));
            let mut live = LiveTraceStream::new();
            live.register(collector.clone());
            live.start().unwrap();
            if meta.params["filter_case"].is_string() {
                let items = reader.by_ref().filter(|item| {
                    item.as_ref().ok().is_none_or(|trace| {
                        trace
                            .attributes
                            .get("concept:name")
                            .and_then(AttributeValue::as_str)
                            != meta.params["filter_case"].as_str()
                    })
                });
                assert_eq!(ichnos_stream::feed(items, &mut live).unwrap(), first.len());
            } else {
                assert_eq!(reader.to_trace_stream(&mut live).unwrap(), first.len());
            }
            live.stop().unwrap();
            compare(
                &summary(collector.borrow().get().iter().map(trace).collect()),
                &g.expected["forwarded"],
            );
        }
        kind => {
            // Both readers expose the same event-at-a-time interface.
            fn exercise<R: Iterator<Item = ichnos_stream::Result<Event>>>(
                reader: &mut R,
                expected: &Value,
                params: &Value,
            ) {
                compare(
                    &summary(
                        reader
                            .filter_map(|item| event_adapter(item, params))
                            .map(|e| attributes(&e.unwrap().attributes))
                            .collect(),
                    ),
                    expected,
                );
            }
            let collector = Rc::new(RefCell::new(Collector::<Event>::new()));
            let mut live = LiveEventStream::new();
            live.register(collector.clone());
            live.start().unwrap();
            if kind == "csv" {
                let mut reader = CsvEventReader::open(path, Default::default()).unwrap();
                exercise(&mut reader, &g.expected["items"], &meta.params);
                reader.reset().unwrap();
                exercise(&mut reader, &g.expected["reset"], &meta.params);
                reader.reset().unwrap();
                if meta.params["transform"] == true || meta.params["filter_activity"].is_string() {
                    ichnos_stream::feed(
                        reader
                            .by_ref()
                            .filter_map(|item| event_adapter(item, &meta.params)),
                        &mut live,
                    )
                    .unwrap();
                } else {
                    reader.to_event_stream(&mut live).unwrap();
                }
            } else {
                let mut reader = XesEventReader::open(path, Default::default()).unwrap();
                exercise(&mut reader, &g.expected["items"], &meta.params);
                reader.reset().unwrap();
                exercise(&mut reader, &g.expected["reset"], &meta.params);
                reader.reset().unwrap();
                if meta.params["transform"] == true || meta.params["filter_activity"].is_string() {
                    ichnos_stream::feed(
                        reader
                            .by_ref()
                            .filter_map(|item| event_adapter(item, &meta.params)),
                        &mut live,
                    )
                    .unwrap();
                } else {
                    reader.to_event_stream(&mut live).unwrap();
                }
            }
            live.stop().unwrap();
            compare(
                &summary(
                    collector
                        .borrow()
                        .get()
                        .iter()
                        .map(|e| attributes(&e.attributes))
                        .collect(),
                ),
                &g.expected["forwarded"],
            );
        }
    }
}

fn dataframe(name: &str) {
    let g = golden("stream", &format!("dataframe-{name}"));
    let custom = g.meta().params["custom"].as_bool().unwrap_or(false);
    let keys = if custom {
        EventKeys::default()
            .with_case_id("case")
            .with_activity("task")
            .with_timestamp("stamp")
    } else {
        EventKeys::default()
    };
    let events = if let Some(rows) = g.meta().params["rows"].as_array() {
        rows.iter()
            .map(|row| {
                let mut event = Event::new();
                event.insert(keys.case_id.as_str(), row[0].as_str().unwrap());
                event.insert(keys.activity.as_str(), row[1].as_str().unwrap());
                event.insert(
                    keys.timestamp.as_str(),
                    AttributeValue::Date(
                        ichnos_core::chrono::DateTime::parse_from_rfc3339(row[2].as_str().unwrap())
                            .unwrap(),
                    ),
                );
                event
            })
            .collect()
    } else {
        XesEventReader::open(g.fixture("log"), Default::default())
            .unwrap()
            .collect::<ichnos_stream::Result<Vec<_>>>()
            .unwrap()
    };
    let stream = EventStream {
        events,
        ..Default::default()
    };
    let before = stream.clone();
    let mut reader = TraceIterator::from_event_stream(&stream, &keys).unwrap();
    let first: Vec<_> = reader.by_ref().collect();
    compare(
        &summary(first.iter().map(trace).collect()),
        &g.expected["traces"],
    );
    reader.reset();
    assert_eq!(reader.by_ref().collect::<Vec<_>>(), first);
    reader.reset();
    let collector = Rc::new(RefCell::new(Collector::<Trace>::new()));
    let mut live = LiveTraceStream::new();
    live.register(collector.clone());
    live.start().unwrap();
    reader.to_trace_stream(&mut live).unwrap();
    live.stop().unwrap();
    compare(
        &summary(collector.borrow().get().iter().map(trace).collect()),
        &g.expected["forwarded"],
    );
    assert_eq!(stream, before);
    if name != "empty" {
        let batch = stream.to_arrow().unwrap();
        let arrow = TraceIterator::from_record_batch(&batch, &keys)
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(arrow, first);
    }
    if g.expected["grouped_input"] == true {
        assert_ne!(g.expected["raw"], g.expected["traces"]);
    }
}

macro_rules! cases {
    ($function:ident; $($id:ident => $name:literal),* $(,)?) => { $( #[test] fn $id() { $function($name); } )* };
}

cases!(dfg; dfg_running => "running-example", dfg_receipt => "receipt", dfg_traffic => "roadtraffic100traces",
    dfg_empty => "empty", dfg_interleaved => "interleaved", dfg_missing => "missing", dfg_custom => "custom");
cases!(importer;
    xes_events_running => "xes-events-running-example", xes_events_receipt => "xes-events-receipt", xes_events_traffic => "xes-events-roadtraffic100traces",
    xes_traces_running => "xes-traces-running-example", xes_traces_receipt => "xes-traces-receipt", xes_traces_traffic => "xes-traces-roadtraffic100traces",
    xes_events_typed => "xes-events-typed", xes_traces_typed => "xes-traces-typed",
    csv_running => "csv-running-example", csv_receipt => "csv-receipt", csv_quoted => "csv-quoted");
cases!(dataframe; dataframe_running => "running-example", dataframe_receipt => "receipt", dataframe_traffic => "roadtraffic100traces",
    dataframe_empty => "empty", dataframe_unsorted => "unsorted", dataframe_interleaved => "interleaved", dataframe_custom => "custom");

cases!(importer; csv_transformed_filtered => "csv-transformed-filtered", xes_events_filtered => "xes-events-filtered", xes_traces_filtered => "xes-traces-filtered");
