//! Behavioural footprints of models, ported from pm4py's
//! `algo/discovery/footprints/petri/variants/reach_graph.py` and
//! `algo/discovery/footprints/tree/variants/bottomup.py`.
//!
//! Footprints give a model-independent fingerprint of behaviour: which
//! activities exist, which can start a trace, and which pairs can follow each
//! other directly in one order only (`sequence`) or in both orders
//! (`parallel`). The golden harness compares them against pm4py.

use std::collections::{BTreeSet, HashSet};

use crate::Label;
use crate::petri::{Marking, PetriNet, ReachabilityError, ReachabilityOptions, TransitionId};
use crate::process_tree::{LoopRedo, Operator, ProcessTree};

/// A pair of activities.
pub type LabelPair = (Label, Label);

/// Footprints shared by every model type.
///
/// With the `serde` feature this serializes as pm4py's footprints dict, with
/// each relation as a list of `[a, b]` pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Footprints {
    /// All visible activities.
    pub activities: BTreeSet<Label>,
    /// Activities that can start a trace.
    pub start_activities: BTreeSet<Label>,
    /// Pairs `(a, b)` where `b` can directly follow `a` but not the reverse.
    pub sequence: BTreeSet<LabelPair>,
    /// Pairs that can follow each other in both orders. Symmetric.
    pub parallel: BTreeSet<LabelPair>,
}

/// Footprints of a process tree, with the extra outputs pm4py computes for
/// trees.
///
/// With the `serde` feature this serializes as one flat object, as pm4py's
/// footprints dict for a tree does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeFootprints {
    /// The shared footprints.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub footprints: Footprints,
    /// Activities that can end a trace.
    pub end_activities: BTreeSet<Label>,
    /// Activities that occur in every trace.
    pub activities_always_happening: BTreeSet<Label>,
    /// `true` if the empty trace is in the language.
    pub skippable: bool,
    /// See [`ProcessTree::min_trace_length`].
    pub min_trace_length: usize,
    /// See [`ProcessTree::max_trace_length_without_loops`].
    #[cfg_attr(feature = "serde", serde(rename = "max_trace_length_wo_loops"))]
    pub max_trace_length_without_loops: usize,
}

/// Moves pairs that occur in both orders from `sequence` to `parallel`
/// (pm4py's `fix_fp`).
fn fix_fp(sequence: &mut BTreeSet<LabelPair>, parallel: &mut BTreeSet<LabelPair>) {
    sequence.retain(|p| !parallel.contains(p));
    let both: Vec<LabelPair> = sequence
        .iter()
        .filter(|(a, b)| sequence.contains(&(b.clone(), a.clone())))
        .cloned()
        .collect();
    for p in both {
        sequence.remove(&p);
        parallel.insert(p);
    }
}

fn product(a: &BTreeSet<Label>, b: &BTreeSet<Label>, out: &mut BTreeSet<LabelPair>) {
    for x in a {
        for y in b {
            out.insert((x.clone(), y.clone()));
        }
    }
}

#[derive(Debug, Clone, Default)]
struct NodeFp {
    start: BTreeSet<Label>,
    end: BTreeSet<Label>,
    activities: BTreeSet<Label>,
    skippable: bool,
    sequence: BTreeSet<LabelPair>,
    parallel: BTreeSet<LabelPair>,
    always: BTreeSet<Label>,
}

fn tree_fp(tree: &ProcessTree) -> NodeFp {
    match tree {
        ProcessTree::Tau => NodeFp {
            skippable: true,
            ..NodeFp::default()
        },
        ProcessTree::Activity(l) => {
            let one = BTreeSet::from([l.clone()]);
            NodeFp {
                start: one.clone(),
                end: one.clone(),
                activities: one.clone(),
                skippable: false,
                always: one,
                ..NodeFp::default()
            }
        }
        ProcessTree::Node(op, children) => {
            let mut fp = match tree.loop_parts() {
                Some((do_part, redo)) => loop_fp(do_part, redo),
                None => {
                    let fps: Vec<NodeFp> = children.iter().map(tree_fp).collect();
                    match op {
                        Operator::Xor => xor_fp(fps),
                        Operator::Sequence => sequence_fp(fps),
                        // pm4py treats OR as parallel and has no rule for
                        // interleaving; interleaving is treated as parallel
                        // here too.
                        _ => parallel_fp(fps),
                    }
                }
            };
            fix_fp(&mut fp.sequence, &mut fp.parallel);
            fp
        }
    }
}

