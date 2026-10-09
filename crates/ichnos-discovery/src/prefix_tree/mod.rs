//! Prefix-tree discovery from pm4py's `algo.transformation.log_to_trie.apply`.
use crate::Result;
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::Label;
use std::collections::BTreeMap;

/// Prefix-tree discovery options.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrefixTreeOptions {
    /// Trim each trace to at most this many activities. None means unlimited.
    /// Zero returns only the non-final root, as in pm4py.
    pub max_path_length: Option<usize>,
}

/// A node in a prefix-tree arena. The root is node zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixNode {
    /// Visible label; only the root is unlabelled.
    pub label: Option<Label>,
    /// Parent arena index; only the root has no parent.
    pub parent: Option<usize>,
    /// Child arena indices, keyed by label in stable lexical order.
    pub children: BTreeMap<Label, usize>,
    /// Whether a nonempty, possibly truncated trace ends here.
    pub final_node: bool,
    /// Number of visible activities from the root.
    pub depth: usize,
}

/// A prefix tree with stable arena indices and no reference cycles.
/// Trace frequencies are intentionally absent: this represents unique prefixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixTree {
    /// Nodes in first-creation order, beginning with the root.
    pub nodes: Vec<PrefixNode>,
}

/// Discover unique prefixes, marking the ends of nonempty truncated traces.
/// Empty traces do not mark the root final, matching pm4py's trie algorithm.
/// Activity order is retained; timestamps and case identifiers are unnecessary.
pub fn prefix_tree(
    log: &EventLog,
    keys: &EventKeys,
    options: &PrefixTreeOptions,
) -> Result<PrefixTree> {
    let sequences = log.activity_sequences(keys)?;
    let mut tree = PrefixTree {
        nodes: vec![PrefixNode {
            label: None,
            parent: None,
            children: BTreeMap::new(),
            final_node: false,
            depth: 0,
        }],
    };
    for trace in &sequences.traces {
        let mut parent = 0;
        let limit = options.max_path_length.unwrap_or(usize::MAX);
        for &activity in trace.iter().take(limit) {
            let label = Label::from(sequences.activities.name(activity));
            parent = if let Some(&child) = tree.nodes[parent].children.get(&label) {
                child
            } else {
                let child = tree.nodes.len();
                let depth = tree.nodes[parent].depth + 1;
                tree.nodes.push(PrefixNode {
                    label: Some(label.clone()),
                    parent: Some(parent),
                    children: BTreeMap::new(),
                    final_node: false,
                    depth,
                });
                tree.nodes[parent].children.insert(label, child);
                child
            };
        }
        if parent != 0 {
            tree.nodes[parent].final_node = true;
        }
    }
    Ok(tree)
}
