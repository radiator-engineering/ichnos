use super::*;

/// `p0 -a-> p1 -b-> p2`, plus a silent skip `p0 -tau-> p1`.
fn chain() -> (PetriNet, [PlaceId; 3], [TransitionId; 3]) {
    let mut net = PetriNet::new("chain");
    let p = [
        net.add_place("p0"),
        net.add_place("p1"),
        net.add_place("p2"),
    ];
    let a = net.add_transition("ta", Some("a"));
    let b = net.add_transition("tb", Some("b"));
    let skip = net.add_transition("skip", None::<Label>);
    net.add_input_arc(p[0], a).unwrap();
    net.add_output_arc(a, p[1]).unwrap();
    net.add_input_arc(p[1], b).unwrap();
    net.add_output_arc(b, p[2]).unwrap();
    net.add_input_arc(p[0], skip).unwrap();
    net.add_output_arc(skip, p[1]).unwrap();
    (net, p, [a, b, skip])
}

#[test]
fn build_and_navigate() {
    let (net, p, [a, b, skip]) = chain();
    assert_eq!(net.place_count(), 3);
    assert_eq!(net.transition_count(), 3);
    assert_eq!(net.arc_count(), 6);
    assert!(net.transition(skip).is_silent());
    assert_eq!(net.transition(a).label.as_deref(), Some("a"));
    assert_eq!(net.preset(b).collect::<Vec<_>>(), vec![p[1]]);
    assert_eq!(net.postset(b).collect::<Vec<_>>(), vec![p[2]]);
    assert_eq!(net.place_preset(p[1]).collect::<Vec<_>>(), vec![a, skip]);
    assert_eq!(net.place_postset(p[0]).collect::<Vec<_>>(), vec![a, skip]);
    assert_eq!(net.place_by_name("p2"), Some(p[2]));
    assert_eq!(net.transition_by_name("tb"), Some(b));
    assert_eq!(net.discover_initial_marking(), Marking::from([(p[0], 1)]));
    assert_eq!(net.discover_final_marking(), Marking::from([(p[2], 1)]));
}

#[test]
fn arc_errors() {
    let (mut net, p, [a, ..]) = chain();
    assert_eq!(
        net.add_arc(ArcEnds::PlaceToTransition(p[0], a), 0, ArcKind::Normal),
        Err(PetriNetError::ZeroWeight)
    );
    assert_eq!(
        net.add_input_arc(PlaceId(99), a),
        Err(PetriNetError::UnknownPlace(PlaceId(99)))
    );
    assert_eq!(
        net.add_output_arc(TransitionId(99), p[0]),
        Err(PetriNetError::UnknownTransition(TransitionId(99)))
    );
}

#[test]
fn enabling_and_firing() {
    let (net, p, [a, b, skip]) = chain();
    let m0 = Marking::from([(p[0], 1)]);
    assert_eq!(net.enabled_transitions(&m0), vec![a, skip]);
    assert!(!net.is_enabled(b, &m0));
    assert_eq!(net.fire(b, &m0), Err(NotEnabled(b)));
    let m1 = net.fire(a, &m0).unwrap();
    assert_eq!(m1, Marking::from([(p[1], 1)]));
    assert_eq!(net.fire(b, &m1).unwrap(), Marking::from([(p[2], 1)]));
    // Weak firing clamps at zero instead of failing.
    assert_eq!(net.weak_fire(b, &m0), Marking::from([(p[0], 1), (p[2], 1)]));
}

#[test]
fn weighted_arcs() {
    let mut net = PetriNet::new("w");
    let p = net.add_place("p");
    let q = net.add_place("q");
    let t = net.add_transition("t", Some("t"));
    net.add_arc(ArcEnds::PlaceToTransition(p, t), 2, ArcKind::Normal)
        .unwrap();
    net.add_arc(ArcEnds::TransitionToPlace(t, q), 3, ArcKind::Normal)
        .unwrap();
    assert!(!net.is_enabled(t, &Marking::from([(p, 1)])));
    let m = net.fire(t, &Marking::from([(p, 3)])).unwrap();
    assert_eq!(m, Marking::from([(p, 1), (q, 3)]));
    let inc = IncidenceMatrix::new(&net);
    assert_eq!(inc.rows(), &[vec![-2], vec![3]]);
    assert_eq!(inc.encode_marking(&m), vec![1, 3]);
}

