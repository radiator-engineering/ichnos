//! Native IWS prefix/completion and OCEL routing fixtures.
use ichnos_core::{AttributeValue, Event, EventKeys, chrono::DateTime};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_model::{
    Marking, PetriNet,
    petri::{ArcEnds, ArcKind},
};
use ichnos_stream::{
    Collector, LiveEventStream, OcelDistributorOptions, OcelFlatteningDistributor,
    StreamingAlignmentOptions, StreamingAlignmentResult, StreamingAlignments, XesEventReader,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

fn compare(a: &Value, b: &Value) {
    assert_json_eq(
        a,
        b,
        &JsonCompare {
            unordered_arrays: false,
            ..Default::default()
        },
    );
}

fn net(spec: &Value) -> (PetriNet, Marking, Marking) {
    let mut net = PetriNet::new("stream");
    let places: BTreeMap<_, _> = spec["places"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let name = p.as_str().unwrap();
            (name.to_owned(), net.add_place(name))
        })
        .collect();
    let transitions: BTreeMap<_, _> = spec["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let name = t[0].as_str().unwrap();
            (name.to_owned(), net.add_transition(name, t[1].as_str()))
        })
        .collect();
    for arc in spec["arcs"].as_array().unwrap() {
        let (p, t) = (
            places[arc[0].as_str().unwrap()],
            transitions[arc[1].as_str().unwrap()],
        );
        let ends = if arc[2].as_bool().unwrap() {
            ArcEnds::TransitionToPlace(t, p)
        } else {
            ArcEnds::PlaceToTransition(p, t)
        };
        net.add_arc(
            ends,
            u32::try_from(arc[3].as_u64().unwrap()).unwrap(),
            ArcKind::Normal,
        )
        .unwrap();
    }
    let mark = |v: &Value| -> Marking {
        v.as_object()
            .unwrap()
            .iter()
            .map(|(p, n)| (places[p], u32::try_from(n.as_u64().unwrap()).unwrap()))
            .collect()
    };
    let (im, fm) = (mark(&spec["initial"]), mark(&spec["final"]));
    (net, im, fm)
}

fn attr(v: &Value) -> AttributeValue {
    if let Some(s) = v.as_str() {
        return s.into();
    }
    if let Some(b) = v.as_bool() {
        return b.into();
    }
    if let Some(n) = v.as_i64() {
        return n.into();
    }
    if let Some(n) = v.as_f64() {
        return n.into();
    }
    if let Some(d) = v.get("date") {
        return DateTime::parse_from_rfc3339(d.as_str().unwrap())
            .unwrap()
            .into();
    }
    if let Some(values) = v.as_array() {
        return AttributeValue::List(
            values
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string().into(), attr(v)))
                .collect(),
        );
    }
    panic!("unsupported fixture attribute: {v}")
}

fn event(row: &Value) -> Event {
    let mut e = Event::new();
    for (k, v) in row.as_object().unwrap() {
        e.insert(k.as_str(), attr(v));
    }
    e
}

fn result(s: &StreamingAlignmentResult, algo: &StreamingAlignments) -> Value {
    assert!(s.is_valid);
    let moves: Vec<_> = s
        .alignment
        .iter()
        .map(|step| {
            let log = step.activity.as_deref().unwrap_or(">>");
            let (name, label) = step.transition.map_or((">>", Some(">>")), |t| {
                (
                    algo.net().transition(t).name.as_str(),
                    algo.net().transition(t).label.as_deref(),
                )
            });
            json!([[log, name], [log, label]])
        })
        .collect();
    let mut value = json!({"alignment": moves, "cost": s.cost, "standard_cost": s.cost,
        "is_valid": s.is_valid, "is_complete": s.is_complete, "active_states": s.active_states,
        "approximation_method":"iws", "bound_type":"proxy_alignment_upper_bound", "upper_bound":s.cost});
    if !s.is_complete {
        value["trie_node"] = json!(s.trie_node.unwrap());
        value["decay"] = json!(s.decay.unwrap());
        value["processed_events"] = json!(algo.processed_events());
    }
    value
}

fn state(algo: &StreamingAlignments) -> Value {
    Value::Object(
        algo.get()
            .iter()
            .map(|(c, s)| (c.clone(), result(s, algo)))
            .collect(),
    )
}

