//! Compares object-centric Petri net discovery with pm4py's goldens
//! `fixtures/golden/discovery/ocpn-*.json`.
//!
//! The activities, object types, double arcs and object ids must equal
//! pm4py's. For each object type, the tree must equal one of pm4py's trees
//! for that type up to the order of XOR and parallel children, and the
//! footprints of the tree and of its Petri net must equal that run's. pm4py's
//! IMf trees can depend on Python's hash seed, so a golden holds one run per
//! distinct tree.

#[allow(dead_code)]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use common::canonical;
use ichnos_discovery::{InductiveOptions, InductiveVariant, OcpnOptions, discover_oc_petri_net};
use ichnos_golden::{cases, golden};
use ichnos_model::footprints::LabelPair;
use ichnos_model::{Footprints, ProcessTree, TreeFootprints};
use ichnos_ocel::Ocel;
use serde::Deserialize;

#[derive(Deserialize)]
struct Expected {
    activities: BTreeSet<String>,
    object_types: BTreeSet<String>,
    nets: BTreeMap<String, ExpectedNet>,
}

#[derive(Deserialize)]
struct ExpectedNet {
    double_arcs: BTreeMap<String, bool>,
    object_ids: BTreeSet<String>,
    runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    tree: String,
    tree_footprints: TreeFootprints,
    petri_net_footprints: Footprints,
}

fn read(path: &Path) -> Ocel {
    if path.to_string_lossy().contains("ocel20") {
        ichnos_io::read_ocel2_json(path).unwrap()
    } else {
        ichnos_io::read_ocel_json(path).unwrap()
    }
}

/// The options of the golden run `run` (see `OCPN_RUNS` in
/// `tools/golden/cases/discovery.py`).
fn options(run: &str) -> OcpnOptions {
    match run {
        "im" => OcpnOptions::default(),
        "imf" => OcpnOptions::new(InductiveVariant::Imf {
            noise_threshold: 0.2,
        }),
        "imd" => OcpnOptions::new(InductiveVariant::Imd),
        "fallthroughs" => {
            OcpnOptions::default().with_inductive(InductiveOptions::new(InductiveVariant::Im))
        }
        "doublearc" => OcpnOptions::default().with_double_arc_threshold(0.6),
        _ => panic!("unknown OCPN run {run}"),
    }
}

#[test]
fn ocpn_matches_pm4py() {
    let ids: Vec<String> = cases("discovery")
        .into_iter()
        .filter(|c| c.starts_with("ocpn-"))
        .collect();
    assert_eq!(ids.len(), 13, "OCPN goldens: {ids:?}");
    for id in ids {
        let g = golden("discovery", &id);
        let run = id.split('-').nth(1).unwrap();
        let ocpn = discover_oc_petri_net(&read(&g.fixture("log")), &options(run)).unwrap();
        let expected: Expected = g.expected_as();

        assert_eq!(ocpn.activities, expected.activities, "{id}: activities");
        let types: BTreeSet<String> = ocpn.nets.keys().cloned().collect();
        assert_eq!(types, expected.object_types, "{id}: object types");
        assert_eq!(
            expected.nets.keys().cloned().collect::<BTreeSet<_>>(),
            types,
            "{id}: nets"
        );
        for (ot, e) in &expected.nets {
            let case = format!("{id} ({ot})");
            let net = &ocpn.nets[ot];
            assert_eq!(net.double_arcs, e.double_arcs, "{case}: double arcs");
            assert_eq!(net.object_ids, e.object_ids, "{case}: object ids");
            assert_eq!(net.net, net.tree.to_petri_net(), "{case}: net");

            let actual = canonical(&net.tree).to_string();
            let trees: Vec<String> = e
                .runs
                .iter()
                .map(|r| {
                    canonical(&ProcessTree::parse(&r.tree).expect("pm4py tree parses")).to_string()
                })
                .collect();
            let i = trees.iter().position(|t| *t == actual).unwrap_or_else(|| {
                panic!("{case}: tree {actual} matches no pm4py run: {trees:#?}")
            });
            let run = &e.runs[i];
            assert_eq!(
                net.tree.footprints(),
                run.tree_footprints,
                "{case}: tree footprints"
            );
            let footprints = net
                .net
                .net
                .footprints(&net.net.initial_marking, Default::default())
                .expect("reachability graph fits");
            if footprints != run.petri_net_footprints {
                assert_net_footprints_cover(
                    &footprints,
                    &run.petri_net_footprints,
                    &run.tree_footprints.footprints,
                    &case,
                );
            }
        }
    }
}

/// pm4py's footprints of a Petri net can miss pairs the net allows: its
/// `get_visible_transitions_eventually_enabled_by_marking` keeps one marking
/// per silent transition. `ichnos-model` explores every marking. Asserts
/// that `actual` holds every pair of pm4py's net footprints `expected`, and
/// that each extra pair is in pm4py's footprints of the tree, which pm4py
/// computes without that search.
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
}