#[test]
fn inhibitor_and_reset_arcs() {
    let mut net = PetriNet::new("ir");
    let src = net.add_place("src");
    let block = net.add_place("block");
    let bin = net.add_place("bin");
    let out = net.add_place("out");
    let t = net.add_transition("t", Some("t"));
    net.add_input_arc(src, t).unwrap();
    net.add_arc(ArcEnds::PlaceToTransition(block, t), 1, ArcKind::Inhibitor)
        .unwrap();
    net.add_arc(ArcEnds::PlaceToTransition(bin, t), 1, ArcKind::Reset)
        .unwrap();
    net.add_output_arc(t, out).unwrap();
    assert!(net.has_special_arcs());

    let blocked = Marking::from([(src, 1), (block, 1)]);
    assert!(!net.is_enabled(t, &blocked));

    let m = Marking::from([(src, 1), (bin, 5)]);
    assert!(net.is_enabled(t, &m));
    assert_eq!(net.fire(t, &m).unwrap(), Marking::from([(out, 1)]));
}

#[test]
fn marking_operations() {
    let p = PlaceId(0);
    let q = PlaceId(1);
    let mut m = Marking::new();
    m.add(p, 2);
    m.add(q, 0);
    assert_eq!(m.len(), 1);
    m.remove(p, 5);
    assert!(m.is_empty());
    let a = Marking::from([(p, 1)]);
    let b = Marking::from([(p, 2), (q, 1)]);
    assert!(a.is_covered_by(&b));
    assert!(!b.is_covered_by(&a));
    assert_eq!(&a + &b, Marking::from([(p, 3), (q, 1)]));
    assert_eq!(b.total_tokens(), 3);

    let (net, places, _) = chain();
    let m = Marking::from([(places[2], 1), (places[0], 2)]);
    assert_eq!(m.display(&net).to_string(), "['p0:2', 'p2:1']");
}

#[test]
fn reachability_graph() {
    let (net, p, [a, b, skip]) = chain();
    let g = net
        .reachability_graph(&Marking::from([(p[0], 1)]), ReachabilityOptions::default())
        .unwrap();
    assert_eq!(g.len(), 3);
    let m1 = g.index_of(&Marking::from([(p[1], 1)])).unwrap();
    let m2 = g.index_of(&Marking::from([(p[2], 1)])).unwrap();
    assert_eq!(g.outgoing(0), &[(a, m1), (skip, m1)]);
    assert_eq!(g.outgoing(m1), &[(b, m2)]);
    assert!(g.outgoing(m2).is_empty());
    assert_eq!(g.edges().count(), 3);
}

#[test]
fn reachability_limit_on_unbounded_net() {
    let mut net = PetriNet::new("unbounded");
    let p = net.add_place("p");
    let q = net.add_place("q");
    let t = net.add_transition("t", Some("t"));
    net.add_input_arc(p, t).unwrap();
    net.add_output_arc(t, p).unwrap();
    net.add_output_arc(t, q).unwrap();
    let err = net
        .reachability_graph(
            &Marking::from([(p, 1)]),
            ReachabilityOptions { max_markings: 50 },
        )
        .unwrap_err();
    assert_eq!(err, ReachabilityError::TooManyMarkings(50));
}

#[test]
fn eventually_enabled_passes_silent_transitions() {
    let (net, p, [a, b, _]) = chain();
    let ev = net.visible_transitions_eventually_enabled(&Marking::from([(p[0], 1)]));
    assert_eq!(ev.into_iter().collect::<Vec<_>>(), vec![a, b]);
}
