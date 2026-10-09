use ichnos_golden::{Tolerance, assert_close, golden};
use ichnos_model::{ProcessTree, comparison::Model};
#[test]
fn behavioral_similarity_matches_tree_net_and_powl() {
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
}
