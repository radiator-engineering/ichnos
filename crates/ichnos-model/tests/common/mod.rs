//! Helpers shared by the golden tests.

use std::collections::BTreeMap;

use ichnos_model::conversion::WfNetToPowlError;
use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::powl::Powl;
use ichnos_model::{AcceptingPetriNet, Label, Marking, Operator, PetriNet, PlaceId, ProcessTree};
use serde_json::{Value, json};

pub fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {v}"))
}

pub fn build_net(model: &Value) -> (PetriNet, Marking) {
    let mut net = PetriNet::new("golden");
    let mut places = BTreeMap::new();
    for p in model["places"].as_array().expect("places") {
        let name = p.as_str().expect("place name");
        places.insert(name.to_owned(), net.add_place(name));
    }
    let mut transitions = BTreeMap::new();
    for t in model["transitions"].as_array().expect("transitions") {
        let name = str_field(t, "name");
        let label = t["label"].as_str().map(Label::from);
        transitions.insert(name.to_owned(), net.add_transition(name, label));
    }
    for a in model["arcs"].as_array().expect("arcs") {
        let (source, target) = (str_field(a, "source"), str_field(a, "target"));
        let ends = match (places.get(source), transitions.get(target)) {
            (Some(&p), Some(&t)) => ArcEnds::PlaceToTransition(p, t),
            _ => ArcEnds::TransitionToPlace(transitions[source], places[target]),
        };
        let kind = match str_field(a, "type") {
            "normal" => ArcKind::Normal,
            "inhibitor" => ArcKind::Inhibitor,
            "reset" => ArcKind::Reset,
            other => panic!("unknown arc type {other}"),
        };
        let weight = u32::try_from(a["weight"].as_u64().expect("weight")).expect("weight fits");
        net.add_arc(ends, weight, kind).expect("valid arc");
    }
    let initial = marking(&model["initial_marking"], &places);
    (net, initial)
}

fn marking(v: &Value, places: &BTreeMap<String, PlaceId>) -> Marking {
    v.as_object()
        .expect("a marking")
        .iter()
        .map(|(p, n)| {
            let n = u32::try_from(n.as_u64().expect("token count")).expect("count fits");
            (places[p], n)
        })
        .collect()
}

/// Builds the accepting Petri net a golden file describes, with both
/// markings.
#[allow(dead_code)]
pub fn build_accepting(model: &Value) -> AcceptingPetriNet {
    let (net, initial) = build_net(model);
    let places: BTreeMap<String, PlaceId> =
        net.places().map(|(id, p)| (p.name.clone(), id)).collect();
    let fin = marking(&model["final_marking"], &places);
    AcceptingPetriNet::new(net, initial, fin)
}

/// Sorts the children of choice and parallel nodes by their string form.
/// pm4py sorts them by a hash of their labels.
#[allow(dead_code)]
pub fn canonical_tree(tree: &ProcessTree) -> ProcessTree {
    match tree {
        ProcessTree::Node(op, children) => {
            let mut children: Vec<ProcessTree> = children.iter().map(canonical_tree).collect();
            if matches!(op, Operator::Xor | Operator::Parallel) {
                children.sort_by_key(ToString::to_string);
            }
            ProcessTree::Node(*op, children)
        }
        leaf => leaf.clone(),
    }
}

/// Checks a converted process tree against a golden `tree` (pm4py's string
/// form) or `error` (`not_workflow_net` or `not_block_structured`).
#[allow(dead_code)]
pub fn assert_tree<E: std::fmt::Debug + PartialEq>(
    id: &str,
    ours: Result<ProcessTree, E>,
    tree: &Value,
    error: &Value,
    expected_error: impl Fn(&str) -> E,
) {
    match (tree.as_str(), error.as_str()) {
        (Some(theirs), None) => {
            let theirs = ProcessTree::parse(theirs)
                .unwrap_or_else(|e| panic!("{id}: pm4py's tree {theirs:?} does not parse: {e}"))
                .fold();
            let ours = ours.unwrap_or_else(|e| panic!("{id}: ours failed with {e:?}"));
            assert_eq!(canonical_tree(&ours), canonical_tree(&theirs), "{id}: tree");
        }
        (None, Some(error)) => assert_eq!(ours.err(), Some(expected_error(error)), "{id}"),
        (tree, error) => panic!("{id}: golden has tree {tree:?} and error {error:?}"),
    }
}

