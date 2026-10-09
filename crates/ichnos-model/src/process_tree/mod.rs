//! Process trees, ported from pm4py's `objects/process_tree/`.
//!
//! A [`ProcessTree`] is an owned recursive enum. Leaves are activities or
//! silent steps (tau). Inner nodes apply an [`Operator`] to their children.

mod parse;
mod playout;

pub use parse::ParseError;

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use crate::Label;

/// A process tree operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operator {
    /// `->`: children run one after the other, left to right.
    Sequence,
    /// `X`: exactly one child runs.
    Xor,
    /// `+`: all children run concurrently.
    Parallel,
    /// `*`: the first child (do) runs, then optionally the second (redo)
    /// followed by the do part again, any number of times.
    Loop,
    /// `O`: a non-empty subset of the children runs concurrently.
    Or,
    /// `<>`: all children run, one at a time, in any order.
    Interleaving,
}

impl Operator {
    /// The symbol pm4py uses for this operator.
    pub fn symbol(self) -> &'static str {
        match self {
            Operator::Sequence => "->",
            Operator::Xor => "X",
            Operator::Parallel => "+",
            Operator::Loop => "*",
            Operator::Or => "O",
            Operator::Interleaving => "<>",
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

/// A process tree.
///
/// A [`Operator::Loop`] node is expected to have exactly two children, do and
/// redo, as pm4py produces. Algorithms in this crate read a loop with one
/// child as having a tau redo, and a loop with more than two children as
/// having an XOR of the remaining children as redo. [`ProcessTree::validate`]
/// reports such trees, and the parser rejects them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcessTree {
    /// A silent step.
    Tau,
    /// A visible activity.
    Activity(Label),
    /// An operator applied to child trees.
    Node(Operator, Vec<ProcessTree>),
}

/// Structural problems reported by [`ProcessTree::validate`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    /// An operator node has no children.
    #[error("operator {0} has no children")]
    EmptyOperator(Operator),
    /// A loop node does not have exactly two children.
    #[error("loop has {0} children; expected 2 (do, redo)")]
    LoopArity(usize),
}

impl ProcessTree {
    /// Creates an activity leaf.
    pub fn activity(label: impl Into<Label>) -> Self {
        ProcessTree::Activity(label.into())
    }

    /// Creates an operator node.
    pub fn node(op: Operator, children: impl IntoIterator<Item = ProcessTree>) -> Self {
        ProcessTree::Node(op, children.into_iter().collect())
    }

    /// Creates a sequence node.
    pub fn sequence(children: impl IntoIterator<Item = ProcessTree>) -> Self {
        Self::node(Operator::Sequence, children)
    }

    /// Creates an exclusive-choice node.
    pub fn xor(children: impl IntoIterator<Item = ProcessTree>) -> Self {
        Self::node(Operator::Xor, children)
    }

    /// Creates a parallel node.
    pub fn parallel(children: impl IntoIterator<Item = ProcessTree>) -> Self {
        Self::node(Operator::Parallel, children)
    }

    /// Creates an OR node.
    pub fn or(children: impl IntoIterator<Item = ProcessTree>) -> Self {
        Self::node(Operator::Or, children)
    }

    /// Creates an interleaving node.
    pub fn interleaving(children: impl IntoIterator<Item = ProcessTree>) -> Self {
        Self::node(Operator::Interleaving, children)
    }

    /// Creates a loop node with a do and a redo part.
    pub fn looped(do_part: ProcessTree, redo: ProcessTree) -> Self {
        ProcessTree::Node(Operator::Loop, vec![do_part, redo])
    }

