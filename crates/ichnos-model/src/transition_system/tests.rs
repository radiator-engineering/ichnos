use super::*;
use crate::conversion::EdgeNaming;
use crate::petri::ReachabilityOptions;
use crate::{Label, Marking, PetriNet};

fn names(ts: &TransitionSystem) -> Vec<(String, String, String)> {
    let mut out: Vec<_> = ts
        .edges()
        .map(|(_, e)| {
            (
                ts.state(e.from()).name.clone(),
                e.name.clone(),
                ts.state(e.to()).name.clone(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn edges_are_unique_per_name_and_ends() {
    let mut ts = TransitionSystem::new("t");
    let a = ts.add_state("a");
    let b = ts.add_state("b");
    let e1 = ts.add_edge("x", a, b).unwrap();
    assert_eq!(ts.add_edge("x", a, b).unwrap(), e1);
    let e2 = ts.add_edge("y", a, b).unwrap();
    assert_ne!(e1, e2);
    assert_eq!(ts.edge_count(), 2);
    assert_eq!(ts.state(a).outgoing(), &[e1, e2]);
    assert_eq!(ts.state(b).incoming(), &[e1, e2]);
    assert_eq!(ts.successors(a), vec![b]);
    assert_eq!(ts.state_by_name("b"), Some(b));
    assert_eq!(
        ts.add_edge("z", a, StateId(9)),
        Err(TsError::UnknownState(StateId(9)))
    );
}

#[test]
fn edge_removal() {
    let mut ts = TransitionSystem::new("t");
    let a = ts.add_state("a");
    let b = ts.add_state("b");
    let c = ts.add_state("c");
    let x = ts.add_edge("x", a, b).unwrap();
    ts.add_edge("y", a, b).unwrap();
    let x2 = ts.add_edge("x", b, c).unwrap();
    // Only the edge between a and b goes, not the other edge named x.
    assert!(ts.remove_edge_named("x", a, b));
    assert!(!ts.remove_edge_named("x", a, b));
    assert!(!ts.contains_edge(x));
    assert!(ts.contains_edge(x2));
    assert_eq!(ts.remove_edges_between(a, b), 1);
    assert_eq!(ts.edge_count(), 1);
    assert!(ts.state(a).outgoing().is_empty());
    assert!(ts.state(b).incoming().is_empty());
    ts.remove_edge(x);
    assert_eq!(ts.edge_count(), 1);
}

#[test]
fn transitive_reduction_removes_shortcuts() {
    // a -> b -> c -> d, plus shortcuts a -> c, a -> d and b -> d.
    let mut ts = TransitionSystem::new("t");
    let [a, b, c, d] = ["a", "b", "c", "d"].map(|n| ts.add_state(n));
    for (x, y) in [(a, b), (b, c), (c, d), (a, c), (a, d), (b, d)] {
        ts.add_edge("e", x, y).unwrap();
    }
    // Two parallel edges on the chain stay.
    ts.add_edge("f", a, b).unwrap();
    ts.transitive_reduction().unwrap();
    let edges: Vec<(StateId, StateId)> = ts.edges().map(|(_, e)| (e.from(), e.to())).collect();
    assert_eq!(edges, vec![(a, b), (b, c), (c, d), (a, b)]);
}

#[test]
fn transitive_reduction_rejects_cycles() {
    let mut ts = TransitionSystem::new("t");
    let a = ts.add_state("a");
    let b = ts.add_state("b");
    ts.add_edge("x", a, b).unwrap();
    assert!(ts.topological_order().is_some());
    ts.add_edge("y", b, a).unwrap();
    assert_eq!(ts.transitive_reduction(), Err(TsError::Cyclic));
    let mut ts = TransitionSystem::new("self");
    let a = ts.add_state("a");
    ts.add_edge("x", a, a).unwrap();
    assert_eq!(ts.topological_order(), None);
}

#[test]
fn reachability_graph_as_transition_system() {
    // source -a-> p 1 -tau-> sink, and a puts two tokens in "p 1".
    let mut net = PetriNet::new("n");
    let source = net.add_place("source");
    let p = net.add_place("p 1");
    let sink = net.add_place("sink");
    let a = net.add_transition("ta", Some("a"));
    let tau = net.add_transition("skip", None::<Label>);
    net.add_input_arc(source, a).unwrap();
    net.add_arc(
        crate::petri::ArcEnds::TransitionToPlace(a, p),
        2,
        crate::petri::ArcKind::Normal,
    )
    .unwrap();
    net.add_input_arc(p, tau).unwrap();
    net.add_output_arc(tau, sink).unwrap();
    let im = Marking::from([(source, 1)]);

    let ts = net
        .to_transition_system(&im, ReachabilityOptions::default(), EdgeNaming::Repr)
        .unwrap();
    assert_eq!(ts.state_count(), 4);
    assert_eq!(ts.state(StateId(0)).name, "source1");
    assert_eq!(
        names(&ts),
        [
            ("p11sink1".into(), "(skip, None)".into(), "sink2".into()),
            ("p12".into(), "(skip, None)".into(), "p11sink1".into()),
            ("source1".into(), "(ta, 'a')".into(), "p12".into()),
        ]
    );

    let ts = net
        .to_transition_system(&im, ReachabilityOptions::default(), EdgeNaming::Name)
        .unwrap();
    assert_eq!(names(&ts)[2].1, "ta");

    let err = net
        .to_transition_system(
            &im,
            ReachabilityOptions { max_markings: 2 },
            EdgeNaming::Repr,
        )
        .unwrap_err();
    assert_eq!(err, crate::petri::ReachabilityError::TooManyMarkings(2));
}

/// `a` empties `p1`; `b` needs `p2` and an empty `p1` (inhibitor arc).
pub(crate) fn inhibited_net() -> (PetriNet, Marking) {
    let mut net = PetriNet::new("inhibitor");
    let p1 = net.add_place("p1");
    let p2 = net.add_place("p2");
    let d1 = net.add_place("done1");
    let d2 = net.add_place("done2");
    let a = net.add_transition("ta", Some("a"));
    let b = net.add_transition("tb", Some("b"));
    net.add_input_arc(p1, a).unwrap();
    net.add_output_arc(a, d1).unwrap();
    net.add_input_arc(p2, b).unwrap();
    net.add_output_arc(b, d2).unwrap();
    net.add_arc(
        crate::petri::ArcEnds::PlaceToTransition(p1, b),
        1,
        crate::petri::ArcKind::Inhibitor,
    )
    .unwrap();
    (net, Marking::from([(p1, 1), (p2, 1)]))
}

#[test]
fn reachability_graph_honours_inhibitor_arcs() {
    // pm4py's ClassicSemantics lets `b` fire first and consume `p1`, giving
    // `p11p21 -b-> done21` instead of the `b` edge below.
    let (net, im) = inhibited_net();
    let ts = net
        .to_transition_system(&im, ReachabilityOptions::default(), EdgeNaming::Repr)
        .unwrap();
    assert_eq!(
        names(&ts),
        [
            (
                "done11p21".into(),
                "(tb, 'b')".into(),
                "done11done21".into()
            ),
            ("p11p21".into(), "(ta, 'a')".into(), "done11p21".into()),
        ]
    );
}

#[test]
fn transitive_reduction_keeps_unrelated_edges_with_the_same_name() {
    // pm4py removes all four edges here.
    let mut ts = TransitionSystem::new("t");
    let [a, b, c, x, y] = ["a", "b", "c", "x", "y"].map(|n| ts.add_state(n));
    for (s, d) in [(a, b), (b, c), (a, c), (x, y)] {
        ts.add_edge("e", s, d).unwrap();
    }
    ts.transitive_reduction().unwrap();
    let edges: Vec<(StateId, StateId)> = ts.edges().map(|(_, e)| (e.from(), e.to())).collect();
    assert_eq!(edges, vec![(a, b), (b, c), (x, y)]);
}
