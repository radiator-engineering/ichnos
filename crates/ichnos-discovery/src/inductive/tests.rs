use ichnos_core::{EventKeys, EventLog};
use ichnos_model::ProcessTree;

use super::cuts::{concurrency_cut, loop_cut, sequence_cut, strict_sequence_cut, xor_cut};
use super::data::{Dfg, Group, Uvcl};
use super::miner::filter_noise;
use super::*;

/// A variant log over activities `a` = 0, `b` = 1, ...
fn uvcl(traces: &[(&str, u64)]) -> Uvcl {
    let mut log = Uvcl::new();
    for &(t, n) in traces {
        add_trace(
            &mut log,
            t.bytes().map(|c| Act::from(c - b'a')).collect(),
            n,
        );
    }
    log
}

fn group(acts: &str) -> Group {
    acts.bytes().map(|c| Act::from(c - b'a')).collect()
}

fn mine(traces: &[&str], options: &InductiveOptions) -> ProcessTree {
    let keys = EventKeys::default();
    let log = EventLog::from_trace_strings(traces.iter().copied(), ",", &keys);
    process_tree_inductive(&log, &keys, options).unwrap()
}

fn im(traces: &[&str]) -> ProcessTree {
    mine(traces, &InductiveOptions::default())
}

/// Asserts that `actual` equals pm4py's tree for the same input, up to the
/// order of XOR and parallel children. The expected strings come from
/// running pm4py 2.7.23.8 on the same traces.
#[track_caller]
fn assert_pm4py(actual: &ProcessTree, pm4py: &str) {
    let expected = ProcessTree::parse(pm4py).unwrap();
    assert!(
        actual.structurally_language_equal(&expected),
        "{actual} differs from pm4py's {pm4py}"
    );
}

#[test]
fn base_cases() {
    assert_eq!(im(&[]), ProcessTree::Tau);
    assert_eq!(im(&["a", "a"]), ProcessTree::activity("a"));
}

#[test]
fn xor_groups_are_connected_components() {
    // Largest group first, then by smallest activity.
    let dfg = Dfg::from_log(&uvcl(&[("ab", 1), ("c", 1), ("de", 1)]));
    assert_eq!(
        xor_cut(&dfg),
        Some(vec![group("ab"), group("de"), group("c")])
    );
    assert_pm4py(
        &im(&["a,b", "c", "d,e"]),
        "X( 'c', ->( 'a', 'b' ), ->( 'd', 'e' ) )",
    );
}

#[test]
fn sequence_groups_follow_reachability() {
    // b and c never reach each other, so they share a group.
    let dfg = Dfg::from_log(&uvcl(&[("abd", 1), ("acd", 1)]));
    assert_eq!(
        sequence_cut(&dfg),
        Some(vec![group("a"), group("bc"), group("d")])
    );
    assert_eq!(strict_sequence_cut(&dfg), sequence_cut(&dfg));
}

#[test]
fn strict_sequence_merges_skippable_groups() {
    // b and c can each be skipped, but only together: a c b d never occurs.
    let log = uvcl(&[("abcd", 1), ("ad", 1)]);
    let dfg = Dfg::from_log(&log);
    assert_eq!(
        sequence_cut(&dfg),
        Some(vec![group("a"), group("b"), group("c"), group("d")])
    );
    assert_eq!(
        strict_sequence_cut(&dfg),
        Some(vec![group("a"), group("bc"), group("d")])
    );
    assert_pm4py(
        &im(&["a,b,c,d", "a,d"]),
        "->( 'a', X( tau, ->( 'b', 'c' ) ), 'd' )",
    );
}

#[test]
fn concurrency_needs_edges_both_ways() {
    let dfg = Dfg::from_log(&uvcl(&[("ab", 1), ("ba", 1)]));
    assert_eq!(concurrency_cut(&dfg), Some(vec![group("a"), group("b")]));
    // c starts no trace, so its group joins the one before it.
    let traces = ["a,b,c", "a,c,b", "b,a,c", "b,c,a"];
    let dfg = Dfg::from_log(&uvcl(&[("abc", 1), ("acb", 1), ("bac", 1), ("bca", 1)]));
    assert_eq!(concurrency_cut(&dfg), Some(vec![group("a"), group("bc")]));
    assert_pm4py(&im(&traces), "+( 'a', 'c', 'b' )");
}

