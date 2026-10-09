use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::*;
use crate::conversion::tests::{Language, Trace, net_language, tree_language};
use crate::conversion::{BpmnToPetriOptions, UnsupportedOperator};
use crate::{Operator, ProcessTree};

fn tree(s: &str) -> ProcessTree {
    ProcessTree::parse(s).unwrap()
}

fn kinds(bpmn: &Bpmn) -> BTreeMap<&'static str, usize> {
    let mut out = BTreeMap::new();
    for (_, n) in bpmn.nodes() {
        *out.entry(n.kind.class_name()).or_default() += 1;
    }
    out
}

/// `(source name or class, target name or class)` for every flow.
fn flows(bpmn: &Bpmn) -> BTreeSet<(String, String)> {
    let show = |n: NodeId| {
        let n = bpmn.node(n);
        if n.name.is_empty() {
            n.kind.class_name().to_owned()
        } else {
            n.name.clone()
        }
    };
    bpmn.flows()
        .map(|(_, f)| (show(f.source()), show(f.target())))
        .collect()
}

/// The traces of tasks that reach an end event of the top-level process,
/// with at most `max` tasks, under the token semantics.
fn bpmn_language(bpmn: &Bpmn, max: usize) -> Language {
    let mut out = Language::new();
    let start = (bpmn.initial_marking(), Trace::new());
    let mut seen = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([start]);
    while let Some((m, trace)) = queue.pop_front() {
        if m.keys().any(|&n| {
            let n = bpmn.node(n);
            n.kind.is_end_event() && n.process == bpmn.process_id
        }) {
            out.insert(trace.clone());
        }
        for n in bpmn.enabled_nodes(&m) {
            let node = bpmn.node(n);
            let mut next_trace = trace.clone();
            if node.kind.is_task() {
                if trace.len() == max {
                    continue;
                }
                next_trace.push(crate::Label::from(node.name.as_str()));
            }
            for next in bpmn.fire(n, &m).unwrap() {
                let state = (next, next_trace.clone());
                if seen.insert(state.clone()) {
                    queue.push_back(state);
                }
            }
        }
    }
    out
}

const TREES: &[&str] = &[
    "'a'",
    "tau",
    "->( 'a', 'b', 'c' )",
    "->( 'a', +( 'b', 'c' ), 'd' )",
    "X( 'a', ->( 'b', 'c' ), tau )",
    "*( X( 'a', tau ), 'b' )",
    "->( tau, 'a', tau )",
    "*( ->( 'a', 'b' ), tau )",
    "+( 'a', ->( 'b', X( 'c', tau ) ) )",
    "->( 'a', ->( 'b', 'c' ), X( 'd', +( 'e', 'f' ) ) )",
];

#[test]
fn tree_to_bpmn_builds_gateways_and_tasks() {
    let b = tree("->( 'a', X( 'b', tau ), +( 'c', 'd' ) )")
        .to_bpmn()
        .unwrap();
    assert_eq!(
        kinds(&b),
        BTreeMap::from([
            ("ExclusiveGateway", 2),
            ("NormalEndEvent", 1),
            ("ParallelGateway", 2),
            ("StartEvent", 1),
            ("Task", 4),
        ])
    );
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        flows(&b),
        BTreeSet::from([
            s("start", "a"),
            s("a", "ExclusiveGateway"),
            s("ExclusiveGateway", "b"),
            s("b", "ExclusiveGateway"),
            // The tau branch becomes a flow from split to join.
            s("ExclusiveGateway", "ExclusiveGateway"),
            s("ExclusiveGateway", "ParallelGateway"),
            s("ParallelGateway", "c"),
            s("ParallelGateway", "d"),
            s("c", "ParallelGateway"),
            s("d", "ParallelGateway"),
            s("ParallelGateway", "end"),
        ])
    );
    assert_eq!(b.flow_count(), 11);
    let dirs: BTreeSet<_> = b
        .nodes()
        .filter_map(|(_, n)| n.kind.gateway_direction())
        .collect();
    assert_eq!(
        dirs,
        BTreeSet::from([GatewayDirection::Diverging, GatewayDirection::Converging])
    );
}

