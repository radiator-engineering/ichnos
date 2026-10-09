//! Complete typed BPMN graph goldens for classic and lifecycle-aware SM2.

use ichnos_core::{Event, EventKeys, EventLog, Trace};
use ichnos_discovery::{
    SplitMinerOptions, SplitMinerVariant, bpmn_split_miner, discover_split_miner,
};
use ichnos_golden::{Golden, fixture_path, golden};
use ichnos_model::bpmn::{Bpmn, GatewayKind};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn load(g: &Golden) -> (EventLog, EventKeys) {
    let keys = EventKeys::default()
        .with_activity(
            g.meta().params["activity_key"]
                .as_str()
                .unwrap_or("concept:name"),
        )
        .with_timestamp(
            g.meta().params["timestamp_key"]
                .as_str()
                .unwrap_or("time:timestamp"),
        )
        .with_transition(
            g.meta().params["transition_key"]
                .as_str()
                .unwrap_or("lifecycle:transition"),
        );
    let log = if let Some(traces) = g.meta().params["events"].as_array() {
        EventLog {
            traces: traces
                .iter()
                .map(|t| {
                    let mut trace = Trace::new();
                    for row in t.as_array().unwrap() {
                        let mut event = Event::new();
                        event
                            .attributes
                            .insert(keys.activity.as_str(), row[0].as_str().unwrap());
                        event
                            .attributes
                            .insert(keys.transition.as_str(), row[1].as_str().unwrap());
                        if let Some(stamp) = row[2].as_str() {
                            event.attributes.insert(
                                keys.timestamp.as_str(),
                                ichnos_core::AttributeValue::Date(
                                    ichnos_core::chrono::DateTime::parse_from_rfc3339(stamp)
                                        .unwrap(),
                                ),
                            );
                        }
                        trace.events.push(event);
                    }
                    trace
                })
                .collect(),
            ..Default::default()
        }
    } else if let Some(traces) = g.meta().params["traces"].as_array() {
        EventLog {
            traces: traces
                .iter()
                .map(|t| {
                    let mut trace = Trace::new();
                    for label in t.as_array().unwrap() {
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
    } else {
        let path = fixture_path(
            g.meta().fixtures["log"]
                .strip_prefix("fixtures/logs/")
                .unwrap(),
        );
        if path.extension().unwrap() == "xes" {
            ichnos_io::read_xes(path, &Default::default()).unwrap()
        } else {
            ichnos_io::read_csv(path, &Default::default()).unwrap()
        }
    };
    (log, keys)
}

fn graph(b: &Bpmn, looped: &std::collections::BTreeSet<ichnos_model::bpmn::NodeId>) -> Value {
    let nodes: Vec<_> = b.nodes().collect();
    let ids: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();
    let node_rows: Vec<_> = nodes
        .iter()
        .map(|(id, node)| {
            let kind = if node.kind.is_task() {
                "task"
            } else if node.kind.is_start_event() {
                "start"
            } else if node.kind.is_end_event() {
                "end"
            } else {
                match node.kind.gateway_kind().unwrap() {
                    GatewayKind::Exclusive => "xor",
                    GatewayKind::Parallel => "and",
                    GatewayKind::Inclusive => "or",
                    _ => panic!("unexpected gateway"),
                }
            };
            json!([kind, node.name, looped.contains(id)])
        })
        .collect();
    let edges: Vec<_> = b
        .flows()
        .map(|(_, flow)| json!([ids[&flow.source()], ids[&flow.target()]]))
        .collect();
    json!({"nodes": node_rows, "edges": edges})
}

fn matrix(v: &Value) -> Vec<Vec<usize>> {
    let n = v["nodes"].as_array().unwrap().len();
    let mut m = vec![vec![0; n]; n];
    for e in v["edges"].as_array().unwrap() {
        m[e[0].as_u64().unwrap() as usize][e[1].as_u64().unwrap() as usize] += 1;
    }
    m
}

fn isomorphic(a: &Value, b: &Value) -> bool {
    let an = a["nodes"].as_array().unwrap();
    let bn = b["nodes"].as_array().unwrap();
    if an.len() != bn.len()
        || a["edges"].as_array().unwrap().len() != b["edges"].as_array().unwrap().len()
    {
        return false;
    }
    let n = an.len();
    let (am, bm) = (matrix(a), matrix(b));
    let mut colors: Vec<_> = an.iter().chain(bn).map(Value::to_string).collect();
    loop {
        let mut classes: BTreeMap<String, usize> = BTreeMap::new();
        for c in &colors {
            classes.insert(c.clone(), 0);
        }
        for (i, c) in classes.values_mut().enumerate() {
            *c = i;
        }
        let codes: Vec<_> = colors.iter().map(|c| classes[c]).collect();
        let mut next = Vec::new();
        for (m, offset) in [(&am, 0), (&bm, n)] {
            for i in 0..n {
                let mut incoming = Vec::new();
                let mut outgoing = Vec::new();
                for j in 0..n {
                    incoming.extend(std::iter::repeat_n(codes[offset + j], m[j][i]));
                    outgoing.extend(std::iter::repeat_n(codes[offset + j], m[i][j]));
                }
                incoming.sort();
                outgoing.sort();
                next.push(format!("{:?}", (codes[offset + i], incoming, outgoing)));
            }
        }
        let next_count = next.iter().collect::<std::collections::BTreeSet<_>>().len();
        colors = next;
        if next_count == classes.len() {
            break;
        }
    }
    fn search(
        am: &[Vec<usize>],
        bm: &[Vec<usize>],
        colors: &[String],
        map: &mut [Option<usize>],
        used: &mut [bool],
    ) -> bool {
        let n = map.len();
        let choose = (0..n)
            .filter(|&i| map[i].is_none())
            .map(|i| {
                let candidates: Vec<_> = (0..n)
                    .filter(|&j| {
                        !used[j]
                            && colors[i] == colors[n + j]
                            && am[i][i] == bm[j][j]
                            && (0..n).all(|k| {
                                map[k].is_none_or(|l| am[i][k] == bm[j][l] && am[k][i] == bm[l][j])
                            })
                    })
                    .collect();
                (i, candidates)
            })
            .min_by_key(|(_, c)| c.len());
        let Some((i, candidates)) = choose else {
            return true;
        };
        for j in candidates {
            map[i] = Some(j);
            used[j] = true;
            if search(am, bm, colors, map, used) {
                return true;
            }
            map[i] = None;
            used[j] = false;
        }
        false
    }
    search(&am, &bm, &colors, &mut vec![None; n], &mut vec![false; n])
}

fn check(name: &str) {
    let g = golden("discovery", &format!("split-miner-{name}"));
    let expected: Value = g.expected_as();
    let (log, keys) = load(&g);
    let before = log.clone();
    for row in expected.as_array().unwrap() {
        let o = &row["options"];
        let options = SplitMinerOptions {
            variant: if o["variant"] == "sm2" {
                SplitMinerVariant::Sm2
            } else {
                SplitMinerVariant::Classic
            },
            epsilon: o["epsilon"].as_f64().unwrap(),
            eta: o["eta"].as_f64().unwrap(),
            minimize_or_joins: o["minimize_or_joins"].as_bool().unwrap(),
        };
        let actual = discover_split_miner(&log, &keys, &options);
        if row["graph"].is_null() {
            assert!(actual.is_err());
        } else {
            let result = actual.unwrap();
            let model = result.bpmn;
            let observed = graph(&model, &result.looped_tasks);
            assert!(
                isomorphic(&observed, &row["graph"]),
                "{name} {options:?}\nactual {observed}\nexpected {}",
                row["graph"]
            );
            for (_, node) in model.nodes() {
                assert!(!node.in_flows().is_empty() || node.kind.is_start_event());
                assert!(!node.out_flows().is_empty() || node.kind.is_end_event());
            }
        }
    }
    assert_eq!(log, before);
}

macro_rules! cases{($($id:ident=>$name:literal),*)=>{$(#[test]fn $id(){check($name);})*};}
cases!(running_example=>"running-example-xes",receipt=>"receipt-xes",roadtraffic=>"roadtraffic100traces-xes",even=>"interleavings-receipt_even-csv",odd=>"interleavings-receipt_odd-csv",empty=>"empty",empty_traces=>"empty-traces",single=>"single",sequence=>"sequence",xor=>"xor",parallel=>"parallel",self_loop=>"self-loop",short_loop=>"short-loop",nested=>"nested",rigid=>"rigid",custom_key=>"custom-key",overlap=>"overlap",or_lifecycle=>"or-lifecycle",lifecycle_sorting=>"lifecycle-sorting",lifecycle_fallback=>"lifecycle-fallback",start_only=>"start-only",ignored_lifecycle=>"ignored-lifecycle",custom_lifecycle=>"custom-lifecycle");
#[test]
fn invalid_options_and_synthetic_labels() {
    let keys = EventKeys::default();
    for value in [f64::NAN, f64::INFINITY, -1.0, 1.1] {
        assert!(
            bpmn_split_miner(
                &EventLog::default(),
                &keys,
                &SplitMinerOptions {
                    epsilon: value,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            bpmn_split_miner(
                &EventLog::default(),
                &keys,
                &SplitMinerOptions {
                    eta: value,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let mut t = Trace::new();
    for a in ["__start__", "xor_1", "__end__"] {
        let mut e = Event::new();
        e.attributes.insert("concept:name", a);
        t.events.push(e);
    }
    let log = EventLog {
        traces: vec![t],
        ..Default::default()
    };
    let b = bpmn_split_miner(&log, &keys, &Default::default()).unwrap();
    assert_eq!(b.task_labels().len(), 3);
}

#[test]
fn seeded_graphs() {
    for seed in 0..8 {
        check(&format!("seeded-{seed}"));
    }
}

#[test]
fn core_errors_and_sm2_ignored_settings() {
    let keys = EventKeys::default();
    let mut trace = Trace::new();
    trace.events.push(Event::new());
    let mut log = EventLog {
        traces: vec![trace],
        ..Default::default()
    };
    assert!(matches!(
        bpmn_split_miner(&log, &keys, &Default::default()),
        Err(ichnos_discovery::Error::Core(_))
    ));
    log.traces[0].events[0]
        .attributes
        .insert("concept:name", "a");
    let options = SplitMinerOptions {
        variant: SplitMinerVariant::Sm2,
        ..Default::default()
    };
    let normal = discover_split_miner(&log, &keys, &options).unwrap();
    let ignored = discover_split_miner(
        &log,
        &keys,
        &SplitMinerOptions {
            eta: f64::NAN,
            minimize_or_joins: false,
            ..options.clone()
        },
    )
    .unwrap();
    assert!(isomorphic(
        &graph(&normal.bpmn, &normal.looped_tasks),
        &graph(&ignored.bpmn, &ignored.looped_tasks)
    ));
    log.traces[0].events[0]
        .attributes
        .insert("time:timestamp", "bad");
    assert!(matches!(
        bpmn_split_miner(&log, &keys, &options),
        Err(ichnos_discovery::Error::Core(
            ichnos_core::Error::AttributeType {
                position: ichnos_core::Position::Event { trace: 0, event: 0 },
                ..
            }
        ))
    ));
    assert!(bpmn_split_miner(&log, &keys, &Default::default()).is_ok());
}

#[test]
fn submicroseconds() {
    check("submicroseconds");
}

#[test]
fn lifecycle_or_split_promotion() {
    check("or-promotion");
    let g = golden("discovery", "split-miner-or-promotion");
    let mut promoted = 0;
    for row in g.expected.as_array().unwrap() {
        if row["options"]["variant"] == "sm2" && row["options"]["epsilon"].as_f64().unwrap() <= 0.1
        {
            let nodes = row["graph"]["nodes"].as_array().unwrap();
            let edges = row["graph"]["edges"].as_array().unwrap();
            let inclusive: Vec<_> = nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node[0] == "or")
                .map(|(i, _)| i)
                .collect();
            assert_eq!(
                inclusive.len(),
                2,
                "native SM2 must promote the split and its join"
            );
            assert!(inclusive.iter().any(|&id| {
                edges
                    .iter()
                    .filter(|edge| edge[0].as_u64().unwrap() as usize == id)
                    .count()
                    == 3
            }));
            assert!(inclusive.iter().any(|&id| {
                edges
                    .iter()
                    .filter(|edge| edge[1].as_u64().unwrap() as usize == id)
                    .count()
                    == 3
            }));
            promoted += 1;
        }
    }
    assert_eq!(promoted, 2);
}
