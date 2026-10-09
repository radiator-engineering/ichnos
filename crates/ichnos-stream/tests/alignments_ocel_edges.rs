use ichnos_core::{AttributeValue, Event, EventKeys};
use ichnos_model::{
    Marking, PetriNet,
    petri::{ArcEnds, ArcKind},
};
use ichnos_stream::{
    Collector, Error, MissingEventPolicy, OcelDistributorOptions, OcelFlatteningDistributor,
    StreamSink, StreamingAlignmentOptions, StreamingAlignments,
};
use std::{cell::RefCell, rc::Rc};

fn chain() -> (PetriNet, Marking, Marking) {
    let mut net = PetriNet::new("chain");
    let p = net.add_place("p");
    let q = net.add_place("q");
    let t = net.add_transition("A", Some("A"));
    net.add_input_arc(p, t).unwrap();
    net.add_output_arc(t, q).unwrap();
    (net, Marking::from([(p, 1)]), Marking::from([(q, 1)]))
}

fn event(c: &str, a: &str) -> Event {
    [("case:concept:name", c), ("concept:name", a)]
        .into_iter()
        .collect()
}

#[test]
fn iws_missing_policy_atomic_and_restart() {
    let (net, im, fm) = chain();
    let mut a =
        StreamingAlignments::new(net.clone(), im.clone(), fm.clone(), Default::default()).unwrap();
    a.push(&event("c", "A")).unwrap();
    let before = a.get();
    assert!(matches!(
        a.push(&Event::new()),
        Err(Error::MissingField { event: 1, .. })
    ));
    assert_eq!(a.get(), before);
    assert_eq!(a.processed_events(), 1);
    assert_eq!(a.finish("c").unwrap().cost, 0);
    assert!(a.get()["c"].is_complete);
    a.push(&event("c", "X")).unwrap();
    assert_eq!(a.get()["c"].cost, 0); // retained completion shadows prefix
    assert!(a.remove_case("c"));
    assert!(!a.remove_case("c"));
    assert!(a.get().is_empty());
    a.push(&event("c", "A")).unwrap();
    assert_eq!(a.get()["c"].cost, 0);
    assert!(!a.get()["c"].is_complete);
    let mut o = StreamingAlignmentOptions::default();
    o.events.missing = MissingEventPolicy::Ignore;
    let mut a = StreamingAlignments::new(net, im, fm, o).unwrap();
    a.push(&Event::new()).unwrap();
    assert_eq!(a.skipped_events(), 1);
    assert_eq!(a.processed_events(), 0);
    assert!(a.get().is_empty());
}

#[test]
fn iws_rejects_invalid_options_runs_and_models() {
    let (net, im, fm) = chain();
    for o in [
        StreamingAlignmentOptions {
            look_ahead: 0,
            ..Default::default()
        },
        StreamingAlignmentOptions {
            decay_time: f64::NAN,
            ..Default::default()
        },
        StreamingAlignmentOptions {
            discount_factor: 0.0,
            ..Default::default()
        },
        StreamingAlignmentOptions {
            discount_factor: f64::INFINITY,
            ..Default::default()
        },
        StreamingAlignmentOptions {
            max_states: 0,
            ..Default::default()
        },
        StreamingAlignmentOptions {
            max_expansions: 0,
            ..Default::default()
        },
    ] {
        assert!(StreamingAlignments::new(net.clone(), im.clone(), fm.clone(), o).is_err());
    }
    assert!(
        StreamingAlignments::with_proxy_sequences(
            net.clone(),
            im.clone(),
            fm.clone(),
            vec![],
            Default::default()
        )
        .is_err()
    );
    assert!(
        StreamingAlignments::with_proxy_sequences(
            net.clone(),
            im.clone(),
            fm.clone(),
            vec![vec![]],
            Default::default()
        )
        .is_err()
    );
    let t = net.transition_by_name("A").unwrap();
    assert!(
        StreamingAlignments::with_proxy_sequences(
            net.clone(),
            im.clone(),
            fm.clone(),
            vec![vec![t, t]],
            Default::default()
        )
        .is_err()
    );
    let mut special = net.clone();
    let p = net.place_by_name("p").unwrap();
    special
        .add_arc(ArcEnds::PlaceToTransition(p, t), 1, ArcKind::Inhibitor)
        .unwrap();
    assert!(StreamingAlignments::new(special, im.clone(), fm.clone(), Default::default()).is_err());
    let foreign = net.clone().add_place("foreign");
    assert!(
        StreamingAlignments::new(
            net.clone(),
            Marking::from([(foreign, 1)]),
            fm.clone(),
            Default::default()
        )
        .is_err()
    );
    let mut unreachable = net;
    unreachable.remove_transition(t);
    assert!(StreamingAlignments::new(unreachable, im, fm, Default::default()).is_err());
}