#[test]
fn tree_to_bpmn_keeps_the_language() {
    for s in TREES {
        let t = tree(s);
        let b = t.to_bpmn().unwrap();
        let expected = tree_language(&t, 6);
        assert_eq!(bpmn_language(&b, 6), expected, "token game on {s}");
        let apn = b.to_petri_net(BpmnToPetriOptions::default()).net;
        assert_eq!(net_language(&apn, 6), expected, "Petri net of {s}");
    }
}

#[test]
fn one_child_sequence_reaches_the_end() {
    let t = ProcessTree::sequence([ProcessTree::activity("a")]);
    let b = t.to_bpmn().unwrap();
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(flows(&b), BTreeSet::from([s("start", "a"), s("a", "end")]));
}

#[test]
fn interleaving_is_not_converted() {
    let t = tree("<>( 'a', 'b' )");
    assert_eq!(
        t.to_bpmn(),
        Err(UnsupportedOperator(Operator::Interleaving))
    );
}

#[test]
fn petri_to_bpmn_round_trips_the_language() {
    for s in TREES {
        let t = tree(s);
        let apn = t.to_petri_net();
        let b = apn.to_bpmn();
        let back = b.to_petri_net(BpmnToPetriOptions::default()).net;
        assert_eq!(
            net_language(&back, 6),
            tree_language(&t, 6),
            "round trip of {s}"
        );
    }
}

#[test]
fn petri_to_bpmn_maps_places_and_transitions() {
    // p1 -> a -> p2 -> (b and c) -> p3, p4 -> silent join -> p5.
    let mut net = crate::PetriNet::new("n");
    let p: Vec<_> = (1..=5).map(|i| net.add_place(format!("p{i}"))).collect();
    let a = net.add_transition("a", Some(crate::Label::from("a")));
    let split = net.add_transition("s", None::<crate::Label>);
    let join = net.add_transition("j", None::<crate::Label>);
    net.add_input_arc(p[0], a).unwrap();
    net.add_output_arc(a, p[1]).unwrap();
    net.add_input_arc(p[1], split).unwrap();
    net.add_output_arc(split, p[2]).unwrap();
    net.add_output_arc(split, p[3]).unwrap();
    net.add_input_arc(p[2], join).unwrap();
    net.add_input_arc(p[3], join).unwrap();
    net.add_output_arc(join, p[4]).unwrap();
    let im = [(p[0], 1)].into_iter().collect();
    let fm = [(p[4], 1)].into_iter().collect();
    let b = crate::AcceptingPetriNet::new(net, im, fm).to_bpmn();
    // Every exclusive gateway had one flow in and one out, so all are gone.
    assert_eq!(
        kinds(&b),
        BTreeMap::from([
            ("NormalEndEvent", 1),
            ("ParallelGateway", 2),
            ("StartEvent", 1),
            ("Task", 1),
        ])
    );
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        flows(&b),
        BTreeSet::from([
            s("start", "a"),
            s("a", "ParallelGateway"),
            s("ParallelGateway", "ParallelGateway"),
            s("ParallelGateway", "end"),
        ])
    );
    assert_eq!(b.flow_count(), 5, "the split and join share two flows");
}

