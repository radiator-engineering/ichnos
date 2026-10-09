use ichnos_model::{Label, ProcessTree};

use super::*;

fn pair(a: &str, b: &str) -> LabelPair {
    (Label::new(a), Label::new(b))
}

fn labels(names: &[&str]) -> BTreeSet<Label> {
    names.iter().map(|&n| Label::new(n)).collect()
}

#[test]
fn trace_footprints_split_the_dfg() {
    let fp = LogFootprints::of_trace(&["a", "b", "c", "b", "d", "d"]);
    assert_eq!(fp.dfg[&pair("b", "c")], 1);
    assert_eq!(
        fp.sequence,
        BTreeSet::from([pair("a", "b"), pair("b", "d")])
    );
    assert_eq!(
        fp.parallel,
        BTreeSet::from([pair("b", "c"), pair("c", "b"), pair("d", "d")])
    );
    assert_eq!(fp.start_activities, labels(&["a"]));
    assert_eq!(fp.end_activities, labels(&["d"]));
    assert_eq!(fp.min_trace_length, 6);
}

#[test]
fn empty_trace_has_no_footprints() {
    let fp = LogFootprints::of_trace::<&str>(&[]);
    assert_eq!(fp, LogFootprints::default());
}

#[test]
fn tree_checks_end_activities_length_and_always_happening() {
    // ->(a, X(b, c), d)
    let tree = ProcessTree::parse("->( 'a', X( 'b', 'c' ), 'd' )").unwrap();
    let model = ModelFootprints::of_tree(&tree);
    let fits =
        |trace: &[&str]| FootprintsDeviations::of_trace(&LogFootprints::of_trace(trace), &model);
    assert!(fits(&["a", "b", "d"]).is_footprints_fit);
    let d = fits(&["a", "b"]);
    assert_eq!(d.end_activities, labels(&["b"]));
    assert_eq!(d.activities_always_happening, labels(&["d"]));
    assert!(!d.min_length_fit);
    let d = fits(&["b", "c", "d"]);
    assert_eq!(d.start_activities, labels(&["b"]));
    assert_eq!(d.footprints, BTreeSet::from([pair("b", "c")]));
    assert_eq!(d.activities_always_happening, labels(&["a"]));
}

#[test]
fn net_footprints_skip_the_tree_only_checks() {
    let tree = ProcessTree::parse("->( 'a', 'b' )").unwrap();
    let mut model = ModelFootprints::of_tree(&tree);
    model.end_activities = None;
    model.activities_always_happening = None;
    model.min_trace_length = None;
    let d = FootprintsDeviations::of_trace(&LogFootprints::of_trace(&["a"]), &model);
    assert!(d.is_footprints_fit);
}

#[test]
fn strict_violations_tell_sequence_from_parallel() {
    let log = LogFootprints::of_trace(&["a", "b", "a"]);
    let model: ModelFootprints = LogFootprints::of_trace(&["a", "b"]).into();
    assert_eq!(
        footprint_violations(&log, &model, false),
        BTreeSet::from([pair("b", "a")])
    );
    assert_eq!(
        footprint_violations(&log, &model, true),
        BTreeSet::from([pair("a", "b"), pair("b", "a")])
    );
}

#[test]
fn fitness_and_precision_formulas() {
    // DFG a->b (1), b->c (1); the model allows a->b only.
    let log = LogFootprints::of_trace(&["a", "b", "c"]);
    let model: ModelFootprints = LogFootprints::of_trace(&["a", "b", "d"]).into();
    let d = FootprintsDeviations::of_log(&log, &model);
    assert_eq!(d.footprints, BTreeSet::from([pair("b", "c")]));
    assert_eq!(d.end_activities, labels(&["c"]));
    // ((1 - 1/2) * 2 + (1 + 1 - 0 - 1)) / (2 + 2)
    assert_eq!(footprints_fitness(&log, &d), 0.5);
    // a->b of {a->b, b->d}.
    assert_eq!(footprints_precision(&log, &model), 0.5);
    assert_eq!(
        footprints_fitness(&LogFootprints::of_trace(&["a"]), &d),
        1.0
    );
    assert_eq!(footprints_precision(&log, &ModelFootprints::default()), 1.0);
}
