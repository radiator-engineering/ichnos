//! Workflow net to process tree and to POWL against pm4py
//! (`fixtures/golden/wfnet`); see `tools/golden/cases/wfnet.py`.

mod common;

use ichnos_golden::{cases, golden};

fn ids() -> Vec<String> {
    let ids = cases("wfnet");
    assert_eq!(ids.len(), 22, "expected 22 wfnet goldens, found {ids:?}");
    ids
}

#[test]
fn wf_nets_convert_like_pm4py() {
    for id in ids() {
        let g = golden("wfnet", &id);
        let net = common::build_accepting(g.expected_at("/model"));
        common::assert_tree(
            &id,
            net.to_process_tree(),
            g.expected_at("/tree"),
            g.expected_at("/error"),
            common::tree_error,
        );
    }
}

#[test]
fn wf_nets_convert_to_powl_like_pm4py() {
    for id in ids() {
        let g = golden("wfnet", &id);
        let net = common::build_accepting(g.expected_at("/model"));
        common::assert_powl(
            &id,
            net.net.to_powl(),
            g.expected_at("/powl"),
            g.expected_at("/powl_error"),
        );
    }
}

/// A net as sorted place names, transitions, arcs and markings, with
/// visible transitions renamed `t_<n>` in order of label and neighbouring
/// places, as `tools/golden/cases/wfnet.py` renames pm4py's.
fn shape(apn: &ichnos_model::AcceptingPetriNet) -> Vec<String> {
    let net = &apn.net;
    let mut visible: Vec<_> = net
        .transitions()
        .filter(|(_, t)| !t.is_silent())
        .map(|(id, t)| {
            let mut pre: Vec<&str> = net.preset(id).map(|p| net.place(p).name.as_str()).collect();
            let mut post: Vec<&str> = net
                .postset(id)
                .map(|p| net.place(p).name.as_str())
                .collect();
            pre.sort_unstable();
            post.sort_unstable();
            (t.label.clone(), pre, post, id)
        })
        .collect();
    visible.sort();
    let name = |id| match visible.iter().position(|v| v.3 == id) {
        Some(i) => format!("t_{i}"),
        None => net.transition(id).name.clone(),
    };
    let mut out: Vec<String> = net
        .places()
        .map(|(_, p)| format!("place {}", p.name))
        .collect();
    for (id, t) in net.transitions() {
        out.push(format!("transition {} {:?}", name(id), t.label));
        for p in net.preset(id) {
            out.push(format!("arc {} -> {}", net.place(p).name, name(id)));
        }
        for p in net.postset(id) {
            out.push(format!("arc {} -> {}", name(id), net.place(p).name));
        }
    }
    for (key, m) in [
        ("initial", &apn.initial_marking),
        ("final", &apn.final_marking),
    ] {
        for (p, n) in m.iter() {
            out.push(format!("{key} {} {n}", net.place(p).name));
        }
    }
    out.sort();
    out
}

#[test]
fn trees_convert_to_the_nets_pm4py_builds() {
    for id in ids().into_iter().filter(|id| id.starts_with("tree-")) {
        let g = golden("wfnet", &id);
        let tree = g.expected_at("/tree_in").as_str().expect("tree_in");
        let tree = ichnos_model::ProcessTree::parse(tree)
            .unwrap_or_else(|e| panic!("{id}: {tree:?} does not parse: {e}"));
        let theirs = common::build_accepting(g.expected_at("/model"));
        assert_eq!(shape(&tree.to_petri_net()), shape(&theirs), "{id}");
    }
}
