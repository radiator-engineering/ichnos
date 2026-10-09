use std::collections::BTreeMap;

use super::*;
use crate::ProcessTree;
use crate::conversion::tests::{net_language, tree_language};

fn a(l: &str) -> Powl {
    Powl::activity(l)
}

fn po(children: Vec<Powl>, pairs: &[(usize, usize)]) -> Powl {
    let mut po = StrictPartialOrder::new(children);
    for &(i, j) in pairs {
        po.add_edge(i, j);
    }
    Powl::PartialOrder(po)
}

fn pairs(p: &Powl) -> Vec<(usize, usize)> {
    match p {
        Powl::PartialOrder(po) => po.order().edges().collect(),
        other => panic!("not a partial order: {other}"),
    }
}

const MODEL_DESCRIPTION: &str = "PO=(nodes={ NODE1, NODE2, NODE3, X ( NODE4, NODE5 ) }, \
     order={ NODE1-->NODE2, NODE1-->X ( NODE4, NODE5 ), NODE2-->X ( NODE4, NODE5 ) })";

#[test]
fn relation_start_end_and_reduction() {
    let mut r = BinaryRelation::new(4);
    r.add_edge(0, 1);
    r.add_edge(1, 2);
    r.add_edge(0, 3);
    assert_eq!(r.start_nodes(), vec![0]);
    assert_eq!(r.end_nodes(), vec![2, 3]);
    assert!(!r.is_transitive());
    r.add_transitive_edges();
    assert!(r.is_strict_partial_order());
    assert_eq!(
        r.edges().collect::<Vec<_>>(),
        [(0, 1), (0, 2), (0, 3), (1, 2)]
    );
    let reduced = r.transitive_reduction().unwrap();
    assert_eq!(
        reduced.edges().collect::<Vec<_>>(),
        [(0, 1), (0, 3), (1, 2)]
    );

    r.add_edge(2, 2);
    assert!(!r.is_irreflexive());
    assert_eq!(r.transitive_reduction(), Err(PowlError::Reflexive(2)));
}

#[test]
fn relation_grows_with_its_pairs() {
    let mut po = StrictPartialOrder::sequence([a("a"), a("b")]);
    assert_eq!(po.add_child(a("c")), 2);
    po.add_edge(1, 2);
    assert_eq!(po.order().edges().collect::<Vec<_>>(), [(0, 1), (1, 2)]);
    assert_eq!(po.order().len(), 3);
}

#[test]
fn removing_a_pair_without_violating_transitivity() {
    // pm4py removes j -> k for every i -> j -> k left without i -> k.
    let mut r = BinaryRelation::new(3);
    r.add_edge(0, 1);
    r.add_edge(1, 2);
    r.add_edge(0, 2);
    r.remove_edge_without_violating_transitivity(0, 2);
    assert_eq!(r.edges().collect::<Vec<_>>(), [(0, 1)]);
}

#[test]
fn parses_pm4py_examples() {
    let p = Powl::parse(MODEL_DESCRIPTION).unwrap();
    let expected = po(
        vec![
            a("NODE1"),
            a("NODE2"),
            a("NODE3"),
            Powl::xor([a("NODE4"), a("NODE5")]),
        ],
        &[(0, 1), (0, 3), (1, 3)],
    );
    assert_eq!(p, expected);
    assert_eq!(
        p.to_string(),
        "PO=(nodes={NODE1, NODE2, NODE3, X( 'NODE4', 'NODE5' )}, \
         order={NODE1-->NODE2, NODE1-->X( 'NODE4', 'NODE5' ), NODE2-->X( 'NODE4', 'NODE5' )})"
    );

    let p = Powl::parse("PO=(nodes={ NODE1, NODE2 }, order={ })").unwrap();
    assert_eq!(p, Powl::concurrent([a("NODE1"), a("NODE2")]));
    assert_eq!(Powl::parse(" tau ").unwrap(), Powl::Silent);
    assert_eq!(Powl::parse("'a b'").unwrap(), a("a b"));
    assert_eq!(
        Powl::parse("*( a,\n\ttau )").unwrap(),
        Powl::looped(a("a"), Powl::Silent)
    );
}

