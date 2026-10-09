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
    assert_eq!(ids.len(), 16, "expected 16 wfnet goldens, found {ids:?}");
    for id in ids {
        let g = golden("wfnet", &id);
        let net = common::build_accepting(g.expected_at("/model"));
        match (net.to_process_tree(), g.expected_at("/tree").as_str()) {
            (Ok(ours), Some(theirs)) => {
                let theirs = ProcessTree::parse(theirs)
                    .unwrap_or_else(|_| ProcessTree::activity(theirs))
                    .fold();
                assert_eq!(canonical(&ours), canonical(&theirs), "{id}: tree");
            }
            (
                Err(WfNetToTreeError::NotWorkflowNet | WfNetToTreeError::NotBlockStructured),
                None,
            ) => {}
            (ours, theirs) => panic!("{id}: ours {ours:?}, pm4py {theirs:?}"),
        }
    }
}
