//! Native streaming snapshots and explicit termination on full real event logs.

use ichnos_core::{Event, EventKeys, chrono::DateTime};
use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_model::{
    Footprints, Label, Marking, PetriNet,
    petri::{ArcEnds, ArcKind},
};
use ichnos_stream::{
    LiveEventStream, StreamingConformanceOptions, StreamingFootprintsConformance,
    StreamingTbrConformance, StreamingTbrOptions, StreamingTemporalConformance,
    StreamingTemporalOptions, TemporalProfile, XesEventReader,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    rc::Rc,
};

fn compare(actual: &Value, expected: &Value) {
    assert_json_eq(
        actual,
        expected,
        &JsonCompare {
            unordered_arrays: false,
            tolerance: ichnos_golden::Tolerance {
                abs: 1e-6,
                rel: 1e-8,
            },
        },
    );
}

fn marking(m: &Marking, net: &PetriNet) -> Value {
    Value::Object(
        m.iter()
            .map(|(p, n)| (net.place(p).name.clone(), json!(n)))
            .collect(),
    )
}

fn tbr_state(algo: &StreamingTbrConformance) -> Value {
    Value::Object(algo.get().iter().map(|(c,s)| {
        assert_eq!(algo.get_status(c),Some(s));
        (c.clone(),json!({"marking":marking(&s.marking,algo.net()),"missing":s.missing,"is_fit":s.is_fit()}))
    }).collect())
}

fn fp_state(algo: &StreamingFootprintsConformance) -> Value {
    Value::Object(algo.get().iter().map(|(c,s)| {
        assert_eq!(algo.get_status(c),Some(s));
        (c.clone(),json!({"last":s.last_activity.as_ref().map(Label::as_str),"deviations":s.deviations,"is_fit":s.is_fit()}))
    }).collect())
}

fn temporal_state(algo: &StreamingTemporalConformance) -> Value {
    Value::Object(
        algo.get()
            .iter()
            .map(|(c, rows)| {
                (
                    c.clone(),
                    json!(
                        rows.iter()
                            .map(|d| {
                                json!([
                                    d.from,
                                    d.to,
                                    d.seconds,
                                    if d.zeta.is_infinite() {
                                        json!("infinity")
                                    } else {
                                        json!(d.zeta)
                                    }
                                ])
                            })
                            .collect::<Vec<_>>()
                    ),
                )
            })
            .collect(),
    )
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
        net.add_arc(ends, arc[3].as_u64().unwrap() as u32, ArcKind::Normal)
            .unwrap();
    }
    let mark = |v: &Value| -> Marking {
        v.as_object()
            .unwrap()
            .iter()
            .map(|(p, n)| (places[p], n.as_u64().unwrap() as u32))
            .collect()
    };
    let (im, fm) = (mark(&spec["initial"]), mark(&spec["final"]));
    (net, im, fm)
}

fn footprints(spec: &Value) -> (Footprints, BTreeSet<Label>) {
    let labels = |key: &str| -> BTreeSet<Label> {
        spec[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| Label::from(v.as_str().unwrap()))
            .collect()
    };
    let pairs = |key: &str| {
        spec[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    Label::from(v[0].as_str().unwrap()),
                    Label::from(v[1].as_str().unwrap()),
                )
            })
            .collect()
    };
    (
        Footprints {
            activities: labels("activities"),
            start_activities: labels("start_activities"),
            sequence: pairs("sequence"),
            parallel: pairs("parallel"),
        },
        labels("end_activities"),
    )
}