#[test]
fn parses_nested_nodes_in_pairs() {
    let inner = "PO=(nodes={ b, c }, order={ b-->c })";
    let s = format!("PO(nodes={{ a, {inner}, d }}, order={{ a-->{inner}, {inner}-->d }})");
    let p = Powl::parse(&s).unwrap();
    assert_eq!(pairs(&p), [(0, 1), (1, 2)]);
    assert_eq!(pairs(&p.children()[1]), [(0, 1)]);
}

#[test]
fn reads_labels_that_start_like_keywords() {
    // pm4py reads these as a choice, a loop and a silent step.
    let p =
        Powl::parse("PO=(nodes={ Xray, *star, tauon, POTATO }, order={ Xray-->tauon })").unwrap();
    assert_eq!(
        p,
        po(
            vec![a("Xray"), a("*star"), a("tauon"), a("POTATO")],
            &[(0, 2)]
        )
    );
}

#[test]
fn parse_errors() {
    assert_eq!(
        Powl::parse("X( a )"),
        Err(PowlParseError::Invalid(PowlError::ChoiceArity(1)))
    );
    assert_eq!(
        Powl::parse("*( a, b, c )"),
        Err(PowlParseError::Invalid(PowlError::LoopArity(3)))
    );
    assert_eq!(
        Powl::parse("PO=(nodes={ a, b }, order={ a-->c })"),
        Err(PowlParseError::UnknownPair("a-->c".into()))
    );
    assert_eq!(
        Powl::parse("PO=(nodes={ a, b }, order={ a->b })"),
        Err(PowlParseError::UnknownPair("a->b".into()))
    );
    assert_eq!(
        Powl::parse("PO=(nodes={ a, a, b }, order={ a-->b })"),
        Err(PowlParseError::AmbiguousNode("a".into()))
    );
    assert_eq!(Powl::parse("'a' b"), Err(PowlParseError::TrailingInput(4)));
    assert_eq!(Powl::parse("'a"), Err(PowlParseError::UnterminatedLabel(0)));
    assert_eq!(Powl::parse("X( a, b"), Err(PowlParseError::UnexpectedEnd));
    assert!(matches!(
        Powl::parse("PO=(nodes={ a }, order={ a-->a )"),
        Err(PowlParseError::Unexpected { found: ')', .. })
    ));
}

#[test]
fn display_reads_back() {
    let models = [
        Powl::parse(MODEL_DESCRIPTION).unwrap(),
        po(
            vec![
                Powl::Silent,
                Powl::looped(a("a"), Powl::Silent),
                a("b, c"),
                a("tau"),
                po(vec![a("d"), a("e")], &[(0, 1)]),
            ],
            &[(0, 1), (0, 2), (1, 3), (2, 4)],
        ),
        Powl::xor([a("a"), Powl::sequence([a("b"), a("c")])]),
    ];
    for m in models {
        let s = m.to_string();
        assert_eq!(Powl::parse(&s).unwrap(), m, "{s}");
    }
}

#[test]
fn validate_checks_choices_and_orders() {
    assert_eq!(Powl::parse(MODEL_DESCRIPTION).unwrap().validate(), Ok(()));
    assert_eq!(
        Powl::xor([a("a")]).validate(),
        Err(PowlError::ChoiceArity(1))
    );
    let chain = po(vec![a("a"), a("b"), a("c")], &[(0, 1), (1, 2)]);
    assert_eq!(chain.validate(), Err(PowlError::NotTransitive(0, 1, 2)));
    let reflexive = Powl::looped(a("x"), po(vec![a("a")], &[(0, 0)]));
    assert_eq!(reflexive.validate(), Err(PowlError::Reflexive(0)));
    assert_eq!(reflexive.to_petri_net(), Err(PowlError::Reflexive(0)));
}

