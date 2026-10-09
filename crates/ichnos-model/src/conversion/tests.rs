use std::collections::{BTreeSet, HashSet, VecDeque};

use crate::petri::{AcceptingPetriNet, Marking};
use crate::{Label, Operator, ProcessTree};

pub(crate) type Trace = Vec<Label>;
pub(crate) type Language = BTreeSet<Trace>;

fn concat(a: &Language, b: &Language, max: usize) -> Language {
    let mut out = Language::new();
    for x in a {
        for y in b {
            if x.len() + y.len() <= max {
                out.insert(x.iter().chain(y).cloned().collect());
            }
        }
    }
    out
}

fn shuffle_pair(x: &[Label], y: &[Label], out: &mut Language, prefix: &mut Trace) {
    if x.is_empty() || y.is_empty() {
        out.insert(prefix.iter().chain(x).chain(y).cloned().collect());
        return;
    }
    prefix.push(x[0].clone());
    shuffle_pair(&x[1..], y, out, prefix);
    prefix.pop();
    prefix.push(y[0].clone());
    shuffle_pair(x, &y[1..], out, prefix);
    prefix.pop();
}

fn shuffle(a: &Language, b: &Language, max: usize) -> Language {
    let mut out = Language::new();
    for x in a {
        for y in b {
            if x.len() + y.len() <= max {
                shuffle_pair(x, y, &mut out, &mut Vec::new());
            }
        }
    }
    out
}

fn permutations(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    for p in permutations(n - 1) {
        for i in 0..=p.len() {
            let mut q = p.clone();
            q.insert(i, n - 1);
            out.push(q);
        }
    }
    out
}

/// All traces of the tree with at most `max` activities.
fn tree_language(tree: &ProcessTree, max: usize) -> Language {
    match tree {
        ProcessTree::Tau => Language::from([vec![]]),
        ProcessTree::Activity(l) => Language::from([vec![l.clone()]]),
        ProcessTree::Node(op, children) => {
            let langs: Vec<Language> = children.iter().map(|c| tree_language(c, max)).collect();
            let unit = Language::from([vec![]]);
            match op {
                Operator::Sequence => langs.iter().fold(unit, |acc, l| concat(&acc, l, max)),
                Operator::Xor => langs.into_iter().flatten().collect(),
                Operator::Parallel => langs.iter().fold(unit, |acc, l| shuffle(&acc, l, max)),
                Operator::Or => {
                    let mut out = Language::new();
                    for mask in 1u32..(1 << langs.len()) {
                        let chosen = langs
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| mask & (1 << i) != 0);
                        out.extend(chosen.fold(unit.clone(), |acc, (_, l)| shuffle(&acc, l, max)));
                    }
                    out
                }
                Operator::Interleaving => {
                    let mut out = Language::new();
                    for perm in permutations(langs.len()) {
                        out.extend(
                            perm.iter()
                                .fold(unit.clone(), |acc, &i| concat(&acc, &langs[i], max)),
                        );
                    }
                    out
                }
                Operator::Loop => {
                    let do_lang = &langs[0];
                    let redo: Language = langs[1..].iter().flatten().cloned().collect();
                    let step = concat(&redo, do_lang, max);
                    let mut out = do_lang.clone();
                    let mut frontier = do_lang.clone();
                    while !frontier.is_empty() {
                        let next: Language = concat(&frontier, &step, max)
                            .into_iter()
                            .filter(|t| !out.contains(t))
                            .collect();
                        out.extend(next.iter().cloned());
                        frontier = next;
                    }
                    out
                }
            }
        }
    }
}

/// All traces from the initial to the final marking with at most `max`
/// visible transitions.
/// The traces of up to `max` visible steps that reach the final marking.
/// Does not end on a net where silent transitions alone can produce
/// infinitely many markings.
pub(crate) fn net_language(apn: &AcceptingPetriNet, max: usize) -> Language {
    let net = &apn.net;
    let mut out = Language::new();
    let mut seen: HashSet<(Marking, Trace)> = HashSet::new();
    let start = (apn.initial_marking.clone(), Vec::new());
    seen.insert(start.clone());
    let mut queue = VecDeque::from([start]);
    while let Some((m, trace)) = queue.pop_front() {
        if m == apn.final_marking {
            out.insert(trace.clone());
        }
        for t in net.enabled_transitions(&m) {
            let mut next_trace = trace.clone();
            if let Some(l) = &net.transition(t).label {
                if trace.len() == max {
                    continue;
                }
                next_trace.push(l.clone());
            }
            let next = (net.fire(t, &m).unwrap(), next_trace);
            if seen.insert(next.clone()) {
                queue.push_back(next);
            }
        }
    }
    out
}

/// Trees with the net sizes pm4py 2.7.23.8 produces for them:
/// (places, transitions, arcs, silent transitions).
const CASES: &[(&str, [usize; 4])] = &[
    ("->( 'a', +( 'b', 'c' ), 'd' )", [6, 4, 10, 0]),
    ("*( X( 'a', tau ), 'b' )", [4, 5, 10, 3]),
    ("->( 'a', X( 'b', tau ), O( 'c', 'd' ) )", [12, 13, 42, 9]),
    ("X( ->( 'a', 'b' ), *( 'c', tau ) )", [7, 8, 16, 5]),
    ("+( 'a', ->( 'b', X( 'c', tau ) ) )", [7, 6, 14, 3]),
    ("<>( 'a', ->( 'b', 'c' ) )", [8, 5, 18, 2]),
    ("'a'", [2, 1, 2, 0]),
    ("tau", [2, 1, 2, 1]),
    ("*( ->( 'a', 'b' ), tau )", [5, 5, 10, 3]),
];

