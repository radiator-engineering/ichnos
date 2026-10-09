//! Case lifecycle, atomic errors and native failure-path regressions.

use ichnos_core::{
    Event, EventKeys,
    chrono::{TimeZone, Utc},
};
use ichnos_model::{
    Footprints, Label, Marking, PetriNet,
    petri::{ArcEnds, ArcKind},
};
use ichnos_stream::{
    Error, MissingEventPolicy, StreamingConformanceOptions, StreamingFootprintsConformance,
    StreamingTbrConformance, StreamingTbrOptions, StreamingTemporalConformance,
    StreamingTemporalOptions, TemporalProfile,
};
use std::collections::BTreeSet;

fn event(case: &str, activity: &str) -> Event {
    Event::from_iter([("case:concept:name", case), ("concept:name", activity)])
}
fn fp() -> StreamingFootprintsConformance {
    StreamingFootprintsConformance::new(
        Footprints {
            activities: BTreeSet::from([Label::from("a"), Label::from("b")]),
            start_activities: BTreeSet::from([Label::from("a")]),
            sequence: BTreeSet::from([(Label::from("a"), Label::from("b"))]),
            parallel: Default::default(),
        },
        BTreeSet::from([Label::from("b")]),
        Default::default(),
    )
}

#[test]
fn unknown_first_footprints_deviations_survive_and_cases_restart() {
    let mut algo = fp();
    algo.push(&event("c", "unknown")).unwrap();
    algo.push(&event("c", "unknown")).unwrap();
    assert_eq!(algo.get_status("c").unwrap().deviations, 2);
    assert!(algo.get_status("c").unwrap().last_activity.is_none());
    algo.push(&event("c", "a")).unwrap();
    algo.push(&event("c", "b")).unwrap();
    assert_eq!(algo.get_status("c").unwrap().deviations, 2);
    assert_eq!(algo.terminate("c"), Some(false));
    algo.push(&event("c", "a")).unwrap();
    assert!(algo.get_status("c").unwrap().is_fit());
    assert_eq!(algo.terminate("c"), Some(false)); // End constraint waits for termination.
    algo.push(&event("c", "a")).unwrap();
    algo.push(&event("c", "b")).unwrap();
    assert_eq!(algo.terminate("c"), Some(true));
    assert_eq!(algo.terminate("c"), None);
}

#[test]
fn missing_policy_is_atomic_and_indexed_for_every_consumer() {
    let keys = EventKeys::default()
        .with_case_id("id")
        .with_activity("task");
    let options = StreamingConformanceOptions {
        keys,
        missing: MissingEventPolicy::Reject,
    };
    let (fp, ends) = (Footprints::default(), BTreeSet::new());
    let mut algo = StreamingFootprintsConformance::new(fp, ends, options.clone());
    assert!(matches!(
        algo.push(&Event::new()),
        Err(Error::MissingField { event: 0, .. })
    ));
    assert!(algo.get().is_empty());
    assert!(matches!(
        algo.push(&Event::new()),
        Err(Error::MissingField { event: 1, .. })
    ));
    let mut tbr = StreamingTbrConformance::new(
        PetriNet::new("empty"),
        Marking::new(),
        Marking::new(),
        StreamingTbrOptions {
            events: options.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        tbr.push(&Event::new()),
        Err(Error::MissingField { event: 0, .. })
    ));
    assert!(tbr.get().is_empty());
    let mut temporal = StreamingTemporalConformance::new(
        TemporalProfile::new(),
        StreamingTemporalOptions {
            events: options,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        temporal.push(&Event::new()),
        Err(Error::MissingField { event: 0, .. })
    ));
    assert_eq!(temporal.terminate("c"), None);
    let mut ignore = fp_for_missing();
    ignore.push(&Event::new()).unwrap();
    assert_eq!(ignore.skipped(), 1);
    let mut tbr = StreamingTbrConformance::new(
        PetriNet::new("empty"),
        Marking::new(),
        Marking::new(),
        Default::default(),
    )
    .unwrap();
    tbr.push(&Event::new()).unwrap();
    assert_eq!(tbr.skipped(), 1);
    tbr.push(&event("c", "unknown")).unwrap();
    assert_eq!(tbr.unknown_activities(), 1);
    assert!(tbr.get().is_empty());
}

fn fp_for_missing() -> StreamingFootprintsConformance {
    fp()
}

#[test]
fn temporal_invalid_types_do_not_create_or_modify_a_case() {
    let profile: TemporalProfile = [(("a".into(), "b".into()), (1.0, 0.0))].into();
    let mut algo = StreamingTemporalConformance::new(profile, Default::default()).unwrap();
    let mut bad = event("c", "a");
    bad.insert("time:timestamp", "not-a-date");
    assert!(matches!(
        algo.push(&bad),
        Err(Error::TimestampType { event: 0, .. })
    ));
    assert!(algo.terminate("c").is_none());
    let mut a = event("c", "a");
    a.insert("time:timestamp", Utc.timestamp_opt(0, 0).unwrap());
    algo.push(&a).unwrap();
    let mut b = event("c", "b");
    b.insert("time:timestamp", Utc.timestamp_opt(2, 0).unwrap());
    algo.push(&b).unwrap();
    assert!(algo.get()["c"][0].zeta.is_infinite());
    assert_eq!(algo.terminate("c").unwrap().len(), 1);
    assert!(algo.get().is_empty());
    algo.push(&b).unwrap();
    assert!(algo.get().is_empty());
    assert_eq!(algo.terminate("c"), Some(vec![]));
    algo.push(&event("c", "a")).unwrap();
    assert_eq!(algo.skipped(), 1);
    assert!(algo.terminate("c").is_none());
}

