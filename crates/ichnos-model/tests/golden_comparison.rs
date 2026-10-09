use ichnos_golden::{Tolerance, assert_close, golden};
use ichnos_model::comparison::{self, Model};
use ichnos_model::{Label, ProcessTree};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn labels(v: &Value) -> BTreeSet<Label> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().into())
        .collect()
}
#[test]
fn fuzzy_labels_match_sequence_matcher() {
    let g = golden("analysis_remaining", "labels");
    let e = &g.expected;
    for (pair, expected) in e["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .zip(e["ratios"].as_array().unwrap())
    {
        assert_close(
            comparison::label_similarity(pair[0].as_str().unwrap(), pair[1].as_str().unwrap()),
            expected.as_f64().unwrap(),
            Tolerance::METRIC,
        );
    }
    let a = labels(&e["a"]);
    let b = labels(&e["b"]);
    assert_close(
        comparison::label_sets_similarity(&a, &b, 0.75).unwrap(),
        e["similarity"].as_f64().unwrap(),
        Tolerance::METRIC,
    );
    assert_eq!(
        json!(comparison::map_labels(&a, &b, 0.75).unwrap()),
        e["mapping"]
    );
    let empty = BTreeSet::new();
    assert_eq!(
        comparison::label_sets_similarity(&empty, &empty, 0.75).unwrap(),
        1.0
    );
    assert!(comparison::label_sets_similarity(&a, &b, f64::NAN).is_err());
    assert!(comparison::map_labels(&a, &b, 1.1).is_err());
    assert_eq!(
        comparison::label_sets_similarity(
            &[Label::from("a")].into(),
            &[Label::from("z")].into(),
            0.0
        )
        .unwrap(),
        0.0
    );
}
#[test]
fn model_labels_and_structural_comparisons_match() {
    let g = golden("analysis_remaining", "models");
    let cases = g.expected.as_array().unwrap();
    let trees: Vec<_> = cases
        .iter()
        .map(|c| ProcessTree::parse(c["tree"].as_str().unwrap()).unwrap())
        .collect();
    let mapping = BTreeMap::from([("A".into(), "Alpha".into())]);
    for (i, c) in cases.iter().enumerate() {
        let tree = Model::Tree(trees[i].clone());
        let net = Model::Petri(trees[i].to_petri_net());
        let powl = Model::Powl(trees[i].to_powl().unwrap());
        let bpmn = Model::Bpmn(trees[i].to_bpmn().unwrap());
        for (m, key, renamed) in [
            (&tree, "labels", "renamed_labels"),
            (&net, "labels", "net_renamed"),
            (&powl, "powl_labels", "powl_renamed"),
            (&bpmn, "bpmn_labels", "bpmn_renamed"),
        ] {
            assert_eq!(m.activity_labels().unwrap(), labels(&c[key]), "{i}: {key}");
            assert_eq!(
                m.replace_activity_labels(&mapping)
                    .unwrap()
                    .activity_labels()
                    .unwrap(),
                labels(&c[renamed]),
                "{i}: {renamed}"
            );
            assert_eq!(
                m.activity_labels().unwrap(),
                labels(&c[key]),
                "input mutated"
            );
        }
        for (j, other) in trees.iter().enumerate() {
            let value =
                comparison::structural_similarity(&tree, &Model::Tree(other.clone())).unwrap();
            assert_close(
                value,
                c["structural"][j].as_f64().unwrap(),
                Tolerance::METRIC,
            );
            // Net conversion may introduce routing steps absent from the tree.
            assert_close(
                comparison::structural_similarity(&net, &Model::Tree(other.clone())).unwrap(),
                c["net_structural"][j].as_f64().unwrap(),
                Tolerance::METRIC,
            );
            assert_close(
                comparison::structural_similarity(&powl, &Model::Tree(other.clone())).unwrap(),
                c["powl_structural"][j].as_f64().unwrap(),
                Tolerance::METRIC,
            );
        }
    }
    let a = Model::Tree(
        ProcessTree::parse("->( 'Archive', 'Approve request', 'Receive request' )").unwrap(),
    );
    let b = Model::Tree(
        ProcessTree::parse("->( 'Archive', 'Approve requests', 'Receive requests' )").unwrap(),
    );
    let label_golden = golden("analysis_remaining", "labels");
    assert_eq!(
        json!(
            a.map_labels_from_second_model(&b, 0.75)
                .unwrap()
                .activity_labels()
                .unwrap()
        ),
        label_golden.expected["mapped_model_labels"]
    );
    assert_eq!(
        a.map_labels_from_second_model(&b, 0.75)
            .unwrap()
            .activity_labels()
            .unwrap(),
        b.activity_labels().unwrap()
    );
    assert_eq!(
        a.label_sets_similarity(&b, 0.75).unwrap(),
        label_golden.expected["model_similarity"].as_f64().unwrap()
    );
}

#[test]
fn malformed_tree_has_a_conversion_error() {
    let model = Model::Tree(ProcessTree::Node(
        ichnos_model::Operator::Loop,
        vec![ProcessTree::Tau],
    ));
    assert!(model.activity_labels().is_err());
}