fn compact(state: &Value) -> Value {
    use sha2::{Digest, Sha256};
    let rows: Vec<_> = state
        .as_object()
        .unwrap()
        .iter()
        .map(|(c, v)| json!([c, v]))
        .collect();
    let indices: BTreeSet<_> = [0, 1, rows.len().saturating_sub(1)]
        .into_iter()
        .filter(|&i| i < rows.len())
        .collect();
    json!({"count": rows.len(), "sha256": format!("{:x}",Sha256::digest(serde_json::to_vec(&rows).unwrap())),
        "samples": indices.into_iter().map(|i| json!([i,rows[i]])).collect::<Vec<_>>()})
}

fn check_iws(name: &str) {
    let g = golden("stream", &format!("iws-{name}"));
    let p = &g.meta().params;
    let expected = &g.expected;
    let (net, im, fm) = net(&expected["model"]);
    let sequences = expected["proxy_sequences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            s.as_array()
                .unwrap()
                .iter()
                .map(|t| net.transition_by_name(t.as_str().unwrap()).unwrap())
                .collect()
        })
        .collect();
    let mut o = StreamingAlignmentOptions::default();
    if p["custom"] == true {
        o.events.keys = EventKeys::default()
            .with_case_id("case")
            .with_activity("task");
    }
    o.look_ahead = p["look_ahead"].as_u64().unwrap_or(3) as usize;
    o.decay_time = p["decay_time"].as_f64().unwrap_or(10.0);
    o.discount_factor = p["discount_factor"].as_f64().unwrap_or(0.9);
    o.max_states = p["max_states"].as_u64().unwrap_or(20) as usize;
    let mut algo = StreamingAlignments::with_proxy_sequences(
        net.clone(),
        im.clone(),
        fm.clone(),
        sequences,
        o.clone(),
    )
    .unwrap();
    // Unique-path fixtures also compare automatic generation and the proxy-log
    // preparation that reuses the merged aligner, not only supplied runs.
    let mut prepared = if p["automatic"] == true {
        Some(StreamingAlignments::new(net.clone(), im.clone(), fm.clone(), o.clone()).unwrap())
    } else if let Some(proxy) = p["proxy"].as_array() {
        let traces: Vec<Vec<_>> = proxy
            .iter()
            .map(|s| {
                s.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect()
            })
            .collect();
        Some(
            StreamingAlignments::from_proxy_traces(
                net.clone(),
                im.clone(),
                fm.clone(),
                &traces,
                o.clone(),
            )
            .unwrap(),
        )
    } else {
        None
    };
    let real = p["events"].is_null();
    let events: Vec<Event> = if real {
        XesEventReader::open(g.fixture("log"), Default::default())
            .unwrap()
            .collect::<ichnos_stream::Result<_>>()
            .unwrap()
    } else {
        p["events"].as_array().unwrap().iter().map(event).collect()
    };
    let normalize = |v: Value| if real { compact(&v) } else { v };
    let mut snapshots = vec![json!({"at":0,"state":normalize(state(&algo))})];
    let points: BTreeSet<_> = expected["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["at"].as_u64().unwrap() as usize)
        .collect();
    for (i, e) in events.iter().enumerate() {
        algo.push(e).unwrap();
        if let Some(prepared) = prepared.as_mut() {
            prepared.push(e).unwrap();
            assert_eq!(prepared.get(), algo.get());
        }
        if points.contains(&(i + 1)) {
            snapshots.push(json!({"at":i+1,"state":normalize(state(&algo))}));
        }
    }
    compare(&json!(snapshots), &expected["snapshots"]);
    let finished: BTreeMap<_, _> = algo
        .get()
        .keys()
        .map(|c| {
            let s = algo.finish(c).unwrap();
            (c.clone(), result(&s, &algo))
        })
        .collect();
    compare(&normalize(json!(finished)), &expected["finished"]);
    compare(&normalize(state(&algo)), &expected["after_finish"]);
    assert!(algo.finish("absent").is_none());
    let live_algo = Rc::new(RefCell::new(
        StreamingAlignments::with_proxy_sequences(
            net,
            im,
            fm,
            expected["proxy_sequences"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| {
                    s.as_array()
                        .unwrap()
                        .iter()
                        .map(|t| algo.net().transition_by_name(t.as_str().unwrap()).unwrap())
                        .collect()
                })
                .collect(),
            o,
        )
        .unwrap(),
    ));
    let mut live = LiveEventStream::new();
    live.register(live_algo.clone());
    if let Some(e) = events.first() {
        live.append(e.clone()).unwrap();
    }
    live.start().unwrap();
    for e in events.iter().skip(1) {
        live.append(e.clone()).unwrap();
    }
    live.stop().unwrap();
    compare(
        &normalize(state(&live_algo.borrow())),
        &expected["snapshots"].as_array().unwrap().last().unwrap()["state"],
    );
}