#[test]
fn replaces_labels() {
    let mut p = Powl::xor([
        a("a"),
        Powl::Frequent(FrequentTransition::new("a", true, false)),
        a("b"),
    ]);
    p.replace_labels(&BTreeMap::from([("a".to_owned(), "z".to_owned())]));
    assert_eq!(p.to_string(), "X( 'z', 'z\n[1,1]', 'b' )");
}

#[test]
fn frequent_labels_follow_pm4py() {
    let label = |s, l| FrequentTransition::new("a", s, l).label().into_string();
    assert_eq!(label(false, false), "a");
    assert_eq!(label(true, false), "a\n[1,1]");
    assert_eq!(label(false, true), "a\n[1,-]");
    assert_eq!(label(true, true), "a\n[1,-]");
}

#[test]
fn simplify_merges_nested_orders() {
    let inner = po(vec![a("b"), a("c")], &[(0, 1)]);
    let p = po(
        vec![a("a"), inner.clone(), a("d")],
        &[(0, 1), (0, 2), (1, 2)],
    );
    let s = p.simplify();
    // Kept children first, then the merged ones; pairs closed transitively.
    assert_eq!(s.children(), &[a("a"), a("d"), a("b"), a("c")]);
    assert_eq!(pairs(&s), [(0, 1), (0, 2), (0, 3), (2, 1), (2, 3), (3, 1)]);
    assert_eq!(s.validate(), Ok(()));

    // A nested order with two start children stays.
    let wide = po(vec![a("b"), a("c")], &[]);
    let p = po(vec![a("a"), wide.clone()], &[(0, 1)]);
    assert_eq!(p.simplify(), p);
    // An unordered one is merged.
    let p = po(vec![a("a"), wide], &[]);
    assert_eq!(p.simplify(), Powl::concurrent([a("a"), a("b"), a("c")]));
}

#[test]
fn simplify_merges_choices_and_skipped_loops() {
    let p = Powl::xor([
        a("a"),
        Powl::xor([a("b"), a("c")]),
        Powl::xor([Powl::Silent, Powl::looped(a("d"), Powl::Silent)]),
    ]);
    assert_eq!(
        p.simplify(),
        Powl::xor([a("a"), a("b"), a("c"), Powl::looped(Powl::Silent, a("d")),])
    );
    let p = Powl::xor([Powl::looped(Powl::Silent, a("e")), Powl::Silent]);
    assert_eq!(p.simplify(), Powl::looped(Powl::Silent, a("e")));
}

#[test]
fn frequent_transitions_replace_skips_and_self_loops() {
    let f = |l, s, r| Powl::Frequent(FrequentTransition::new(l, s, r));
    let p = Powl::concurrent([
        Powl::xor([a("a"), Powl::Silent]),
        Powl::looped(a("b"), Powl::Silent),
        Powl::looped(Powl::Silent, a("c")),
        Powl::xor([Powl::Silent, Powl::Silent]),
        Powl::xor([a("d"), a("e")]),
    ]);
    assert_eq!(
        p.simplify_using_frequent_transitions(),
        Powl::concurrent([
            f("a", true, false),
            f("b", false, true),
            f("c", true, true),
            Powl::xor([Powl::Silent, Powl::Silent]),
            Powl::xor([a("d"), a("e")]),
        ])
    );
}

const TREES: &[&str] = &[
    "'a'",
    "tau",
    "->( 'a', 'b', 'c' )",
    "+( 'a', ->( 'b', 'c' ) )",
    "X( 'a', tau, ->( 'b', 'c' ) )",
    "*( 'a', 'b' )",
    "*( ->( 'a', X( 'b', tau ) ), X( 'c', ->( tau, 'd' ) ) )",
    "->( 'a', X( 'b', ->( 'c', 'd' ), tau ), +( 'e', *( 'f', tau ) ), 'g' )",
    "+( ->( 'a', 'b' ), ->( 'c', 'd' ), 'e' )",
    "X( tau, *( +( 'a', 'b' ), 'c' ) )",
];