#[test]
fn tree_to_petri_net_sizes_match_pm4py() {
    for &(s, expected) in CASES {
        let apn = ProcessTree::parse(s).unwrap().to_petri_net();
        let net = &apn.net;
        let silent = net.transitions().filter(|(_, t)| t.is_silent()).count();
        assert_eq!(
            [
                net.place_count(),
                net.transition_count(),
                net.arc_count(),
                silent
            ],
            expected,
            "net size for {s}"
        );
        assert_eq!(apn.initial_marking.len(), 1);
        assert_eq!(apn.final_marking.len(), 1);
    }
}

#[test]
fn tree_to_petri_net_keeps_language() {
    let extra = [
        "*( 'a', 'b' )",
        "O( 'a', 'b', 'c' )",
        "<>( 'a', 'b', 'c' )",
        "+( *( 'a', tau ), X( 'b', 'c' ) )",
        "->( O( 'a', ->( 'b', 'c' ) ), *( tau, 'd' ) )",
        "X( tau, *( +( 'a', 'b' ), 'c' ) )",
    ];
    let max = 6;
    for s in CASES.iter().map(|c| c.0).chain(extra) {
        let tree = ProcessTree::parse(s).unwrap();
        let net = tree.to_petri_net();
        assert_eq!(
            net_language(&net, max),
            tree_language(&tree, max),
            "language of {s}"
        );
    }
}

#[test]
fn three_child_loop_uses_both_redos() {
    // pm4py converts `*( a, b, c )` with b and c as alternative redo parts.
    let tree = ProcessTree::Node(
        Operator::Loop,
        vec![
            ProcessTree::activity("a"),
            ProcessTree::activity("b"),
            ProcessTree::activity("c"),
        ],
    );
    let net = tree.to_petri_net();
    assert_eq!(
        [
            net.net.place_count(),
            net.net.transition_count(),
            net.net.arc_count()
        ],
        [5, 6, 12]
    );
    assert_eq!(net_language(&net, 3), tree_language(&tree, 3));
}

#[test]
fn net_footprints_match_tree_footprints_after_conversion() {
    // Tree footprints read interleaving as parallel, so the net, which
    // shows the real directly-follows pairs, differs there.
    for &(s, _) in CASES.iter().filter(|c| !c.0.contains("<>")) {
        let tree = ProcessTree::parse(s).unwrap();
        let apn = tree.to_petri_net();
        let from_net = apn
            .net
            .footprints(&apn.initial_marking, Default::default())
            .unwrap();
        assert_eq!(from_net, tree.footprints().footprints, "footprints of {s}");
    }
}

fn sample_dfg() -> crate::Dfg {
    let mut dfg = crate::Dfg::new();
    for (a, b, n) in [
        ("a", "b", 10),
        ("b", "c", 8),
        ("c", "d", 9),
        ("b", "d", 2),
        ("a", "e", 3),
        ("e", "d", 3),
        ("d", "b", 1),
        ("c", "c", 4),
        ("e", "f", 1),
        ("f", "d", 1),
    ] {
        dfg.add_edge(a, b, n);
    }
    dfg.add_start("a", 13);
    dfg.add_end("d", 12);
    dfg.add_end("c", 1);
    dfg
}

fn sizes(apn: &AcceptingPetriNet) -> [usize; 4] {
    let net = &apn.net;
    [
        net.place_count(),
        net.transition_count(),
        net.arc_count(),
        net.transitions().filter(|(_, t)| t.is_silent()).count(),
    ]
}

/// Traces of a DFG: walks from a start to an end activity along edges.
fn dfg_language(dfg: &crate::Dfg, max: usize) -> Language {
    let mut out = Language::new();
    let mut frontier: Vec<Trace> = dfg
        .start_activities
        .keys()
        .map(|a| vec![a.clone()])
        .collect();
    while let Some(t) = frontier.pop() {
        let last = t.last().unwrap();
        if dfg.end_activities.contains_key(last) {
            out.insert(t.clone());
        }
        if t.len() < max {
            for b in dfg.outgoing(last).keys() {
                let mut n = t.clone();
                n.push((*b).clone());
                frontier.push(n);
            }
        }
    }
    out
}

#[test]
fn dfg_to_petri_net_matches_pm4py() {
    let dfg = sample_dfg();
    // Sizes from pm4py 2.7.23.8 with explicit start and end activities.
    let apn = dfg.to_petri_net();
    assert_eq!(sizes(&apn), [8, 13, 26, 2]);
    assert_eq!(net_language(&apn, 6), dfg_language(&dfg, 6));
    let apn = dfg.to_petri_net_invisibles_no_duplicates();
    assert_eq!(sizes(&apn), [16, 21, 42, 15]);
    assert_eq!(net_language(&apn, 6), dfg_language(&dfg, 6));
}

#[test]
fn dfg_to_petri_net_infers_missing_start_and_end() {
    let mut dfg = crate::Dfg::new();
    dfg.add_edge("a", "b", 1);
    dfg.add_edge("b", "c", 1);
    let apn = dfg.to_petri_net();
    let abc: Trace = ["a", "b", "c"].map(Label::from).to_vec();
    assert_eq!(net_language(&apn, 5), Language::from([abc]));
}