#[test]
fn bpmn_to_petri_net_names_follow_pm4py() {
    let mut b = Bpmn::new("main");
    let s = b.add_node_with_id("s", NodeKind::start_event(), "start");
    let a = b.add_node_with_id("a", NodeKind::task(), "do a");
    let e = b.add_node_with_id("e", NodeKind::end_event(), "end");
    b.add_flow_with(FlowKind::Sequence, "f1", "", s, a).unwrap();
    b.add_flow_with(FlowKind::Sequence, "f2", "", a, e).unwrap();
    let raw = b.to_petri_net(BpmnToPetriOptions {
        use_id: false,
        reduce: false,
    });
    let net = &raw.net.net;
    let mut places: Vec<&str> = net.places().map(|(_, p)| p.name.as_str()).collect();
    places.sort_unstable();
    assert_eq!(
        places,
        [
            "ent_a", "ent_e", "ent_s", "exi_a", "exi_e", "exi_s", "f1", "f2", "sink", "source"
        ]
    );
    let mut transitions: Vec<(&str, Option<&str>)> = net
        .transitions()
        .map(|(_, t)| (t.name.as_str(), t.label.as_ref().map(|l| l.as_str())))
        .collect();
    transitions.sort_unstable();
    assert_eq!(
        transitions,
        [
            ("a", Some("do a")),
            ("e", None),
            ("e_end", None),
            ("s", None),
            ("s_start", None),
            ("sfl_f1", None),
            ("sfl_f2", None),
            ("tfl_f1", None),
            ("tfl_f2", None),
        ]
    );
    assert_eq!(raw.flow_places.len(), 2);
    assert_eq!(raw.node_transitions[&a].len(), 3, "a, tfl_f1 and sfl_f2");
    assert_eq!(
        net_language(&raw.net, 3),
        Language::from([vec![crate::Label::from("do a")]])
    );

    let ids = b.to_petri_net(BpmnToPetriOptions {
        use_id: true,
        reduce: true,
    });
    let labels: BTreeSet<&str> = ids
        .net
        .net
        .transitions()
        .filter_map(|(_, t)| t.label.as_ref().map(|l| l.as_str()))
        .collect();
    assert_eq!(labels, BTreeSet::from(["a", "e", "s"]));
    // Reduction leaves only the three labelled transitions.
    assert_eq!(ids.net.net.transition_count(), 3);
    assert!(ids.flow_places.is_empty(), "flow places were reduced away");
}

#[test]
fn inclusive_gateways_get_skip_transitions() {
    let t = tree("O( 'a', 'b' )");
    let b = t.to_bpmn().unwrap();
    assert_eq!(kinds(&b)["InclusiveGateway"], 2);
    let raw = b.to_petri_net(BpmnToPetriOptions {
        use_id: false,
        reduce: false,
    });
    let skips = raw
        .net
        .net
        .transitions()
        .filter(|(_, t)| t.name.ends_with("_skip"))
        .count();
    assert_eq!(skips, 2, "one skip per branch after the split");
    let lang = net_language(&raw.net, 4);
    let tr = |s: &str| -> Trace {
        s.chars()
            .map(|c| crate::Label::from(c.to_string()))
            .collect()
    };
    for w in ["a", "b", "ab", "ba"] {
        assert!(lang.contains(&tr(w)), "{w} is missing from {lang:?}");
    }
    // The token game fires any non-empty set of branches, but as in pm4py
    // the converging inclusive gateway waits for a token on every incoming
    // flow, so only runs that take both branches end.
    assert_eq!(bpmn_language(&b, 4), Language::from([tr("ab"), tr("ba")]));
}

#[test]
fn removing_a_node_removes_its_flows() {
    let mut b = Bpmn::default();
    let x = b.add_node(NodeKind::task(), "x");
    let y = b.add_node(NodeKind::task(), "y");
    let z = b.add_node(NodeKind::task(), "z");
    b.add_flow(x, y).unwrap();
    let f = b.add_flow(y, z).unwrap();
    b.remove_node(y);
    assert_eq!(b.node_count(), 2);
    assert_eq!(b.flow_count(), 0);
    assert!(b.node(x).out_flows().is_empty());
    assert!(!b.contains_flow(f));
    assert_eq!(b.add_flow(x, y), Err(BpmnError::NoSuchNode(y)));
    assert_eq!(b.node(z).id, "id_2");
    assert_eq!(b.node_by_id("id_0"), Some(x));
}

fn gw(kind: GatewayKind) -> NodeKind {
    NodeKind::gateway(kind, GatewayDirection::Unspecified)
}