#[test]
fn trees_convert_to_powl_and_back() {
    let max = 6;
    for s in TREES {
        let tree = ProcessTree::parse(s).unwrap();
        let powl = tree.to_powl().unwrap();
        assert_eq!(powl.validate(), Ok(()), "{s}");
        let (back, precise) = powl.to_process_tree_checked().unwrap();
        assert!(precise, "{s}");
        assert_eq!(tree_language(&back, max), tree_language(&tree, max), "{s}");
        let net = powl.to_petri_net().unwrap();
        assert_eq!(net_language(&net, max), tree_language(&tree, max), "{s}");
        let simplified = powl.simplify();
        assert_eq!(
            net_language(&simplified.to_petri_net().unwrap(), max),
            tree_language(&tree, max),
            "simplified {s}"
        );
    }
}

#[test]
fn tree_operators_powl_cannot_express() {
    for (s, op) in [
        ("O( 'a', 'b' )", Operator::Or),
        ("->( 'a', <>( 'b', 'c' ) )", Operator::Interleaving),
    ] {
        let tree = ProcessTree::parse(s).unwrap();
        assert_eq!(tree.to_powl(), Err(PowlError::UnsupportedOperator(op)));
    }
    let one = ProcessTree::xor([ProcessTree::activity("a")]);
    assert_eq!(one.to_powl(), Err(PowlError::ChoiceArity(1)));
}

#[test]
fn partial_orders_convert_by_levels() {
    let diamond =
        Powl::parse("PO=(nodes={ a, b, c, d }, order={ a-->b, a-->c, a-->d, b-->d, c-->d })")
            .unwrap();
    let (tree, precise) = diamond.to_process_tree_checked().unwrap();
    assert!(precise);
    assert_eq!(tree.to_string(), "->( 'a', +( 'b', 'c' ), 'd' )");

    // b -> d without a -> d: the levels {a, b} and {c, d} are not fully
    // ordered, and the tree also puts a before d.
    let n = Powl::parse("PO=(nodes={ a, b, c, d }, order={ a-->c, b-->c, b-->d })").unwrap();
    let (tree, precise) = n.to_process_tree_checked().unwrap();
    assert!(!precise);
    assert_eq!(tree.to_string(), "->( +( 'a', 'b' ), +( 'c', 'd' ) )");
    let max = 4;
    let model = net_language(&n.to_petri_net().unwrap(), max);
    let trace = |s: &str| {
        s.chars()
            .map(|c| Label::new(c.to_string()))
            .collect::<Vec<_>>()
    };
    assert!(model.contains(&trace("bdac")));
    assert!(tree_language(&tree, max).is_subset(&model));
    assert!(!tree_language(&tree, max).contains(&trace("bdac")));

    let cyclic = Powl::parse("PO=(nodes={ a, b }, order={ a-->b, b-->a })").unwrap();
    assert_eq!(cyclic.to_process_tree(), Err(PowlError::Cyclic));
    // pm4py converts it too: neither child starts or ends the order, so the
    // split has no output places and the join no input places.
    let net = cyclic.to_petri_net().unwrap().net;
    let join = net
        .transitions()
        .find(|(_, t)| t.name.starts_with("tauJoin"))
        .map(|(id, _)| id)
        .unwrap();
    assert_eq!(net.preset(join).count(), 0);
}

#[test]
fn petri_net_names_follow_pm4py() {
    let net = Powl::sequence([a("a"), a("b")]).to_petri_net().unwrap().net;
    let mut places: Vec<&str> = net.places().map(|(_, p)| p.name.as_str()).collect();
    places.sort_unstable();
    assert!(places.contains(&"source") && places.contains(&"sink"));
    assert!(
        places
            .iter()
            .all(|p| *p == "source" || *p == "sink" || p.starts_with("p_"))
    );
    let labels: Vec<_> = net
        .transitions()
        .filter_map(|(_, t)| t.label.clone())
        .collect();
    assert_eq!(labels.len(), 2);
}
