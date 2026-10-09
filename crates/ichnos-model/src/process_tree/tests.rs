use std::collections::BTreeSet;

use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

fn t(s: &str) -> ProcessTree {
    ProcessTree::parse(s).unwrap()
}

#[test]
fn parse_and_display_round_trip() {
    let cases = [
        "->( 'a', +( 'b', 'c' ), 'd' )",
        "*( X( 'a', tau ), 'b' )",
        "->( 'a', X( 'b', tau ), O( 'c', 'd' ) )",
        "<>( 'a', 'b' )",
        "X( 'a b', 'c-d' )",
    ];
    for s in cases {
        assert_eq!(t(s).to_string(), s);
    }
}

#[test]
fn parse_builds_expected_tree() {
    let tree = t("->('a',X('b',tau),*('c', τ))");
    let expected = ProcessTree::sequence([
        ProcessTree::activity("a"),
        ProcessTree::xor([ProcessTree::activity("b"), ProcessTree::Tau]),
        ProcessTree::looped(ProcessTree::activity("c"), ProcessTree::Tau),
    ]);
    assert_eq!(tree, expected);
}

#[test]
fn parse_leaf_roots() {
    assert_eq!(t("'a'"), ProcessTree::activity("a"));
    assert_eq!(t("  tau "), ProcessTree::Tau);
    assert_eq!(ProcessTree::activity("a").to_string(), "a");
}

#[test]
fn parse_errors() {
    assert_eq!(
        ProcessTree::parse("->( 'a', 'b'"),
        Err(ParseError::UnexpectedEnd)
    );
    assert_eq!(
        ProcessTree::parse("'a"),
        Err(ParseError::UnterminatedLabel(0))
    );
    assert_eq!(
        ProcessTree::parse("'a' 'b'"),
        Err(ParseError::TrailingInput(4))
    );
    assert!(matches!(
        ProcessTree::parse("a"),
        Err(ParseError::Unexpected {
            offset: 0,
            found: 'a',
            ..
        })
    ));
    assert!(matches!(
        ProcessTree::parse("->( )"),
        Err(ParseError::Unexpected { found: ')', .. })
    ));
    assert_eq!(
        ProcessTree::parse("*( 'a', 'b', 'c' )"),
        Err(ParseError::Invalid(TreeError::LoopArity(3)))
    );
}

#[test]
fn fold_matches_pm4py() {
    // Expected outputs from pm4py's `generic.fold`.
    let cases = [
        ("->( 'a', tau, 'b' )", "->( 'a', 'b' )"),
        ("X( X( 'a', 'b' ), 'c' )", "X( 'a', 'b', 'c' )"),
        ("->( tau, tau )", "tau"),
        ("+( 'a', ->( 'b' ) )", "+( 'a', 'b' )"),
        ("X( 'a', tau, tau )", "X( 'a', tau )"),
        ("*( tau, tau )", "tau"),
        ("->( 'a', ->( 'b', tau ), 'c' )", "->( 'a', 'b', 'c' )"),
        (
            "->( 'a', +( 'b', 'c' ), 'd' )",
            "->( 'a', +( 'b', 'c' ), 'd' )",
        ),
    ];
    for (input, folded) in cases {
        assert_eq!(t(input).fold().to_string(), folded, "fold of {input}");
    }
}

#[test]
fn trace_lengths_match_pm4py() {
    let cases = [
        ("->( 'a', +( 'b', 'c' ), 'd' )", 4, 4),
        ("*( X( 'a', tau ), 'b' )", 0, 1),
        ("->( 'a', X( 'b', tau ), O( 'c', 'd' ) )", 3, 4),
        ("X( ->( 'a', 'b' ), *( 'c', tau ) )", 1, 2),
    ];
    for (s, min, max) in cases {
        let tree = t(s);
        assert_eq!(tree.min_trace_length(), min, "min of {s}");
        assert_eq!(tree.max_trace_length_without_loops(), max, "max of {s}");
    }
}

#[test]
fn structure_helpers() {
    let tree = t("->( 'a', X( 'b', tau ), 'a' )");
    assert_eq!(tree.height(), 3);
    assert_eq!(tree.node_count(), 6);
    assert_eq!(tree.leaves().count(), 4);
    assert_eq!(
        tree.activities(),
        BTreeSet::from([Label::from("a"), Label::from("b")])
    );
    assert!(tree.validate().is_ok());
    assert_eq!(
        ProcessTree::xor([]).validate(),
        Err(TreeError::EmptyOperator(Operator::Xor))
    );
}

#[test]
fn structural_language_equality_ignores_order_where_allowed() {
    assert!(
        t("+( 'a', X( 'b', 'c' ) )").structurally_language_equal(&t("+( X( 'c', 'b' ), 'a' )"))
    );
    assert!(!t("->( 'a', 'b' )").structurally_language_equal(&t("->( 'b', 'a' )")));
    assert!(!t("X( 'a', 'a' )").structurally_language_equal(&t("X( 'a', 'b' )")));
}

fn labels(trace: &[Label]) -> Vec<&str> {
    trace.iter().map(Label::as_str).collect()
}

#[test]
fn playout_sequence_and_parallel() {
    let tree = t("->( 'a', +( 'b', 'c' ), 'd' )");
    let mut rng = StdRng::seed_from_u64(7);
    let mut seen = BTreeSet::new();
    for trace in tree.play_out(200, &mut rng) {
        let l = labels(&trace).join("");
        assert!(l == "abcd" || l == "acbd", "unexpected trace {l}");
        seen.insert(l);
    }
    assert_eq!(seen.len(), 2);
}

#[test]
fn playout_covers_choices_and_loops() {
    let tree = t("->( X( 'a', tau ), *( 'b', 'c' ), O( 'd', 'e' ), <>( 'f', 'g' ) )");
    let mut rng = StdRng::seed_from_u64(42);
    let mut saw_skip = false;
    let mut saw_repeat = false;
    let mut saw_both_or = false;
    for trace in tree.play_out(500, &mut rng) {
        let s = labels(&trace).join("");
        let rest = s.strip_prefix('a').unwrap_or(&s);
        saw_skip |= !s.starts_with('a');
        // Loop part: b (c b)*.
        let loop_len = rest.find(['d', 'e']).expect("OR runs at least one child");
        let body = &rest[..loop_len];
        assert!(
            body.starts_with('b') && body.ends_with('b'),
            "loop part {body}"
        );
        assert!(
            body.chars()
                .enumerate()
                .all(|(i, c)| c == if i % 2 == 0 { 'b' } else { 'c' }),
            "loop part {body}"
        );
        saw_repeat |= body.len() > 1;
        let tail = &rest[loop_len..];
        let (or_part, il) = tail.split_at(tail.len() - 2);
        assert!(il == "fg" || il == "gf", "interleaving part {il}");
        assert!(
            ["d", "e", "de", "ed"].contains(&or_part),
            "OR part {or_part}"
        );
        saw_both_or |= or_part.len() == 2;
    }
    assert!(saw_skip && saw_repeat && saw_both_or);
}