    /// Parses pm4py's string syntax, for example `->( 'a', X( 'b', tau ) )`.
    ///
    /// Same as `str::parse`. See [`ParseError`] for the accepted grammar.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        parse::parse(s)
    }

    /// Returns `true` for a leaf (activity or tau).
    pub fn is_leaf(&self) -> bool {
        !matches!(self, ProcessTree::Node(..))
    }

    /// Returns `true` for a tau leaf.
    pub fn is_tau(&self) -> bool {
        matches!(self, ProcessTree::Tau)
    }

    /// The label of an activity leaf.
    pub fn label(&self) -> Option<&Label> {
        match self {
            ProcessTree::Activity(l) => Some(l),
            _ => None,
        }
    }

    /// The operator of an inner node.
    pub fn operator(&self) -> Option<Operator> {
        match self {
            ProcessTree::Node(op, _) => Some(*op),
            _ => None,
        }
    }

    /// The children of an inner node; empty for a leaf.
    pub fn children(&self) -> &[ProcessTree] {
        match self {
            ProcessTree::Node(_, c) => c,
            _ => &[],
        }
    }

    /// Checks operator arities: every operator has a child and every loop
    /// has exactly two.
    pub fn validate(&self) -> Result<(), TreeError> {
        if let ProcessTree::Node(op, children) = self {
            if children.is_empty() {
                return Err(TreeError::EmptyOperator(*op));
            }
            if *op == Operator::Loop && children.len() != 2 {
                return Err(TreeError::LoopArity(children.len()));
            }
            children.iter().try_for_each(ProcessTree::validate)?;
        }
        Ok(())
    }

    /// Iterates over the leaves, left to right (pm4py's `get_leaves`).
    pub fn leaves(&self) -> impl Iterator<Item = &ProcessTree> {
        let mut stack = vec![self];
        std::iter::from_fn(move || {
            while let Some(n) = stack.pop() {
                match n {
                    ProcessTree::Node(_, c) => stack.extend(c.iter().rev()),
                    leaf => return Some(leaf),
                }
            }
            None
        })
    }

    /// The set of activity labels in the tree.
    pub fn activities(&self) -> BTreeSet<Label> {
        self.leaves().filter_map(|l| l.label().cloned()).collect()
    }

    /// Number of nodes, leaves included.
    pub fn node_count(&self) -> usize {
        1 + self
            .children()
            .iter()
            .map(ProcessTree::node_count)
            .sum::<usize>()
    }

    /// Height of the tree in nodes: 1 for a leaf (pm4py's
    /// `get_process_tree_height`).
    pub fn height(&self) -> usize {
        1 + self
            .children()
            .iter()
            .map(ProcessTree::height)
            .max()
            .unwrap_or(0)
    }

    /// The do and redo parts of a loop, following the reading described on
    /// [`ProcessTree`]. Returns `None` for other nodes.
    pub(crate) fn loop_parts(&self) -> Option<(&ProcessTree, LoopRedo<'_>)> {
        match self {
            ProcessTree::Node(Operator::Loop, c) if !c.is_empty() => Some((
                &c[0],
                match c.len() {
                    1 => LoopRedo::Tau,
                    2 => LoopRedo::One(&c[1]),
                    _ => LoopRedo::Choice(&c[1..]),
                },
            )),
            _ => None,
        }
    }

    /// Minimum number of activities in a trace of this tree, as pm4py's
    /// `bottomup.get_min_trace_length` computes it. Like pm4py, it counts
    /// every child of an OR node, so it can overstate the true minimum.
    pub fn min_trace_length(&self) -> usize {
        match self {
            ProcessTree::Tau => 0,
            ProcessTree::Activity(_) => 1,
            ProcessTree::Node(op, c) => match op {
                Operator::Xor => c.iter().map(Self::min_trace_length).min().unwrap_or(0),
                Operator::Loop => c.first().map_or(0, Self::min_trace_length),
                // pm4py also sums over the children of an OR node, although
                // one child is enough; kept for parity.
                Operator::Sequence | Operator::Parallel | Operator::Interleaving | Operator::Or => {
                    c.iter().map(Self::min_trace_length).sum()
                }
            },
        }
    }

    /// Maximum number of activities in a trace of this tree when every loop
    /// runs its do part once (pm4py's `get_max_trace_length` with
    /// `avoid_loops`).
    pub fn max_trace_length_without_loops(&self) -> usize {
        match self {
            ProcessTree::Tau => 0,
            ProcessTree::Activity(_) => 1,
            ProcessTree::Node(op, c) => match op {
                Operator::Xor => c
                    .iter()
                    .map(Self::max_trace_length_without_loops)
                    .max()
                    .unwrap_or(0),
                Operator::Loop => c.first().map_or(0, Self::max_trace_length_without_loops),
                Operator::Sequence | Operator::Parallel | Operator::Interleaving | Operator::Or => {
                    c.iter().map(Self::max_trace_length_without_loops).sum()
                }
            },
        }
    }

    /// Removes tau leaves that cannot change the language, as pm4py's
    /// `reduce_tau_leafs` does. For example `->( 'a', tau, 'b' )` becomes
    /// `->( 'a', 'b' )`.
    pub fn reduce_tau_leaves(&mut self) {
        let ProcessTree::Node(op, children) = self else {
            return;
        };
        for c in children.iter_mut() {
            c.reduce_tau_leaves();
        }
        let taus = children.iter().filter(|c| c.is_tau()).count();
        if taus == 0 {
            return;
        }
        let keep_one_tau = |children: &mut Vec<ProcessTree>| {
            let mut seen = false;
            children.retain(|c| {
                if !c.is_tau() {
                    return true;
                }
                let keep = !seen;
                seen = true;
                keep
            });
        };
        if taus == children.len() {
            match op {
                Operator::Sequence | Operator::Parallel | Operator::Xor | Operator::Or => {
                    keep_one_tau(children)
                }
                Operator::Loop if children.len() == 2 => children.clear(),
                _ => {}
            }
        } else {
            match op {
                Operator::Sequence | Operator::Parallel => children.retain(|c| !c.is_tau()),
                Operator::Xor | Operator::Or => keep_one_tau(children),
                _ => {}
            }
        }
    }

    /// Simplifies the tree as pm4py's `fold` does: removes useless tau
    /// leaves, replaces operators with one child by that child, drops
    /// operators without children, and flattens nested sequence, XOR and
    /// parallel nodes such as `X( X( 'a', 'b' ), 'c' )` into
    /// `X( 'a', 'b', 'c' )`.
    ///
    /// A tree that folds away entirely becomes [`ProcessTree::Tau`].
    pub fn fold(self) -> ProcessTree {
        let mut tree = self;
        loop {
            let before = tree.clone();
            tree.reduce_tau_leaves();
            tree = fold_once(tree).unwrap_or(ProcessTree::Tau);
            if tree == before {
                return tree;
            }
        }
    }

    /// Compares two trees, ignoring the order of children under parallel,
    /// XOR and OR nodes (pm4py's `structurally_language_equal`).
    pub fn structurally_language_equal(&self, other: &ProcessTree) -> bool {
        match (self, other) {
            (ProcessTree::Tau, ProcessTree::Tau) => true,
            (ProcessTree::Activity(a), ProcessTree::Activity(b)) => a == b,
            (ProcessTree::Node(o1, c1), ProcessTree::Node(o2, c2)) => {
                if o1 != o2 || c1.len() != c2.len() {
                    return false;
                }
                match o1 {
                    Operator::Sequence | Operator::Loop | Operator::Interleaving => c1
                        .iter()
                        .zip(c2)
                        .all(|(a, b)| a.structurally_language_equal(b)),
                    Operator::Parallel | Operator::Xor | Operator::Or => {
                        let mut unmatched: Vec<&ProcessTree> = c2.iter().collect();
                        for a in c1 {
                            match unmatched
                                .iter()
                                .position(|b| a.structurally_language_equal(b))
                            {
                                Some(i) => {
                                    unmatched.swap_remove(i);
                                }
                                None => return false,
                            }
                        }
                        true
                    }
                }
            }
            _ => false,
        }
    }
}

