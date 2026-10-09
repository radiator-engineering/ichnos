//! Workflow net to process tree against pm4py (`fixtures/golden/wfnet`);
//! see `tools/golden/cases/wfnet.py`.

mod common;

use ichnos_golden::{cases, golden};
use ichnos_model::conversion::WfNetToTreeError;
use ichnos_model::{Operator, ProcessTree};

/// Sorts the children of choice and parallel nodes by their string form.
/// pm4py sorts them by a hash of their labels.
fn canonical(tree: &ProcessTree) -> ProcessTree {
    match tree {
        ProcessTree::Node(op, children) => {
            let mut children: Vec<ProcessTree> = children.iter().map(canonical).collect();
            if matches!(op, Operator::Xor | Operator::Parallel) {
                children.sort_by_key(ToString::to_string);
            }
            ProcessTree::Node(*op, children)
        }
        leaf => leaf.clone(),
    }
}

#[test]
fn wf_nets_convert_like_pm4py() {
    let ids = cases("wfnet");
    assert_eq!(ids.len(), 17, "expected 17 wfnet goldens, found {ids:?}");
    for id in ids {
        let g = golden("wfnet", &id);
        let net = common::build_accepting(g.expected_at("/model"));
        let ours = net.to_process_tree();
        match (
            g.expected_at("/tree").as_str(),
            g.expected_at("/error").as_str(),
        ) {
            (Some(theirs), None) => {
                let theirs = ProcessTree::parse(theirs)
                    .unwrap_or_else(|e| panic!("{id}: pm4py's tree {theirs:?} does not parse: {e}"))
                    .fold();
                let ours = ours.unwrap_or_else(|e| panic!("{id}: ours failed with {e:?}"));
                assert_eq!(canonical(&ours), canonical(&theirs), "{id}: tree");
            }
            (None, Some(error)) => {
                let expected = match error {
                    "not_workflow_net" => WfNetToTreeError::NotWorkflowNet,
                    "not_block_structured" => WfNetToTreeError::NotBlockStructured,
                    other => panic!("{id}: unknown pm4py error {other}"),
                };
                assert_eq!(ours, Err(expected), "{id}");
            }
            (tree, error) => panic!("{id}: golden has tree {tree:?} and error {error:?}"),
        }
    }
}
