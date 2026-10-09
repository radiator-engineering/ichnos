use ichnos_golden::{Tolerance, assert_close, golden};
use ichnos_model::{ProcessTree, comparison::Model};
#[test]
fn behavioral_similarity_matches_tree_net_powl_and_bpmn() {
    let g = golden("analysis_remaining", "models");
    let cases = g.expected.as_array().unwrap();
    let trees: Vec<_> = cases
        .iter()
        .map(|c| ProcessTree::parse(c["tree"].as_str().unwrap()).unwrap())
        .collect();
    for (i, t) in trees.iter().enumerate() {
        for (m, key) in [
            (Model::Tree(t.clone()), "behavioral"),
            (Model::Petri(t.to_petri_net()), "net_behavioral"),
            (Model::Powl(t.to_powl().unwrap()), "powl_behavioral"),
            (Model::Bpmn(t.to_bpmn().unwrap()), "bpmn_behavioral"),
        ] {
            for (j, other) in trees.iter().enumerate() {
                let value = ichnos_conformance::footprints::behavioral_similarity(
                    &m,
                    &Model::Tree(other.clone()),
                )
                .unwrap();
                assert_close(value, cases[i][key][j].as_f64().unwrap(), Tolerance::METRIC);
            }
        }
    }
    for (i, tree) in trees.iter().enumerate() {
        let bpmn = Model::Bpmn(tree.to_bpmn().unwrap());
        for (j, other) in trees.iter().enumerate() {
            let value = ichnos_conformance::footprints::behavioral_similarity(
                &bpmn,
                &Model::Bpmn(other.to_bpmn().unwrap()),
            )
            .unwrap();
            assert_close(
                value,
                cases[i]["bpmn_pair_behavioral"][j].as_f64().unwrap(),
                Tolerance::METRIC,
            );
        }
    }
}

#[test]
fn behavioral_similarity_rejects_dfg_in_either_position() {
    let mut dfg = ichnos_model::Dfg::new();
    dfg.add_start("a", 1);
    dfg.add_end("a", 1);
    let graph = Model::Dfg(dfg);
    let tree = Model::Tree(ProcessTree::activity("a"));
    assert!(ichnos_conformance::footprints::behavioral_similarity(&graph, &tree).is_err());
    assert!(ichnos_conformance::footprints::behavioral_similarity(&tree, &graph).is_err());
    assert!(ichnos_conformance::footprints::behavioral_similarity(&graph, &graph).is_err());
}