fn union_common(fps: &[NodeFp], fp: &mut NodeFp) {
    for n in fps {
        fp.activities.extend(n.activities.iter().cloned());
        fp.sequence.extend(n.sequence.iter().cloned());
        fp.parallel.extend(n.parallel.iter().cloned());
    }
}

fn xor_fp(fps: Vec<NodeFp>) -> NodeFp {
    let mut fp = NodeFp::default();
    union_common(&fps, &mut fp);
    fp.always = fps.first().map(|n| n.always.clone()).unwrap_or_default();
    for n in &fps {
        fp.start.extend(n.start.iter().cloned());
        fp.end.extend(n.end.iter().cloned());
        fp.skippable |= n.skippable;
        if n.skippable {
            fp.always.clear();
        } else {
            fp.always.retain(|a| n.always.contains(a));
        }
    }
    fp
}

fn parallel_fp(fps: Vec<NodeFp>) -> NodeFp {
    let mut fp = NodeFp {
        skippable: true,
        ..NodeFp::default()
    };
    union_common(&fps, &mut fp);
    for n in &fps {
        fp.start.extend(n.start.iter().cloned());
        fp.end.extend(n.end.iter().cloned());
        fp.skippable &= n.skippable;
        if !n.skippable {
            fp.always.extend(n.always.iter().cloned());
        }
    }
    for (i, a) in fps.iter().enumerate() {
        for b in &fps[i + 1..] {
            product(&a.activities, &b.activities, &mut fp.parallel);
            product(&b.activities, &a.activities, &mut fp.parallel);
        }
    }
    fp
}

fn sequence_fp(fps: Vec<NodeFp>) -> NodeFp {
    let mut fp = NodeFp {
        skippable: true,
        ..NodeFp::default()
    };
    union_common(&fps, &mut fp);
    for n in &fps {
        fp.skippable &= n.skippable;
        if !n.skippable {
            fp.always.extend(n.always.iter().cloned());
        }
    }
    for (i, a) in fps.iter().enumerate() {
        for b in &fps[i + 1..] {
            product(&a.end, &b.start, &mut fp.sequence);
            if !b.skippable {
                break;
            }
        }
    }
    for n in &fps {
        fp.start.extend(n.start.iter().cloned());
        if !n.skippable {
            break;
        }
    }
    for n in fps.iter().rev() {
        fp.end.extend(n.end.iter().cloned());
        if !n.skippable {
            break;
        }
    }
    fp
}

fn loop_fp(do_part: &ProcessTree, redo: LoopRedo<'_>) -> NodeFp {
    let d = tree_fp(do_part);
    let r = match redo {
        LoopRedo::Tau => tree_fp(&ProcessTree::Tau),
        LoopRedo::One(t) => tree_fp(t),
        LoopRedo::Choice(ts) => tree_fp(&ProcessTree::xor(ts.iter().cloned())),
    };
    let mut fp = NodeFp::default();
    union_common(&[d.clone(), r.clone()], &mut fp);
    fp.skippable = d.skippable;
    if !d.skippable {
        fp.always = d.always.clone();
    }
    fp.start.extend(d.start.iter().cloned());
    fp.end.extend(d.end.iter().cloned());
    if d.skippable {
        fp.start.extend(r.start.iter().cloned());
        fp.end.extend(r.end.iter().cloned());
    }
    product(&d.end, &r.start, &mut fp.sequence);
    product(&r.end, &d.start, &mut fp.sequence);
    if d.skippable {
        product(&r.end, &r.start, &mut fp.sequence);
    }
    if r.skippable {
        product(&d.end, &d.start, &mut fp.sequence);
    }
    fp
}

