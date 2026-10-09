//! Behavioural footprints of models, ported from pm4py's
//! `algo/discovery/footprints/petri/variants/reach_graph.py`,
//! `algo/discovery/footprints/tree/variants/bottomup.py` and
//! `algo/discovery/footprints/powl/variants/bottomup.py`.
//!
//! Footprints give a model-independent fingerprint of behaviour: which
//! activities exist, which can start a trace, and which pairs can follow each
//! other directly in one order only (`sequence`) or in both orders
//! (`parallel`). The golden harness compares them against pm4py.

use std::collections::{BTreeSet, HashSet};

use crate::Label;
use crate::petri::{Marking, PetriNet, ReachabilityError, ReachabilityOptions, TransitionId};
use crate::powl::Powl;
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

/// Footprints of a POWL model, with the extra outputs pm4py computes for
/// POWL models.
///
/// With the `serde` feature this serializes as one flat object, as pm4py's
/// footprints dict for a POWL model does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PowlFootprints {
    /// The shared footprints.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub footprints: Footprints,
    /// Activities that can end a trace.
    pub end_activities: BTreeSet<Label>,
    /// pm4py's `activities_always_happening` for POWL; see
    /// [`Powl::footprints`].
    pub activities_always_happening: BTreeSet<Label>,
    /// `true` if the empty trace is in the language.
    pub skippable: bool,
    /// pm4py's `min_trace_length` for POWL; see [`Powl::footprints`].
    pub min_trace_length: usize,
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
    /// Computes the footprints of the net from its reachability graph.
    ///
    /// What can follow a transition comes from a complete search of the
    /// markings reachable through silent transitions. pm4py's
    /// `discover_footprints(net, im, fm)` (the `petri_reach_graph` variant)
    /// uses a search that can miss some of them, so its footprints can
    /// differ from these. `ichnos_conformance::footprints::ModelFootprints::of_net`
    /// gives pm4py's values.
    ///
    /// Inhibitor and reset arcs act as such. pm4py explores the state space
    /// with `ClassicSemantics`, which treats them as normal arcs, so on nets
    /// with such arcs the footprints differ (see
    /// [`PetriNet::to_transition_system`]).
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

/// The footprints of one POWL node and its minimum trace length.
fn powl_fp(p: &Powl) -> (NodeFp, usize) {
    match p {
        Powl::Silent => (
            NodeFp {
                skippable: true,
                ..NodeFp::default()
            },
            0,
        ),
        Powl::Activity(_) | Powl::Frequent(_) => {
            let one = BTreeSet::from([p.label().expect("a transition has a label")]);
            let fp = NodeFp {
                start: one.clone(),
                end: one.clone(),
                activities: one.clone(),
                skippable: false,
                always: one,
                ..NodeFp::default()
            };
            (fp, 1)
        }
        Powl::Xor(children) => {
            let fps: Vec<(NodeFp, usize)> = children.iter().map(powl_fp).collect();
            let mut fp = NodeFp::default();
            let mut always: Option<BTreeSet<Label>> = None;
            for (n, _) in &fps {
                fp.start.extend(n.start.iter().cloned());
                fp.end.extend(n.end.iter().cloned());
                fp.activities.extend(n.activities.iter().cloned());
                fp.sequence.extend(n.sequence.iter().cloned());
                fp.parallel.extend(n.parallel.iter().cloned());
                fp.skippable |= n.skippable;
                if !n.skippable {
                    match &mut always {
                        Some(a) => a.retain(|x| n.always.contains(x)),
                        None => always = Some(n.always.clone()),
                    }
                }
            }
            fp.always = always.unwrap_or_default();
            fix_fp(&mut fp.sequence, &mut fp.parallel);
            let min = fps.iter().map(|&(_, m)| m).min().unwrap_or(0);
            (fp, min)
        }
        Powl::Loop(parts) => {
            let (d, min) = powl_fp(&parts[0]);
            let (r, _) = powl_fp(&parts[1]);
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
            fix_fp(&mut fp.sequence, &mut fp.parallel);
            (fp, min)
        }
        Powl::PartialOrder(po) => {
            let fps: Vec<(NodeFp, usize)> = po.children().iter().map(powl_fp).collect();
            let n = fps.len();
            let mut closure = po.order().clone();
            closure.add_transitive_edges();
            let reach = |c: usize, d: usize| closure.is_edge(c, d);
            let mandatory = |c: usize| !fps[c].0.skippable;
            let mut fp = NodeFp {
                skippable: true,
                ..NodeFp::default()
            };
            if let Some((first, _)) = fps.first() {
                fp.always = first.always.clone();
            }
            for (i, (c, _)) in fps.iter().enumerate() {
                fp.activities.extend(c.activities.iter().cloned());
                fp.sequence.extend(c.sequence.iter().cloned());
                fp.parallel.extend(c.parallel.iter().cloned());
                fp.skippable &= c.skippable;
                if i > 0 && !c.skippable {
                    fp.always.extend(c.always.iter().cloned());
                }
            }
            for (c, (child, _)) in fps.iter().enumerate() {
                if !(0..n).any(|p| p != c && mandatory(p) && reach(p, c)) {
                    fp.start.extend(child.start.iter().cloned());
                }
                if !(0..n).any(|q| q != c && mandatory(q) && reach(c, q)) {
                    fp.end.extend(child.end.iter().cloned());
                }
            }
            for c in 0..n {
                for d in 0..n {
                    let skips_to = c != d
                        && reach(c, d)
                        && !(0..n).any(|m| {
                            m != c && m != d && mandatory(m) && reach(c, m) && reach(m, d)
                        });
                    if skips_to {
                        product(&fps[c].0.end, &fps[d].0.start, &mut fp.sequence);
                    }
                }
            }
            for c in 0..n {
                for d in c + 1..n {
                    if !reach(c, d) && !reach(d, c) {
                        let (a, b) = (&fps[c].0.activities, &fps[d].0.activities);
                        product(a, b, &mut fp.parallel);
                        product(b, a, &mut fp.parallel);
                    }
                }
            }
            fix_fp(&mut fp.sequence, &mut fp.parallel);
            let min = fps
                .iter()
                .filter(|(c, _)| !c.skippable)
                .map(|&(_, m)| m)
                .sum();
            (fp, min)
        }
    }
}

impl Powl {
    /// Computes the footprints of the model bottom-up (pm4py's
    /// `footprints_discovery` on a POWL model).
    ///
    /// A partial order gives a sequence pair from the end activities of a
    /// child to the start activities of every later child that only
    /// skippable children can separate from it, and a parallel pair for
    /// every two activities of unordered children. A frequent transition
    /// counts as one activity, named by its label.
    ///
    /// Two rules follow pm4py even where they overstate what always
    /// happens. A choice keeps the activities that always happen in all of
    /// its non-skippable children, even when another child is skippable. A
    /// partial order keeps those of its first child, even when that child
    /// is skippable. The minimum trace length of a choice is that of its
    /// shortest child, of a loop that of its do part, and of a partial
    /// order the sum over its non-skippable children.
    pub fn footprints(&self) -> PowlFootprints {
        let (fp, min_trace_length) = powl_fp(self);
        PowlFootprints {
            footprints: Footprints {
                activities: fp.activities,
                start_activities: fp.start,
                sequence: fp.sequence,
                parallel: fp.parallel,
            },
            end_activities: fp.end,
            activities_always_happening: fp.always,
            skippable: fp.skippable,
            min_trace_length,
        }
    }
}

#[cfg(test)]
mod tests;
