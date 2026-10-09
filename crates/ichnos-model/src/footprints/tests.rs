use std::collections::BTreeSet;

use super::*;
use crate::petri::{Marking, PetriNet};

fn set(items: &[&str]) -> BTreeSet<Label> {
    items.iter().map(|&s| Label::from(s)).collect()
}

fn pairs(items: &[(&str, &str)]) -> BTreeSet<LabelPair> {
    items
        .iter()
        .map(|&(a, b)| (Label::from(a), Label::from(b)))
        .collect()
}

struct Expected {
    tree: &'static str,
    activities: &'static [&'static str],
    start: &'static [&'static str],
    end: &'static [&'static str],
    always: &'static [&'static str],
    sequence: &'static [(&'static str, &'static str)],
    parallel: &'static [(&'static str, &'static str)],
    skippable: bool,
    min: usize,
    max: usize,
}

/// Expected values computed with pm4py 2.7.23.8 `footprints.apply(tree)`.
const TREE_CASES: &[Expected] = &[
    Expected {
        tree: "->( 'a', +( 'b', 'c' ), 'd' )",
        activities: &["a", "b", "c", "d"],
        start: &["a"],
        end: &["d"],
        always: &["a", "b", "c", "d"],
        sequence: &[("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")],
        parallel: &[("b", "c"), ("c", "b")],
        skippable: false,
        min: 4,
        max: 4,
    },
    Expected {
        tree: "*( X( 'a', tau ), 'b' )",
        activities: &["a", "b"],
        start: &["a", "b"],
        end: &["a", "b"],
        always: &[],
        sequence: &[],
        parallel: &[("a", "b"), ("b", "a"), ("b", "b")],
        skippable: true,
        min: 0,
        max: 1,
    },
    Expected {
        tree: "->( 'a', X( 'b', tau ), O( 'c', 'd' ) )",
        activities: &["a", "b", "c", "d"],
        start: &["a"],
        end: &["c", "d"],
        always: &["a", "c", "d"],
        sequence: &[("a", "b"), ("a", "c"), ("a", "d"), ("b", "c"), ("b", "d")],
        parallel: &[("c", "d"), ("d", "c")],
        skippable: false,
        min: 3,
        max: 4,
    },
    Expected {
        tree: "X( ->( 'a', 'b' ), *( 'c', tau ) )",
        activities: &["a", "b", "c"],
        start: &["a", "c"],
        end: &["b", "c"],
        always: &[],
        sequence: &[("a", "b")],
        parallel: &[("c", "c")],
        skippable: false,
        min: 1,
        max: 2,
    },
    Expected {
        tree: "+( 'a', ->( 'b', X( 'c', tau ) ) )",
        activities: &["a", "b", "c"],
        start: &["a", "b"],
        end: &["a", "b", "c"],
        always: &["a", "b"],
        sequence: &[("b", "c")],
        parallel: &[("a", "b"), ("a", "c"), ("b", "a"), ("c", "a")],
        skippable: false,
        min: 2,
        max: 3,
    },
];

#[test]
fn tree_footprints_match_pm4py() {
    for e in TREE_CASES {
        let fp = ProcessTree::parse(e.tree).unwrap().footprints();
        let expected = TreeFootprints {
            footprints: Footprints {
                activities: set(e.activities),
                start_activities: set(e.start),
                sequence: pairs(e.sequence),
                parallel: pairs(e.parallel),
            },
            end_activities: set(e.end),
            activities_always_happening: set(e.always),
            skippable: e.skippable,
            min_trace_length: e.min,
            max_trace_length_without_loops: e.max,
        };
        assert_eq!(fp, expected, "footprints of {}", e.tree);
    }
}

/// The workflow net of `->( 'a', +( 'b', 'c' ), 'd' )`.
fn parallel_net() -> (PetriNet, Marking) {
    let mut net = PetriNet::new("par");
    let i = net.add_place("i");
    let p1 = net.add_place("p1");
    let p2 = net.add_place("p2");
    let p3 = net.add_place("p3");
    let p4 = net.add_place("p4");
    let o = net.add_place("o");
    let a = net.add_transition("a", Some("a"));
    let b = net.add_transition("b", Some("b"));
    let c = net.add_transition("c", Some("c"));
    let d = net.add_transition("d", Some("d"));
    for (p, t) in [(i, a), (p1, b), (p2, c), (p3, d), (p4, d)] {
        net.add_input_arc(p, t).unwrap();
    }
    for (t, p) in [(a, p1), (a, p2), (b, p3), (c, p4), (d, o)] {
        net.add_output_arc(t, p).unwrap();
    }
    (net, Marking::from([(i, 1)]))
}

/// A workflow net of `*( X( 'a', tau ), 'b' )` with silent start and end.
fn loop_net() -> (PetriNet, Marking) {
    let mut net = PetriNet::new("loop");
    let i = net.add_place("i");
    let p = net.add_place("p");
    let q = net.add_place("q");
    let o = net.add_place("o");
    let start = net.add_transition("start", None::<Label>);
    let a = net.add_transition("a", Some("a"));
    let skip = net.add_transition("skip", None::<Label>);
    let b = net.add_transition("b", Some("b"));
    let end = net.add_transition("end", None::<Label>);
    for (pl, t) in [(i, start), (p, a), (p, skip), (q, b), (q, end)] {
        net.add_input_arc(pl, t).unwrap();
    }
    for (t, pl) in [(start, p), (a, q), (skip, q), (b, p), (end, o)] {
        net.add_output_arc(t, pl).unwrap();
    }
    (net, Marking::from([(i, 1)]))
}

#[test]
fn net_footprints_match_tree_footprints() {
    for (tree, (net, im)) in [
        (TREE_CASES[0].tree, parallel_net()),
        (TREE_CASES[1].tree, loop_net()),
    ] {
        let from_net = net.footprints(&im, ReachabilityOptions::default()).unwrap();
        let from_tree = ProcessTree::parse(tree).unwrap().footprints().footprints;
        assert_eq!(from_net, from_tree, "net footprints for {tree}");
    }
}

#[test]
fn net_footprints_honour_inhibitor_arcs() {
    // pm4py's ClassicSemantics treats the inhibitor arc as a normal arc and
    // gives start activities {a, b} and no sequence.
    let (net, im) = crate::transition_system::tests::inhibited_net();
    let fp = net
        .footprints(&im, crate::petri::ReachabilityOptions::default())
        .unwrap();
    let l = |s: &str| Label::from(s);
    assert_eq!(fp.start_activities, BTreeSet::from([l("a")]));
    assert_eq!(fp.sequence, BTreeSet::from([(l("a"), l("b"))]));
    assert!(fp.parallel.is_empty());
}