#[test]
fn automatic_proxy_bounds_loops_and_retains_empty_complete_run() {
    let mut net = PetriNet::new("loop");
    let p = net.add_place("p");
    let q = net.add_place("q");
    let loop_t = net.add_transition("loop", Some("A"));
    let finish = net.add_transition("finish", Some("B"));
    net.add_input_arc(p, loop_t).unwrap();
    net.add_output_arc(loop_t, p).unwrap();
    net.add_input_arc(p, finish).unwrap();
    net.add_output_arc(finish, q).unwrap();
    let o = StreamingAlignmentOptions {
        proxy_traces: 2,
        max_trace_length: 3,
        max_expansions: 20,
        ..Default::default()
    };
    let mut a =
        StreamingAlignments::new(net, Marking::from([(p, 1)]), Marking::from([(q, 1)]), o).unwrap();
    a.push(&event("c", "A")).unwrap();
    a.push(&event("c", "B")).unwrap();
    assert_eq!(a.finish("c").unwrap().cost, 0);
    let (net, im, _) = chain();
    let mut a = StreamingAlignments::with_proxy_sequences(
        net,
        im.clone(),
        im,
        vec![vec![]],
        Default::default(),
    )
    .unwrap();
    a.push(&event("c", "X")).unwrap();
    let r = a.finish("c").unwrap();
    assert!(r.is_valid);
    assert_eq!(r.cost, 10_000);
}

fn ocel() -> Event {
    let mut e = Event::new();
    e.insert("ocel:activity", "A");
    e.insert("ocel:timestamp", 123_i64);
    e.insert(
        "ocel:type:order",
        AttributeValue::List(vec![("0".into(), "o1".into())]),
    );
    e
}

#[test]
fn ocel_validates_before_delivery_and_handles_equal_rename_keys() {
    let c = Rc::new(RefCell::new(Collector::<Event>::new()));
    let mut d = OcelFlatteningDistributor::default();
    d.register("order", c.clone());
    let mut invalid = ocel();
    invalid.insert("ocel:type:unregistered", "not-a-list");
    assert!(matches!(
        d.append(&invalid),
        Err(Error::OcelObjectsType { event: 0, .. })
    ));
    assert!(c.borrow().get().is_empty());
    assert!(matches!(
        d.append(&Event::new()),
        Err(Error::MissingField { event: 1, .. })
    ));
    d.append(&ocel()).unwrap();
    assert_eq!(c.borrow().get().len(), 1);
    let o = OcelDistributorOptions {
        keys: EventKeys::default()
            .with_activity("ocel:activity")
            .with_timestamp("ocel:timestamp"),
        ..Default::default()
    };
    let c = Rc::new(RefCell::new(Collector::<Event>::new()));
    let mut d = OcelFlatteningDistributor::new(o).unwrap();
    d.register("order", c.clone());
    d.append(&ocel()).unwrap();
    assert_eq!(
        c.borrow().get()[0].get("ocel:activity").unwrap().as_str(),
        Some("A")
    );
    assert_eq!(
        c.borrow().get()[0].get("ocel:timestamp").unwrap().as_i64(),
        Some(123)
    );
    assert!(
        OcelFlatteningDistributor::new(OcelDistributorOptions {
            object_type_prefix: String::new(),
            ..Default::default()
        })
        .is_err()
    );
}

#[test]
fn ocel_continues_after_listener_errors_and_preserves_typed_ids() {
    struct Fails;
    impl StreamSink for Fails {
        fn push(&mut self, _: &Event) -> ichnos_stream::Result<()> {
            Err(Error::ObserverBorrowed)
        }
    }
    let c = Rc::new(RefCell::new(Collector::<Event>::new()));
    let mut d = OcelFlatteningDistributor::default();
    d.register("order", Fails);
    d.register("order", c.clone());
    let mut e = ocel();
    e.insert(
        "ocel:type:order",
        AttributeValue::List(vec![
            ("x".into(), 42_i64.into()),
            ("x".into(), 43_i64.into()),
        ]),
    );
    assert!(matches!(d.append(&e), Err(Error::ObserverBorrowed)));
    assert_eq!(c.borrow().get().len(), 2);
    assert_eq!(
        c.borrow().get()[0]
            .get("case:concept:name")
            .unwrap()
            .as_i64(),
        Some(42)
    );
    assert_eq!(
        c.borrow().get()[1]
            .get("case:concept:name")
            .unwrap()
            .as_i64(),
        Some(43)
    );
}
