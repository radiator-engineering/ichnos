use super::*;
use crate::conversion::tests::{Language, Trace, net_language};

fn l(s: &str) -> Label {
    Label::from(s)
}

fn trace(s: &str) -> Trace {
    s.chars().map(|c| Label::from(c.to_string())).collect()
}

/// The DFG of 5 x `abcd` and 5 x `acbd`.
fn diamond_dfg() -> Dfg {
    let mut dfg = Dfg::new();
    for (a, b) in [
        ("a", "b"),
        ("a", "c"),
        ("b", "c"),
        ("c", "b"),
        ("b", "d"),
        ("c", "d"),
    ] {
        dfg.add_edge(a, b, 5);
    }
    dfg.add_start("a", 10);
    dfg.add_end("d", 10);
    dfg
}

/// The heuristics net pm4py discovers for [`diamond_dfg`]: `a` splits into
/// `b` and `c` in parallel, and `d` joins them. `b` and `c` follow each
/// other equally often, so they are not connected.
fn diamond() -> HeuristicsNet {
    let dfg = diamond_dfg();
    let mut h = HeuristicsNet::from_dfg(&dfg);
    for ((a, b), &n) in &dfg.graph {
        h.dfg_matrix
            .entry(a.clone())
            .or_default()
            .insert(b.clone(), n);
    }
    for (a, b) in [("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")] {
        let edge = HeuristicsEdge {
            dependency: 5.0 / 6.0,
            frequency: 5,
        };
        h.add_output_connection(&l(a), &l(b), edge);
        h.add_input_connection(&l(b), &l(a), edge);
    }
    h.calculate_node_measures(DEFAULT_AND_MEASURE_THRESH, DEFAULT_LOOP_LENGTH_TWO_THRESH);
    h
}

#[test]
fn activity_counts_halve_when_both_directions_exist() {
    let dfg = diamond_dfg().graph;
    // a: out 10, no in.
    assert_eq!(activity_count_from_dfg(&dfg, "a"), 10);
    // b: out 10, in 10, halved.
    assert_eq!(activity_count_from_dfg(&dfg, "b"), 10);
    assert_eq!(activity_count_from_dfg(&dfg, "d"), 10);
    assert_eq!(activity_count_from_dfg(&dfg, "z"), 0);
    // Halving rounds down: in 3, out 2.
    let mut dfg = Dfg::new();
    dfg.add_edge("x", "y", 3);
    dfg.add_edge("y", "z", 2);
    assert_eq!(activity_count_from_dfg(&dfg.graph, "y"), 2);
}

#[test]
fn from_dfg_infers_missing_start_and_end() {
    let mut dfg = Dfg::new();
    dfg.add_edge("a", "b", 3);
    let h = HeuristicsNet::from_dfg(&dfg);
    assert_eq!(h.start_activities, vec![BTreeMap::from([(l("a"), 0)])]);
    assert_eq!(h.end_activities, vec![BTreeMap::from([(l("b"), 0)])]);
    assert_eq!(h.activity_occurrences[&l("a")], 3);
    let h = HeuristicsNet::from_dfg(&diamond_dfg());
    assert_eq!(h.start_activities, vec![BTreeMap::from([(l("a"), 10)])]);
}

#[test]
fn and_measures_of_a_parallel_split_and_join() {
    let h = diamond();
    let value = 10.0 / 11.0;
    let pair = Matrix::from([(l("b"), BTreeMap::from([(l("c"), value)]))]);
    assert_eq!(h.nodes[&l("a")].and_measures_out, pair);
    assert_eq!(h.nodes[&l("d")].and_measures_in, pair);
    assert!(h.nodes[&l("a")].and_measures_in.is_empty());
    // Above the measure, no pair qualifies.
    assert!(h.and_measures_out(&l("a"), 0.95).is_empty());
}

#[test]
fn loops_of_length_two() {
    let h = HeuristicsNet {
        freq_triples_matrix: Matrix::from([
            (l("a"), BTreeMap::from([(l("b"), 3), (l("c"), 0)])),
            (l("b"), BTreeMap::from([(l("a"), 1)])),
        ]),
        dfg_matrix: Matrix::from([(l("a"), BTreeMap::from([(l("b"), 7)]))]),
        ..HeuristicsNet::default()
    };
    // a-b: (3 + 1) / 5 = 0.8 qualifies; a-c: 0 does not.
    assert_eq!(
        h.loops_length_two(&l("a"), 0.5),
        BTreeMap::from([(l("b"), 7)])
    );
    assert!(h.loops_length_two(&l("a"), 0.9).is_empty());
}

#[test]
fn parallel_split_becomes_and_split() {
    let apn = diamond().to_petri_net();
    let expected = Language::from([trace("abcd"), trace("acbd")]);
    assert_eq!(net_language(&apn, 6), expected);
}

#[test]
fn without_and_measures_the_split_is_a_choice() {
    let mut h = diamond();
    for node in h.nodes.values_mut() {
        node.and_measures_in.clear();
        node.and_measures_out.clear();
    }
    let apn = h.to_petri_net();
    // b and c are alternatives: after a, one of them puts a token in d's
    // input, which a single d consumes.
    let lang = net_language(&apn, 6);
    assert!(lang.contains(&trace("abd")));
    assert!(lang.contains(&trace("acd")));
    assert!(!lang.contains(&trace("abcd")));
}

#[test]
fn merged_nets_get_one_source_and_sink_each() {
    let h = diamond();
    let merged = h.merge(&h);
    assert_eq!(merged.start_activities.len(), 2);
    assert_eq!(
        merged.nodes[&l("a")].outputs[&l("b")].len(),
        2,
        "edge lists are concatenated"
    );
    let apn = merged.to_petri_net();
    assert_eq!(apn.initial_marking.len(), 2);
    assert_eq!(apn.final_marking.len(), 2);
    assert!(apn.net.place_by_name("source1").is_some());
}