fn output_attr(v: &AttributeValue) -> Value {
    match v.plain() {
        AttributeValue::String(s) | AttributeValue::Id(s) => json!(s.as_ref()),
        AttributeValue::Int(n) => json!(n),
        AttributeValue::Float(n) => json!(n),
        AttributeValue::Bool(b) => json!(b),
        AttributeValue::Date(d) => {
            json!({"date":d.with_timezone(&ichnos_core::chrono::Utc).to_rfc3339_opts(ichnos_core::chrono::SecondsFormat::Micros,false)})
        }
        _ => panic!("unexpected flattened fixture attribute"),
    }
}

fn check_ocel(name: &str) {
    let g = golden("stream", &format!("ocel-{name}"));
    let p = &g.meta().params;
    let mut o = OcelDistributorOptions::default();
    if p["custom"] == true {
        o.keys = EventKeys::default()
            .with_activity("task")
            .with_case_id("case")
            .with_timestamp("end");
        o.activity_key = "act".into();
        o.timestamp_key = "when".into();
        o.object_type_prefix = "objects:".into();
    }
    let mut d = OcelFlatteningDistributor::new(o).unwrap();
    let mut collectors = BTreeMap::new();
    let mut streams = Vec::new();
    for ot in g.expected["types"].as_array().unwrap() {
        let ot = ot.as_str().unwrap();
        let collector = Rc::new(RefCell::new(Collector::<Event>::new()));
        let stream = Rc::new(RefCell::new(LiveEventStream::new()));
        stream.borrow_mut().register(collector.clone());
        d.register(ot, stream.clone());
        if p["duplicate"] == true {
            d.register(ot, stream.clone());
        }
        stream.borrow_mut().start().unwrap();
        collectors.insert(ot, collector);
        streams.push(stream);
    }
    let events: Vec<_> = g.expected["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(event)
        .collect();
    let before = events.clone();
    // Distributor itself is a sink and can consume from an outer live stream.
    let mut input = LiveEventStream::new();
    input.register(d);
    for e in &events {
        input.append(e.clone()).unwrap();
    }
    input.start().unwrap();
    input.stop().unwrap();
    for s in streams {
        s.borrow_mut().stop().unwrap();
    }
    let flattened = Value::Object(
        collectors
            .iter()
            .map(|(ot, c)| {
                let rows: Vec<Value> = c
                    .borrow()
                    .get()
                    .iter()
                    .map(|e| {
                        Value::Object(
                            e.attributes
                                .iter()
                                .map(|(k, v)| (k.to_string(), output_attr(v)))
                                .collect(),
                        )
                    })
                    .collect();
                (ot.to_string(), json!(rows))
            })
            .collect(),
    );
    compare(&flattened, &g.expected["flattened"]);
    assert_eq!(events, before);
}

macro_rules! iws { ($($id:ident => $name:literal),* $(,)?) => { $(#[test] fn $id() { check_iws($name); })* }; }
iws! { iws_running => "running-example", iws_receipt => "receipt", iws_traffic => "roadtraffic100traces",
iws_silent => "silent-lookahead", iws_one => "lookahead-one", iws_decay => "decay-fallback", iws_cap => "state-cap",
iws_auto => "automatic-chain", iws_completion => "completion-custom", iws_empty => "empty", iws_duplicates => "duplicate-labels", iws_branches => "branching-proxy" }
#[test]
fn ocel_example() {
    check_ocel("example");
}
#[test]
fn ocel_duplicates() {
    check_ocel("duplicates");
}
#[test]
fn ocel_custom() {
    check_ocel("custom");
}
#[test]
fn ocel_empty() {
    check_ocel("empty");
}

#[test]
fn iws_real_pnml() {
    check_iws("running-example-pnml");
}
