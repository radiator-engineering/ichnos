//! Compares the inductive miner with pm4py's golden output in
//! `fixtures/golden/discovery/inductive-*.json`.
//!
//! For each variant and log, the tree must equal pm4py's tree up to the order
//! of XOR and parallel children, and the footprints of the tree and of its
//! Petri net must equal pm4py's. Fitness and precision are left to the
//! conformance crate.
//!
//! pm4py's IMf tree can depend on Python's hash seed, so each IMf golden
//! holds one run per distinct tree over several seeds. ichnos's tree must
//! equal one of them.

mod common;

use std::collections::BTreeSet;

use common::{canonical, load_csv_log};
use ichnos_core::{EventKeys, EventLog};
use ichnos_discovery::{
    InductiveOptions, InductiveVariant, petri_net_inductive_dfg, process_tree_inductive,
    process_tree_inductive_dfg,
};
use ichnos_golden::{Golden, golden};
use ichnos_model::footprints::LabelPair;
use ichnos_model::{Dfg, Footprints, ProcessTree, TreeFootprints};
use serde::de::DeserializeOwned;

const LOGS: [&str; 6] = [
    "running-example-csv",
    "receipt-csv",
    "roadtraffic100traces-csv",
    "interleavings-receipt_even-csv",
    "interleavings-receipt_odd-csv",
    "reviewing-csv",
];

fn part<T: DeserializeOwned>(g: &Golden, pointer: &str) -> T {
    serde_json::from_value(g.expected_at(pointer).clone())
        .unwrap_or_else(|e| panic!("{pointer}: {e}"))
}

/// pm4py's tree at `run` (a JSON pointer prefix), up to child order.
fn expected_tree(g: &Golden, run: &str) -> String {
    let tree: String = part(g, &format!("{run}/tree"));
    canonical(&ProcessTree::parse(&tree).expect("pm4py tree parses")).to_string()
}

/// Checks `tree` against the golden: structure up to child order, tree
/// footprints, and the footprints of its Petri net.
fn check(g: &Golden, case: &str, tree: &ProcessTree) {
    check_run(g, "", case, tree);
}

/// [`check`] for a golden with `runs`: `tree` must match one of them.
fn check_any_run(g: &Golden, case: &str, tree: &ProcessTree) {
    let runs: Vec<serde_json::Value> = part(g, "/runs");
    let actual = canonical(tree).to_string();
    let trees: Vec<String> = (0..runs.len())
        .map(|i| expected_tree(g, &format!("/runs/{i}")))
        .collect();
    let i = trees
        .iter()
        .position(|t| *t == actual)
        .unwrap_or_else(|| panic!("{case}: tree {actual} matches no pm4py run: {trees:#?}"));
    check_run(g, &format!("/runs/{i}"), case, tree);
}

fn check_run(g: &Golden, run: &str, case: &str, tree: &ProcessTree) {
    assert_eq!(
        canonical(tree).to_string(),
        expected_tree(g, run),
        "{case}: tree"
    );

    let expected: TreeFootprints = part(g, &format!("{run}/tree_behaviour/footprints"));
    assert_eq!(tree.footprints(), expected, "{case}: tree footprints");

    let net = tree.to_petri_net();
    let expected_net: Footprints = part(g, &format!("{run}/petri_net_behaviour/footprints"));
    let actual = net
        .net
        .footprints(&net.initial_marking, Default::default())
        .expect("reachability graph fits");
    if PM4PY_NET_FOOTPRINTS_INCOMPLETE.contains(&case) {
        assert_net_footprints_cover(&actual, &expected_net, &expected.footprints, case);
    } else {
        assert_eq!(actual, expected_net, "{case}: Petri net footprints");
    }
}

