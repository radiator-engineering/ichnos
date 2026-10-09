//! POWL models and their conversions against pm4py's
//! (`fixtures/golden/powl`); see `tools/golden/cases/powl.py`.

use std::collections::BTreeMap;

use ichnos_golden::{JsonCompare, assert_json_eq, cases, golden};
use ichnos_model::powl::{BinaryRelation, Powl, PowlError};
use ichnos_model::{AcceptingPetriNet, Marking, Operator, ProcessTree};
use serde_json::{Value, json};

fn describe(p: &Powl) -> Value {
    let children = |c: &[Powl]| c.iter().map(describe).collect::<Vec<_>>();
    match p {
        Powl::Silent => json!({"kind": "silent"}),
        Powl::Activity(l) => json!({"kind": "activity", "label": l.as_str()}),
        Powl::Frequent(t) => json!({
            "kind": "frequent",
            "activity": t.activity.as_str(),
            "skippable": t.skippable,
            "selfloop": t.selfloop,
            "label": t.label().as_str(),
        }),
        Powl::Xor(c) => json!({"kind": "xor", "children": children(c)}),
        Powl::Loop(c) => json!({"kind": "loop", "children": children(&c[..])}),
        Powl::PartialOrder(po) => json!({
            "kind": "po",
            "children": children(po.children()),
            "order": po.order().edges().map(|(i, j)| [i, j]).collect::<Vec<_>>(),
        }),
    }
}

/// Closes every order of a described model under transitivity: ichnos keeps
/// the pairs that pm4py's simplification and tree conversion leave out.
fn close(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(children) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in children.iter_mut() {
            *c = close(c);
        }
    }
    if v["kind"] == "po" {
        let n = v["children"].as_array().expect("children").len();
        let mut r = BinaryRelation::new(n);
        for pair in v["order"].as_array().expect("order") {
            let at = |k: usize| usize::try_from(pair[k].as_u64().expect("index")).expect("fits");
            r.add_edge(at(0), at(1));
        }
        r.add_transitive_edges();
        v["order"] = json!(r.edges().map(|(i, j)| [i, j]).collect::<Vec<_>>());
    }
    v
}

/// pm4py turns a choice or loop of two silent steps into a frequent
/// transition with no activity; ichnos keeps the choice or loop. Puts the
/// node of `original` where `frequent` has such a transition.
fn keep_silent_pairs(frequent: &Value, original: &Value) -> Value {
    if frequent["kind"] == "frequent" && frequent["activity"].is_null() {
        return original.clone();
    }
    let mut v = frequent.clone();
    if let Some(children) = v.get_mut("children").and_then(Value::as_array_mut) {
        for (i, c) in children.iter_mut().enumerate() {
            *c = keep_silent_pairs(c, &original["children"][i]);
        }
    }
    v
}

/// Name (empty when visible), label, preset, postset, visible.
type TransitionRow = (String, String, Vec<String>, Vec<String>, bool);

fn describe_net(apn: &AcceptingPetriNet) -> Value {
    let net = &apn.net;
    let names = |ps: Vec<ichnos_model::PlaceId>| {
        let mut v: Vec<String> = ps.into_iter().map(|p| net.place(p).name.clone()).collect();
        v.sort_unstable();
        v
    };
    let mut places: Vec<&str> = net.places().map(|(_, p)| p.name.as_str()).collect();
    places.sort_unstable();
    let mut transitions: Vec<TransitionRow> = net
        .transitions()
        .map(|(id, t)| {
            // Visible transitions have random UUID names in pm4py.
            let name = if t.label.is_some() {
                String::new()
            } else {
                t.name.clone()
            };
            let label = t.label.as_deref().unwrap_or("").to_owned();
            (
                name,
                label,
                names(net.preset(id).collect()),
                names(net.postset(id).collect()),
                t.label.is_some(),
            )
        })
        .collect();
    transitions.sort();
    let marking = |m: &Marking| -> BTreeMap<&str, u32> {
        m.iter()
            .map(|(p, n)| (net.place(p).name.as_str(), n))
            .collect()
    };
    json!({
        "places": places,
        "transitions": transitions
            .into_iter()
            .map(|(name, label, pre, post, visible)| json!({
                "name": (!visible).then_some(name),
                "label": visible.then_some(label),
                "preset": pre,
                "postset": post,
            }))
            .collect::<Vec<_>>(),
        "initial_marking": marking(&apn.initial_marking),
        "final_marking": marking(&apn.final_marking),
    })
}

/// Sorts the children of parallel nodes: their order in pm4py's tree
/// conversion depends on hashing.
fn canonical(tree: &ProcessTree) -> ProcessTree {
    match tree {
        ProcessTree::Node(op, children) => {
            let mut children: Vec<ProcessTree> = children.iter().map(canonical).collect();
            if *op == Operator::Parallel {
                children.sort_by_key(ToString::to_string);
            }
            ProcessTree::Node(*op, children)
        }
        leaf => leaf.clone(),
    }
}

#[test]
fn powl_models_match_pm4py() {
    let ids = cases("powl");
    assert_eq!(ids.len(), 23, "expected 23 POWL goldens, found {ids:?}");
    let exact = JsonCompare::default();
    for id in ids {
        let g = golden("powl", &id);
        let from_tree = id.starts_with("tree-");
        let p = if from_tree {
            let tree = ProcessTree::parse(g.expected_at("/tree_in").as_str().expect("tree_in"))
                .expect("tree parses");
            tree.to_powl().expect("tree converts")
        } else {
            Powl::parse(g.expected_at("/text").as_str().expect("text")).expect("model parses")
        };
        let fix = |v: &Value| if from_tree { close(v) } else { v.clone() };

        let expected = fix(g.expected_at("/powl"));
        assert_json_eq(&describe(&p), &expected, &exact);
        if let (false, Some(repr)) = (from_tree, g.expected_at("/repr").as_str()) {
            assert_eq!(p.to_string(), repr, "{id}: repr");
        }

        assert_json_eq(
            &describe(&p.simplify()),
            &close(g.expected_at("/simplified")),
            &exact,
        );
        assert_json_eq(
            &describe(&p.simplify_using_frequent_transitions()),
            &keep_silent_pairs(&fix(g.expected_at("/frequent")), &expected),
            &exact,
        );

        let net = p.to_petri_net().expect("no reflexive order");
        assert_json_eq(&describe_net(&net), g.expected_at("/petri_net"), &exact);

        let precise = g.expected_at("/precise").as_bool().expect("precise");
        match (p.to_process_tree_checked(), g.expected_at("/tree").as_str()) {
            (Ok((tree, ours_precise)), Some(theirs)) => {
                // pm4py writes a lone activity without quotes.
                let theirs =
                    ProcessTree::parse(theirs).unwrap_or_else(|_| ProcessTree::activity(theirs));
                assert_eq!(canonical(&tree), canonical(&theirs), "{id}: tree");
                assert_eq!(ours_precise, precise, "{id}: precise");
            }
            (Err(PowlError::Cyclic), None) => {}
            (ours, theirs) => panic!("{id}: tree {ours:?}, pm4py {theirs:?}"),
        }
    }
}