#[test]
fn loop_splits_do_and_redo() {
    let dfg = Dfg::from_log(&uvcl(&[("ab", 1), ("abcab", 1)]));
    assert_eq!(loop_cut(&dfg), Some(vec![group("ab"), group("c")]));
    assert_pm4py(&im(&["a,b", "a,b,c,a,b"]), "*( ->( 'a', 'b' ), 'c' )");
}

#[test]
fn empty_traces_become_a_skip() {
    let log = uvcl(&[("", 1), ("a", 3)]);
    let labels = [Label::from("a")];
    let miner = InductiveOptions::default().miner(&labels);
    assert_eq!(miner.im(log.clone()).fold().to_string(), "X( tau, 'a' )");
    // IMf drops the empty trace when it is at most the noise fraction.
    assert_eq!(miner.imf(log.clone(), 0.25).fold().to_string(), "a");
    assert_eq!(miner.imf(log, 0.2).fold().to_string(), "X( tau, 'a' )");
    // IMd keeps the skip it is given.
    let dfg = Dfg::from_log(&uvcl(&[("a", 1)]));
    assert_eq!(miner.imd(dfg, true).fold().to_string(), "X( tau, 'a' )");
}

#[test]
fn fall_throughs() {
    // Activity once per trace: b and c occur once in every trace; b is
    // taken first.
    let traces = ["a,c,b,a", "b,c"];
    assert_pm4py(&im(&traces), "+( X( tau, *( 'a', tau ) ), 'c', 'b' )");
    // The flower model when the other fall-throughs are off.
    let options = InductiveOptions::default().with_fallthroughs_disabled(true);
    assert_pm4py(&mine(&traces, &options), "*( tau, X( 'a', 'c', 'b' ) )");
    // Strict tau loop.
    assert_pm4py(&im(&["a,a,a"]), "*( 'a', tau )");
}

#[test]
fn imf_filters_infrequent_edges() {
    // The rare b -> a edge hides the sequence; IM needs a tau loop.
    let mut traces = vec!["a,b"; 99];
    traces.push("a,b,a,b");
    assert_pm4py(&im(&traces), "*( ->( 'a', 'b' ), tau )");
    let dfg = Dfg::from_log(&uvcl(&[("ab", 99), ("abab", 1)]));
    let filtered = filter_noise(&dfg, 0.2);
    assert_eq!(filtered.graph.keys().collect::<Vec<_>>(), [&(0, 1)]);
    let options = InductiveOptions::new(InductiveVariant::Imf {
        noise_threshold: 0.2,
    });
    assert_pm4py(&mine(&traces, &options), "->( 'a', *( 'b', tau ) )");
}

#[test]
fn imd_on_a_dfg() {
    let mut dfg = ichnos_model::Dfg::new();
    dfg.add_edge("a", "b", 1);
    dfg.add_edge("a", "c", 1);
    dfg.add_start("a", 2);
    dfg.add_end("b", 1);
    dfg.add_end("c", 1);
    let tree = process_tree_inductive_dfg(&dfg, &InductiveOptions::default());
    assert_pm4py(&tree, "->( 'a', X( 'c', 'b' ) )");
    // An end activity alone gives that activity, where pm4py fails.
    let mut dfg = ichnos_model::Dfg::new();
    dfg.add_end("a", 1);
    let tree = process_tree_inductive_dfg(&dfg, &InductiveOptions::default());
    assert_eq!(tree, ProcessTree::activity("a"));
}

#[test]
fn noise_threshold_must_be_a_fraction() {
    for noise_threshold in [-0.1, 1.5, f64::NAN] {
        let options = InductiveOptions::new(InductiveVariant::Imf { noise_threshold });
        let keys = EventKeys::default();
        let log = EventLog::from_trace_strings(["a"], ",", &keys);
        assert!(matches!(
            process_tree_inductive(&log, &keys, &options),
            Err(Error::NoiseThreshold(_))
        ));
    }
}

#[test]
fn options_follow_pm4py_defaults() {
    assert_eq!(
        InductiveOptions::from_noise_threshold(0.0).variant,
        InductiveVariant::Im
    );
    let options = InductiveOptions::default();
    assert!(!options.disable_fallthroughs && !options.disable_strict_sequence_cut);
}