/// Cases where pm4py's footprints of the Petri net miss pairs that the net
/// allows. pm4py's `get_visible_transitions_eventually_enabled_by_marking`
/// keeps one marking per silent transition, so when two silent paths reach
/// the same transition it explores only one of them. `ichnos-model`
/// explores every marking. In `inductive-imf-receipt-csv`, for example,
/// pm4py misses `T03 -> T02`, which the tree allows when every step
/// between them is skipped.
const PM4PY_NET_FOOTPRINTS_INCOMPLETE: [&str; 1] = ["inductive-imf-receipt-csv"];

/// Asserts that `actual` holds every pair of pm4py's net footprints
/// `expected`, and that each extra pair is in pm4py's footprints of the
/// tree, which pm4py computes without the bug.
fn assert_net_footprints_cover(
    actual: &Footprints,
    expected: &Footprints,
    tree: &Footprints,
    case: &str,
) {
    assert_eq!(actual.activities, expected.activities, "{case}: activities");
    assert_eq!(
        actual.start_activities, expected.start_activities,
        "{case}: start"
    );
    let pairs = |f: &Footprints| -> BTreeSet<LabelPair> {
        f.sequence.union(&f.parallel).cloned().collect()
    };
    let (actual, expected, tree) = (pairs(actual), pairs(expected), pairs(tree));
    let missing: Vec<_> = expected.difference(&actual).collect();
    assert!(missing.is_empty(), "{case}: ichnos misses {missing:?}");
    let unexplained: Vec<_> = actual
        .difference(&expected)
        .filter(|p| !tree.contains(p))
        .collect();
    assert!(
        unexplained.is_empty(),
        "{case}: extra pairs not in the tree: {unexplained:?}"
    );
    assert_ne!(
        actual, expected,
        "{case}: pm4py's footprints are now complete"
    );
}

fn load(g: &Golden) -> EventLog {
    load_csv_log(&g.fixture("log"))
}

#[test]
fn im_matches_pm4py() {
    for log_id in LOGS {
        let case = format!("inductive-im-{log_id}");
        let g = golden("discovery", &case);
        let options = InductiveOptions::default();
        let tree = process_tree_inductive(&load(&g), &EventKeys::default(), &options).unwrap();
        check(&g, &case, &tree);
    }
}

#[test]
fn imf_matches_pm4py() {
    for log_id in LOGS {
        let case = format!("inductive-imf-{log_id}");
        let g = golden("discovery", &case);
        let options = InductiveOptions::from_noise_threshold(0.2);
        assert_eq!(
            options.variant,
            InductiveVariant::Imf {
                noise_threshold: 0.2
            }
        );
        let tree = process_tree_inductive(&load(&g), &EventKeys::default(), &options).unwrap();
        check_any_run(&g, &case, &tree);
    }
}

/// pm4py's DFG of the log: edge, start and end frequencies.
fn dfg_of(log: &EventLog) -> Dfg {
    let seqs = log.activity_sequences(&EventKeys::default()).unwrap();
    let name = |a| seqs.activities.name(a);
    let mut dfg = Dfg::new();
    for trace in &seqs.traces {
        for pair in trace.windows(2) {
            dfg.add_edge(name(pair[0]), name(pair[1]), 1);
        }
        if let (Some(&first), Some(&last)) = (trace.first(), trace.last()) {
            dfg.add_start(name(first), 1);
            dfg.add_end(name(last), 1);
        }
    }
    dfg
}

#[test]
fn imd_matches_pm4py() {
    for log_id in LOGS {
        let case = format!("inductive-imd-{log_id}");
        let g = golden("discovery", &case);
        let log = load(&g);
        let options = InductiveOptions::new(InductiveVariant::Imd);
        let tree = process_tree_inductive_dfg(&dfg_of(&log), &options);
        check(&g, &case, &tree);

        // These logs have no empty traces, so IMd on the log itself gives
        // the same tree.
        let from_log = process_tree_inductive(&log, &EventKeys::default(), &options).unwrap();
        assert_eq!(from_log, tree, "{case}: IMd on the log");
        let net = petri_net_inductive_dfg(&dfg_of(&log), &options);
        assert_eq!(net, tree.to_petri_net(), "{case}: Petri net");
    }
}