#[test]
fn xor_reduction_splices_single_flow_gateways() {
    let mut b = Bpmn::default();
    let a = b.add_node(NodeKind::task(), "a");
    let g1 = b.add_node(gw(GatewayKind::Exclusive), "");
    let g2 = b.add_node(gw(GatewayKind::Exclusive), "");
    let c = b.add_node(NodeKind::task(), "c");
    let par = b.add_node(gw(GatewayKind::Parallel), "");
    let lone = b.add_node(gw(GatewayKind::Exclusive), "");
    b.add_flow(a, g1).unwrap();
    b.add_flow(g1, g2).unwrap();
    b.add_flow(g2, c).unwrap();
    b.add_flow(c, par).unwrap();
    b.add_flow(par, a).unwrap();
    // A gateway whose only flow is a self-loop stays (pm4py loops forever).
    b.add_flow(lone, lone).unwrap();
    b.reduce(false);
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        flows(&b),
        BTreeSet::from([
            s("a", "c"),
            s("c", "ParallelGateway"),
            s("ParallelGateway", "a"),
            s("ExclusiveGateway", "ExclusiveGateway"),
        ])
    );
    assert!(b.contains_node(lone));
    // Trivial-gateway removal drops the parallel gateway and the lone one.
    b.remove_trivial_gateways();
    assert_eq!(flows(&b), BTreeSet::from([s("a", "c"), s("c", "a")]));
}

#[test]
fn nested_gateways_collapse() {
    // X-split -> X-split -> {b, c}, plus a; joins mirror it.
    let mut b = Bpmn::default();
    let start = b.add_node(NodeKind::start_event(), "start");
    let s1 = b.add_node(gw(GatewayKind::Exclusive), "");
    let s2 = b.add_node(gw(GatewayKind::Exclusive), "");
    let j2 = b.add_node(gw(GatewayKind::Exclusive), "");
    let j1 = b.add_node(gw(GatewayKind::Exclusive), "");
    let end = b.add_node(NodeKind::end_event(), "end");
    let tasks: Vec<NodeId> = ["a", "b", "c"]
        .iter()
        .map(|n| b.add_node(NodeKind::task(), *n))
        .collect();
    b.add_flow(start, s1).unwrap();
    b.add_flow(s1, tasks[0]).unwrap();
    b.add_flow(s1, s2).unwrap();
    b.add_flow(s2, tasks[1]).unwrap();
    b.add_flow(s2, tasks[2]).unwrap();
    b.add_flow(tasks[1], j2).unwrap();
    b.add_flow(tasks[2], j2).unwrap();
    b.add_flow(j2, j1).unwrap();
    b.add_flow(tasks[0], j1).unwrap();
    b.add_flow(j1, end).unwrap();
    b.reduce(true);
    assert_eq!(b.node_count(), 7);
    assert!(!b.contains_node(s2) && !b.contains_node(j2));
    assert_eq!(b.node(s1).out_flows().len(), 3);
    assert_eq!(b.node(j1).in_flows().len(), 3);
}

#[test]
fn subprocess_end_returns_to_the_parent() {
    let mut b = Bpmn::new("main");
    let start = b.add_node(NodeKind::start_event(), "start");
    let sub = b.add_node_with_id("sub", NodeKind::SubProcess { depth: Some(1) }, "sub");
    let end = b.add_node(NodeKind::end_event(), "end");
    let inner_start = b.add_node(NodeKind::start_event(), "s");
    let x = b.add_node(NodeKind::task(), "x");
    let inner_end = b.add_node(NodeKind::end_event(), "e");
    for n in [inner_start, x, inner_end] {
        b.node_mut(n).process = "sub".into();
    }
    b.add_flow(start, sub).unwrap();
    b.add_flow(sub, end).unwrap();
    b.add_flow(inner_start, x).unwrap();
    b.add_flow(x, inner_end).unwrap();

    assert_eq!(b.global_start_events(), vec![start]);
    assert_eq!(b.start_events_of_subprocess("sub"), vec![inner_start]);
    assert_eq!(
        b.processes_deep(x),
        vec!["sub".to_owned(), "main".to_owned()]
    );
    assert_eq!(
        b.nodes_inside_process("sub", true),
        vec![inner_start, x, inner_end]
    );

    let m = b.initial_marking();
    let m = b.fire(start, &m).unwrap().remove(0);
    assert_eq!(m, BpmnMarking::from([(inner_start, 1)]));
    let m = b.fire(inner_start, &m).unwrap().remove(0);
    let m = b.fire(x, &m).unwrap().remove(0);
    assert_eq!(m, BpmnMarking::from([(inner_end, 1)]));
    let m = b.fire(inner_end, &m).unwrap().remove(0);
    assert_eq!(m, BpmnMarking::from([(end, 1)]));
    assert!(b.enabled_nodes(&m).is_empty());
    assert_eq!(b.fire(x, &m), Err(NodeNotEnabled(x)));
}

