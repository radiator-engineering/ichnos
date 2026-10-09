use ichnos_model::{Marking, PetriNet};

use super::*;

/// source -> tau -> p -> a -> sink
fn silent_then_a() -> (PetriNet, Marking, Marking) {
    let mut net = PetriNet::new("n");
    let source = net.add_place("source");
    let p = net.add_place("p");
    let sink = net.add_place("sink");
    let tau = net.add_transition("tau", None::<&str>);
    let a = net.add_transition("a", Some("a"));
    net.add_input_arc(source, tau).unwrap();
    net.add_output_arc(tau, p).unwrap();
    net.add_input_arc(p, a).unwrap();
    net.add_output_arc(a, sink).unwrap();
    let im = Marking::from([(source, 1)]);
    let fm = Marking::from([(sink, 1)]);
    (net, im, fm)
}

#[test]
fn walks_through_silent_transitions() {
    let (net, im, fm) = silent_then_a();
    let replayer = TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).unwrap();
    let r = replayer.replay(&["a"]).unwrap();
    assert!(r.is_fit);
    let fired: Vec<&str> = r
        .activated_transitions
        .iter()
        .map(|&t| net.transition(t).name.as_str())
        .collect();
    assert_eq!(fired, ["tau", "a"]);
    assert_eq!((r.consumed_tokens, r.produced_tokens), (3, 3));

    // Without the walk, `a` fires with a missing token and the token in
    // `source` remains.
    let options = TokenReplayOptions::default().walk_through_hidden_transitions(false);
    let replayer = TokenReplayer::new(&net, &im, &fm, options).unwrap();
    let r = replayer.replay(&["a"]).unwrap();
    assert!(!r.is_fit);
    assert_eq!((r.missing_tokens, r.remaining_tokens), (1, 1));
}

#[test]
fn reaches_the_final_marking_through_silent_transitions() {
    // source -> a -> p -> tau -> sink
    let mut net = PetriNet::new("n");
    let source = net.add_place("source");
    let p = net.add_place("p");
    let sink = net.add_place("sink");
    let a = net.add_transition("a", Some("a"));
    let tau = net.add_transition("tau", None::<&str>);
    net.add_input_arc(source, a).unwrap();
    net.add_output_arc(a, p).unwrap();
    net.add_input_arc(p, tau).unwrap();
    net.add_output_arc(tau, sink).unwrap();
    let im = Marking::from([(source, 1)]);
    let fm = Marking::from([(sink, 1)]);
    let replayer = TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).unwrap();
    let r = replayer.replay(&["a"]).unwrap();
    assert!(r.is_fit);
    assert_eq!(r.activated_transitions, [a, tau]);
    assert_eq!(r.reached_marking, fm);

    let options = TokenReplayOptions::default().try_to_reach_final_marking_through_hidden(false);
    let replayer = TokenReplayer::new(&net, &im, &fm, options).unwrap();
    let r = replayer.replay(&["a"]).unwrap();
    assert!(!r.is_fit);
    assert_eq!((r.missing_tokens, r.remaining_tokens), (1, 1));
    assert_eq!(r.enabled_transitions_in_marking, BTreeSet::new());
}

#[test]
fn missing_tokens_add_the_arc_weight() {
    // p -(2)-> a -> sink, with one token in p.
    let mut net = PetriNet::new("n");
    let p = net.add_place("p");
    let sink = net.add_place("sink");
    let a = net.add_transition("a", Some("a"));
    net.add_arc(
        ichnos_model::petri::ArcEnds::PlaceToTransition(p, a),
        2,
        ichnos_model::petri::ArcKind::Normal,
    )
    .unwrap();
    net.add_output_arc(a, sink).unwrap();
    let im = Marking::from([(p, 1)]);
    let fm = Marking::from([(sink, 1)]);
    let replayer = TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).unwrap();
    let r = replayer.replay(&["a"]).unwrap();
    // One token is missing, but pm4py adds two, so one is left in p.
    assert_eq!(r.missing_tokens, 1);
    assert_eq!(r.reached_marking, Marking::from([(p, 1), (sink, 1)]));
    assert_eq!(r.remaining_tokens, 1);
}

#[test]
fn stops_at_the_first_problem() {
    let (net, im, fm) = silent_then_a();
    let options = TokenReplayOptions::default().stop_immediately_unfit(true);
    let replayer = TokenReplayer::new(&net, &im, &fm, options).unwrap();
    let r = replayer.replay(&["a", "a", "a"]).unwrap();
    assert_eq!(r.transitions_with_problems.len(), 1);
    // One missing token for the stop, none for the final marking it reached.
    assert_eq!(r.missing_tokens, 1);
}

#[test]
fn activities_not_in_the_model() {
    let (net, im, fm) = silent_then_a();
    let replayer = TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).unwrap();
    assert!(replayer.replay(&["a", "x"]).unwrap().is_fit);
    let options = TokenReplayOptions::default().consider_activities_not_in_model_in_fitness(true);
    let replayer = TokenReplayer::new(&net, &im, &fm, options).unwrap();
    assert!(!replayer.replay(&["a", "x"]).unwrap().is_fit);
    assert!(replayer.replay(&["a"]).unwrap().is_fit);
}

#[test]
fn duplicate_labels_prefer_an_enabled_transition() {
    // source -> a1 -> p -> a2 -> sink, both labelled "a".
    let mut net = PetriNet::new("n");
    let source = net.add_place("source");
    let p = net.add_place("p");
    let sink = net.add_place("sink");
    let a1 = net.add_transition("a1", Some("a"));
    let a2 = net.add_transition("a2", Some("a"));
    net.add_input_arc(source, a1).unwrap();
    net.add_output_arc(a1, p).unwrap();
    net.add_input_arc(p, a2).unwrap();
    net.add_output_arc(a2, sink).unwrap();
    let im = Marking::from([(source, 1)]);
    let fm = Marking::from([(sink, 1)]);
    let replayer = TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).unwrap();
    let r = replayer.replay(&["a", "a"]).unwrap();
    assert!(r.is_fit);
    assert_eq!(r.activated_transitions, [a1, a2]);
    // With none enabled, the last transition in name order fires.
    let r = replayer.replay(&["a", "a", "a"]).unwrap();
    assert_eq!(r.transitions_with_problems, [a2]);
}

#[test]
fn rejects_markings_outside_the_net() {
    let (net, im, _) = silent_then_a();
    let mut other = PetriNet::new("other");
    for i in 0..5 {
        other.add_place(format!("q{i}"));
    }
    let foreign = other.place_ids().last().unwrap();
    let fm = Marking::from([(foreign, 1)]);
    assert!(matches!(
        TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()),
        Err(Error::UnknownPlace(p)) if p == foreign
    ));
}

#[test]
fn fitness_of_no_traces_is_zero() {
    let f = TokenReplayFitness::evaluate([]);
    assert_eq!(f.log_fitness, 0.0);
    assert_eq!(f.average_trace_fitness, 0.0);
    assert_eq!(f.percentage_of_fitting_traces, 0.0);
}