#[test]
fn temporal_profile_and_tolerance_validation() {
    for zeta in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(
            StreamingTemporalConformance::new(
                TemporalProfile::new(),
                StreamingTemporalOptions {
                    zeta,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    for bounds in [(f64::NAN, 1.0), (0.0, -1.0), (0.0, f64::INFINITY)] {
        assert!(
            StreamingTemporalConformance::new(
                [(("a".into(), "b".into()), bounds)].into(),
                Default::default()
            )
            .is_err()
        );
    }
}

#[test]
fn blocked_silent_path_falls_back_without_partial_marking_or_native_exception() {
    let mut net = PetriNet::new("join");
    let p0 = net.add_place("p0");
    let prerequisite = net.add_place("prerequisite");
    let p1 = net.add_place("p1");
    let out = net.add_place("out");
    let tau = net.add_transition("tau", None::<&str>);
    let a = net.add_transition("a", Some("a"));
    net.add_input_arc(p0, tau).unwrap();
    net.add_input_arc(prerequisite, tau).unwrap();
    net.add_output_arc(tau, p1).unwrap();
    net.add_input_arc(p1, a).unwrap();
    net.add_output_arc(a, out).unwrap();
    let mut algo =
        StreamingTbrConformance::new(net, [(p0, 1)].into(), [(out, 1)].into(), Default::default())
            .unwrap();
    algo.push(&event("c", "a")).unwrap();
    let s = algo.get_status("c").unwrap();
    assert_eq!(s.missing, 1);
    assert_eq!(s.marking, [(p0, 1), (out, 1)].into());
    let final_status = algo.terminate("c").unwrap();
    assert_eq!((final_status.missing, final_status.remaining), (1, 1));
}

#[test]
fn model_validation_and_deterministic_duplicate_labels() {
    let mut net = PetriNet::new("duplicate");
    let source = net.add_place("source");
    let z = net.add_place("z");
    let a = net.add_place("a");
    let tz = net.add_transition("z", Some("same"));
    let ta = net.add_transition("a", Some("same"));
    net.add_input_arc(source, tz).unwrap();
    net.add_output_arc(tz, z).unwrap();
    net.add_input_arc(source, ta).unwrap();
    net.add_output_arc(ta, a).unwrap();
    let mut algo = StreamingTbrConformance::new(
        net.clone(),
        [(source, 1)].into(),
        [(a, 1)].into(),
        Default::default(),
    )
    .unwrap();
    algo.push(&event("c", "same")).unwrap();
    assert_eq!(algo.get_status("c").unwrap().marking, [(a, 1)].into());
    assert!(algo.terminate("c").unwrap().is_fit);
    let alien = net.add_place("alien");
    assert!(
        StreamingTbrConformance::new(
            algo.net().clone(),
            [(alien, 1)].into(),
            Marking::new(),
            Default::default()
        )
        .is_err()
    );
    net.add_arc(ArcEnds::PlaceToTransition(alien, ta), 1, ArcKind::Inhibitor)
        .unwrap();
    assert!(
        StreamingTbrConformance::new(net, Marking::new(), Marking::new(), Default::default())
            .is_err()
    );
}

#[test]
fn capped_partial_silent_path_counts_original_marking_shortfall() {
    let mut net = PetriNet::new("partial silent path");
    let p0 = net.add_place("p0");
    let p1 = net.add_place("p1");
    let p2 = net.add_place("p2");
    let tau = net.add_transition("tau", None::<&str>);
    let a = net.add_transition("A", Some("A"));
    net.add_input_arc(p1, tau).unwrap();
    net.add_output_arc(tau, p0).unwrap();
    net.add_arc(ArcEnds::PlaceToTransition(p0, a), 2, ArcKind::Normal)
        .unwrap();
    net.add_output_arc(a, p2).unwrap();
    let mut algo = StreamingTbrConformance::new(
        net,
        [(p0, 1), (p1, 1)].into(),
        [(p2, 1)].into(),
        StreamingTbrOptions {
            maximum_iterations_invisibles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    algo.push(&event("c", "A")).unwrap();
    let status = algo.get_status("c").unwrap();
    // pm4py tests the two tokens after tau but fires from the original one-
    // token marking, reporting missing=0. Rust inserts the actual shortfall.
    assert_eq!(status.missing, 1);
    assert_eq!(status.marking, [(p1, 1), (p2, 1)].into());
}