/// The redo part of a loop.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LoopRedo<'a> {
    Tau,
    One(&'a ProcessTree),
    Choice(&'a [ProcessTree]),
}

/// One bottom-up pass of pm4py's `_fold`. Returns `None` when the node
/// disappears.
fn fold_once(tree: ProcessTree) -> Option<ProcessTree> {
    let ProcessTree::Node(op, children) = tree else {
        return Some(tree);
    };
    let mut children: Vec<ProcessTree> = children.into_iter().filter_map(fold_once).collect();
    match children.len() {
        0 => return None,
        1 => return children.pop(),
        _ => {}
    }
    if matches!(op, Operator::Sequence | Operator::Parallel) {
        children.retain(|c| !c.is_tau());
        if children.is_empty() {
            return Some(ProcessTree::Tau);
        }
    }
    if matches!(op, Operator::Sequence | Operator::Xor | Operator::Parallel) {
        let mut flat = Vec::with_capacity(children.len());
        for c in children {
            match c {
                ProcessTree::Node(cop, grand) if cop == op => flat.extend(grand),
                other => flat.push(other),
            }
        }
        children = flat;
    }
    Some(ProcessTree::Node(op, children))
}

impl FromStr for ProcessTree {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::parse(s)
    }
}

/// Formats the tree in pm4py's syntax, matching `ProcessTree.to_string()`:
/// `->( 'a', X( 'b', tau ) )`. A lone activity leaf prints without quotes,
/// as in pm4py.
impl fmt::Display for ProcessTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessTree::Tau => f.write_str("tau"),
            ProcessTree::Activity(l) => f.write_str(l),
            ProcessTree::Node(op, children) => {
                write!(f, "{op}( ")?;
                for (i, c) in children.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    match c {
                        ProcessTree::Activity(l) => write!(f, "'{l}'")?,
                        other => write!(f, "{other}")?,
                    }
                }
                f.write_str(" )")
            }
        }
    }
}

#[cfg(test)]
mod tests;