#[test]
fn terminate_end_event_clears_other_tokens() {
    let mut b = Bpmn::new("main");
    let split = b.add_node(
        NodeKind::gateway(GatewayKind::Parallel, GatewayDirection::Diverging),
        "",
    );
    let a = b.add_node(NodeKind::task(), "a");
    let c = b.add_node(NodeKind::task(), "c");
    let stop = b.add_node(NodeKind::EndEvent(EndTrigger::Terminate), "stop");
    b.add_flow(split, a).unwrap();
    b.add_flow(split, c).unwrap();
    b.add_flow(a, stop).unwrap();
    let m = b.fire(split, &BpmnMarking::from([(split, 1)])).unwrap();
    assert_eq!(m, vec![BpmnMarking::from([(a, 1), (c, 1)])]);
    let m = b.fire(a, &m[0]).unwrap();
    assert_eq!(m, vec![BpmnMarking::from([(stop, 1)])]);
}

#[test]
fn converging_parallel_gateway_waits_for_every_flow() {
    let mut b = Bpmn::default();
    let x = b.add_node(NodeKind::task(), "x");
    let y = b.add_node(NodeKind::task(), "y");
    let join = b.add_node(
        NodeKind::gateway(GatewayKind::Parallel, GatewayDirection::Converging),
        "",
    );
    b.add_flow(x, join).unwrap();
    b.add_flow(y, join).unwrap();
    assert!(!b.is_enabled(join, &BpmnMarking::from([(join, 1)])));
    assert!(b.is_enabled(join, &BpmnMarking::from([(join, 2)])));
    // An unspecified gateway that is not converging gives no marking.
    let g = b.add_node(gw(GatewayKind::Exclusive), "");
    assert!(b.weak_fire(g, &BpmnMarking::from([(g, 1)])).is_empty());
}

#[test]
fn utilities() {
    let mut b = tree("->( 'a', X( 'b', 'c' ) )").to_bpmn().unwrap();
    let levels = b.bfs_levels();
    let level_of = |name: &str| {
        let (id, _) = b.nodes().find(|(_, n)| n.name == name).unwrap();
        levels[&id]
    };
    assert_eq!(level_of("start"), 0);
    assert_eq!(level_of("a"), 1);
    assert_eq!(level_of("b"), 3);
    // End events are never visited, so they come last.
    assert_eq!(level_of("end"), 6);
    let (nodes, flows_sorted) = b.sorted_nodes_and_flows();
    assert_eq!(b.node(nodes[0]).name, "start");
    assert_eq!(flows_sorted.len(), b.flow_count());

    b.replace_task_labels(&BTreeMap::from([("a".to_owned(), "A".to_owned())]));
    assert_eq!(
        b.task_labels(),
        BTreeSet::from(["A".into(), "b".into(), "c".into()])
    );
}

#[test]
fn class_names_follow_pm4py() {
    assert_eq!(NodeKind::start_event().class_name(), "StartEvent");
    assert_eq!(NodeKind::end_event().class_name(), "NormalEndEvent");
    assert_eq!(
        NodeKind::BoundaryEvent {
            trigger: CatchTrigger::Message,
            activity: None
        }
        .class_name(),
        "MessageBoundaryEvent"
    );
    assert_eq!(
        gw(GatewayKind::EventBased).class_name(),
        "EventBasedGateway"
    );
    assert!(NodeKind::SubProcess { depth: None }.is_activity());
    assert!(!NodeKind::Collaboration.is_event());
}