fn check(name: &str) {
    let g = golden("stream", &format!("conf-{name}"));
    let params = &g.meta().params;
    let expected = &g.expected;
    let kind = expected["kind"].as_str().unwrap();
    let custom = params["custom"].as_bool().unwrap_or(false);
    let keys = if custom {
        EventKeys::default()
            .with_case_id("case")
            .with_activity("task")
            .with_timestamp("end")
    } else {
        EventKeys::default()
    };
    let start = if params["interval"].as_bool().unwrap_or(false) {
        "start"
    } else {
        &keys.timestamp
    };
    let events: Vec<Event> = if let Some(rows) = params["events"].as_array() {
        rows.iter()
            .map(|row| {
                let mut e = Event::new();
                for (key, value) in row.as_object().unwrap() {
                    let value = value.as_str().unwrap();
                    if key == start || key == &keys.timestamp {
                        e.insert(key.as_str(), DateTime::parse_from_rfc3339(value).unwrap());
                    } else {
                        e.insert(key.as_str(), value);
                    }
                }
                e
            })
            .collect()
    } else {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(&g.meta().fixtures["log"]);
        XesEventReader::open(path, Default::default())
            .unwrap()
            .collect::<ichnos_stream::Result<Vec<_>>>()
            .unwrap()
    };
    let options = StreamingConformanceOptions {
        keys: keys.clone(),
        ..Default::default()
    };
    let points: BTreeSet<_> = expected["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["at"].as_u64().unwrap() as usize)
        .collect();
    let mut snapshots = Vec::new();
    let mut terminated = json!({});
    let mut after;
    if kind == "tbr" {
        let (net, im, fm) = net(&expected["model"]);
        let mut algo = StreamingTbrConformance::new(
            net,
            im,
            fm,
            StreamingTbrOptions {
                events: options,
                maximum_iterations_invisibles: params["maximum_iterations"].as_u64().unwrap_or(10)
                    as usize,
            },
        )
        .unwrap();
        snapshots.push(json!({"at":0,"state":tbr_state(&algo)}));
        for (i, e) in events.iter().enumerate() {
            algo.push(e).unwrap();
            if points.contains(&(i + 1)) {
                snapshots.push(json!({"at":i+1,"state":tbr_state(&algo)}));
            }
        }
        let cases: Vec<_> = algo.get().keys().cloned().collect();
        for c in cases {
            let result = algo.terminate(&c).unwrap();
            terminated[&c] = json!({"marking":marking(&result.marking,algo.net()),"missing":result.missing,"remaining":result.remaining,"is_fit":result.is_fit});
            assert!(algo.get_status(&c).is_none());
            assert!(algo.terminate(&c).is_none());
        }
        let shared = Rc::new(RefCell::new(algo));
        let mut live = LiveEventStream::new();
        live.register(shared.clone());
        live.start().unwrap();
        for event in events {
            live.append(event).unwrap();
        }
        live.stop().unwrap();
        let mut algo = shared.borrow_mut();
        let results = algo.terminate_all();
        assert_eq!(results.len(), terminated.as_object().unwrap().len());
        after = tbr_state(&algo);
    } else if kind == "footprints" {
        let (fp, ends) = footprints(&expected["model"]);
        let mut algo = StreamingFootprintsConformance::new(fp, ends, options);
        snapshots.push(json!({"at":0,"state":fp_state(&algo)}));
        for (i, e) in events.iter().enumerate() {
            algo.push(e).unwrap();
            if points.contains(&(i + 1)) {
                snapshots.push(json!({"at":i+1,"state":fp_state(&algo)}));
            }
        }
        let cases: Vec<_> = algo.get().keys().cloned().collect();
        for c in cases {
            terminated[&c] = json!(algo.terminate(&c).unwrap());
            assert!(algo.get_status(&c).is_none());
            assert!(algo.terminate(&c).is_none());
        }
        let shared = Rc::new(RefCell::new(algo));
        let mut live = LiveEventStream::new();
        live.register(shared.clone());
        live.start().unwrap();
        for event in events {
            live.append(event).unwrap();
        }
        live.stop().unwrap();
        let mut algo = shared.borrow_mut();
        compare(&json!(algo.terminate_all()), &terminated);
        after = fp_state(&algo);
    } else {
        let profile: TemporalProfile = expected["model"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into()),
                    (p[2].as_f64().unwrap(), p[3].as_f64().unwrap()),
                )
            })
            .collect();
        let mut algo = StreamingTemporalConformance::new(
            profile.clone(),
            StreamingTemporalOptions {
                events: options.clone(),
                start_timestamp: start.into(),
                zeta: params["zeta"].as_f64().unwrap_or(6.0),
            },
        )
        .unwrap();
        snapshots.push(json!({"at":0,"state":temporal_state(&algo)}));
        for (i, e) in events.iter().enumerate() {
            algo.push(e).unwrap();
            if points.contains(&(i + 1)) {
                snapshots.push(json!({"at":i+1,"state":temporal_state(&algo)}));
            }
        }
        let shared = Rc::new(RefCell::new(
            StreamingTemporalConformance::new(
                profile,
                StreamingTemporalOptions {
                    events: options,
                    start_timestamp: start.into(),
                    zeta: params["zeta"].as_f64().unwrap_or(6.0),
                },
            )
            .unwrap(),
        ));
        let mut live = LiveEventStream::new();
        live.register(shared.clone());
        live.start().unwrap();
        for event in events {
            live.append(event).unwrap();
        }
        live.stop().unwrap();
        compare(&temporal_state(&shared.borrow()), &temporal_state(&algo));
        after = Value::Null;
    }
    if params["events"].is_null() && kind != "temporal" {
        for snapshot in &mut snapshots {
            snapshot["state"] = compact(&snapshot["state"]);
        }
        terminated = compact(&terminated);
        after = compact(&after);
    }
    compare(
        &json!({"kind":kind,"model":expected["model"],"snapshots":snapshots,"terminated":terminated,"after_termination":after}),
        expected,
    );
}

macro_rules! cases {($($test:ident => $name:literal),* $(,)?) => {$ (#[test] fn $test(){check($name);})*};}
cases! {
    tbr_running => "tbr-running-example", tbr_receipt => "tbr-receipt", tbr_traffic => "tbr-roadtraffic100traces",
    tbr_interleaved => "tbr-interleaved", tbr_empty => "tbr-empty", tbr_custom => "tbr-custom",
    tbr_silent_zero => "tbr-silent-0", tbr_silent_one => "tbr-silent-1", tbr_silent_two => "tbr-silent-2", tbr_silent_ten => "tbr-silent-10", tbr_weighted => "tbr-weighted",
    fp_running => "footprints-running-example", fp_receipt => "footprints-receipt", fp_traffic => "footprints-roadtraffic100traces",
    fp_bad_running => "footprints-deviating-running-example", fp_bad_receipt => "footprints-deviating-receipt", fp_bad_traffic => "footprints-deviating-roadtraffic100traces",
    fp_interleaved => "footprints-interleaved", fp_empty => "footprints-empty", fp_custom => "footprints-custom",
    temporal_running => "temporal-running-example", temporal_receipt => "temporal-receipt", temporal_traffic => "temporal-roadtraffic100traces",
    temporal_zero => "temporal-interval-0", temporal_one => "temporal-interval-1", temporal_six => "temporal-interval-6", temporal_empty => "temporal-empty",
}

fn compact(state: &Value) -> Value {
    use sha2::{Digest, Sha256};
    let rows: Vec<Value> = state
        .as_object()
        .unwrap()
        .iter()
        .map(|(c, v)| json!([c, v]))
        .collect();
    let indices: BTreeSet<_> = [0, 1, rows.len().saturating_sub(1)]
        .into_iter()
        .filter(|&i| i < rows.len())
        .collect();
    json!({"count":rows.len(),"sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&rows).unwrap())),"samples":indices.into_iter().map(|i|json!([i,rows[i]])).collect::<Vec<_>>()})
}