/// Describes a POWL model as `tools/golden/cases/powl.py` does.
#[allow(dead_code)]
pub fn describe_powl(p: &Powl) -> Value {
    let children = |c: &[Powl]| c.iter().map(describe_powl).collect::<Vec<_>>();
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

/// Sorts choice children, and partial-order children with their order, as
/// `canonical_powl` in `tools/golden/cases/wfnet.py` does, so that two
/// descriptions that differ only in child order become equal.
#[allow(dead_code)]
pub fn canonical_powl(d: &Value) -> Value {
    let kind = str_field(d, "kind");
    if !matches!(kind, "xor" | "loop" | "po") {
        return d.clone();
    }
    let mut children: Vec<Value> = d["children"]
        .as_array()
        .expect("children")
        .iter()
        .map(canonical_powl)
        .collect();
    let mut out = d.clone();
    if kind == "xor" {
        children.sort_by_cached_key(Value::to_string);
    } else if kind == "po" {
        let keys: Vec<String> = children.iter().map(Value::to_string).collect();
        let order: Vec<(usize, usize)> = d["order"]
            .as_array()
            .expect("order")
            .iter()
            .map(|e| {
                let at = |k: usize| usize::try_from(e[k].as_u64().expect("index")).expect("fits");
                (at(0), at(1))
            })
            .collect();
        let rank = |i: usize| {
            let mut before: Vec<&String> = order
                .iter()
                .filter(|e| e.1 == i)
                .map(|e| &keys[e.0])
                .collect();
            let mut after: Vec<&String> = order
                .iter()
                .filter(|e| e.0 == i)
                .map(|e| &keys[e.1])
                .collect();
            before.sort();
            after.sort();
            (&keys[i], before, after)
        };
        let mut perm: Vec<usize> = (0..children.len()).collect();
        perm.sort_by_cached_key(|&i| rank(i));
        let mut pos = vec![0; perm.len()];
        for (new, &old) in perm.iter().enumerate() {
            pos[old] = new;
        }
        let mut new_order: Vec<[usize; 2]> = order.iter().map(|&(a, b)| [pos[a], pos[b]]).collect();
        new_order.sort_unstable();
        children = perm.iter().map(|&i| children[i].clone()).collect();
        out["order"] = json!(new_order);
    }
    out["children"] = Value::Array(children);
    out
}

/// Checks a POWL conversion against a golden `powl` (described, see
/// [`describe_powl`]) or `powl_error`.
#[allow(dead_code)]
pub fn assert_powl(id: &str, ours: Result<Powl, WfNetToPowlError>, powl: &Value, error: &Value) {
    match (powl.is_null(), error.as_str()) {
        (false, None) => {
            let ours = ours.unwrap_or_else(|e| panic!("{id}: ours failed with {e:?}"));
            assert_eq!(
                canonical_powl(&describe_powl(&ours)),
                canonical_powl(powl),
                "{id}: powl"
            );
        }
        (true, Some(error)) => {
            let expected = match error {
                "special_arcs" => WfNetToPowlError::SpecialArcs,
                "not_workflow_net" => WfNetToPowlError::NotWorkflowNet,
                "no_unique_local_start_or_end" => WfNetToPowlError::NoUniqueLocalStartOrEnd,
                "cyclic_order" => WfNetToPowlError::CyclicOrder,
                "no_structure" => WfNetToPowlError::NoStructure,
                other => panic!("{id}: unknown pm4py POWL error {other}"),
            };
            assert_eq!(ours.err(), Some(expected), "{id}: powl");
        }
        (_, error) => panic!("{id}: golden has powl {powl} and powl_error {error:?}"),
    }
}

/// pm4py's tree conversion errors, by golden name.
#[allow(dead_code)]
pub fn tree_error(error: &str) -> ichnos_model::conversion::WfNetToTreeError {
    use ichnos_model::conversion::WfNetToTreeError;
    match error {
        "not_workflow_net" => WfNetToTreeError::NotWorkflowNet,
        "not_block_structured" => WfNetToTreeError::NotBlockStructured,
        other => panic!("unknown pm4py tree error {other}"),
    }
}