impl ProcessTree {
    /// Computes the footprints of the tree (pm4py's
    /// `footprints_discovery` on a process tree).
    pub fn footprints(&self) -> TreeFootprints {
        let fp = tree_fp(self);
        TreeFootprints {
            footprints: Footprints {
                activities: fp.activities,
                start_activities: fp.start,
                sequence: fp.sequence,
                parallel: fp.parallel,
            },
            end_activities: fp.end,
            activities_always_happening: fp.always,
            skippable: fp.skippable,
            min_trace_length: self.min_trace_length(),
            max_trace_length_without_loops: self.max_trace_length_without_loops(),
        }
    }
}

impl PetriNet {
    /// Computes the footprints of the net from its reachability graph
    /// (pm4py's `footprints_discovery` on a Petri net).
    ///
    /// Fails if the reachability graph exceeds `options.max_markings`.
    pub fn footprints(
        &self,
        initial: &Marking,
        options: ReachabilityOptions,
    ) -> Result<Footprints, ReachabilityError> {
        let graph = self.reachability_graph(initial, options)?;
        let n = graph.len();
        let mut incoming: Vec<BTreeSet<TransitionId>> = vec![BTreeSet::new(); n];
        for (_, t, d) in graph.edges() {
            incoming[d].insert(t);
        }
        let visible = |t: &TransitionId| self.transition(*t).label.is_some();
        let label = |t: TransitionId| {
            self.transition(t)
                .label
                .clone()
                .expect("only visible transitions are mapped to labels")
        };

        let mut sequence: HashSet<(TransitionId, TransitionId)> = HashSet::new();
        let mut s1: HashSet<(TransitionId, TransitionId)> = HashSet::new();
        let mut s2: HashSet<(TransitionId, TransitionId)> = HashSet::new();
        let mut start = BTreeSet::new();
        for (i, m) in graph.markings().iter().enumerate() {
            let input: Vec<TransitionId> = incoming[i].iter().copied().filter(visible).collect();
            let output: BTreeSet<TransitionId> = graph
                .outgoing(i)
                .iter()
                .map(|&(t, _)| t)
                .filter(visible)
                .collect();
            let eventually = self.visible_transitions_eventually_enabled(m, options)?;
            if i == 0 {
                start = eventually.iter().map(|&t| label(t)).collect();
            }
            for &x in &output {
                for &y in &output {
                    if x != y {
                        s1.insert((x, y));
                    }
                }
            }
            for &t1 in &input {
                for &t2 in &eventually {
                    sequence.insert((t1, t2));
                }
                for &t2 in &output {
                    s2.insert((t1, t2));
                }
            }
        }
        let parallel_t: HashSet<(TransitionId, TransitionId)> = s2
            .iter()
            .filter(|&&(x, y)| s2.contains(&(y, x)) && s1.contains(&(x, y)))
            .copied()
            .collect();
        let mut parallel: BTreeSet<LabelPair> = parallel_t
            .iter()
            .map(|&(x, y)| (label(x), label(y)))
            .collect();
        let mut seq: BTreeSet<LabelPair> = sequence
            .iter()
            .filter(|p| !parallel_t.contains(p))
            .map(|&(x, y)| (label(x), label(y)))
            .collect();
        let both: Vec<LabelPair> = seq
            .iter()
            .filter(|(a, b)| seq.contains(&(b.clone(), a.clone())))
            .cloned()
            .collect();
        for p in both {
            seq.remove(&p);
            parallel.insert(p);
        }
        let activities = self
            .transitions()
            .filter_map(|(_, t)| t.label.clone())
            .collect();
        Ok(Footprints {
            activities,
            start_activities: start,
            sequence: seq,
            parallel,
        })
    }
}

#[cfg(test)]
mod tests;
