//! The inductive miner family, ported from pm4py's
//! `algo/discovery/inductive/` and the inductive functions of
//! `pm4py/discovery.py`.
//!
//! [`petri_net_inductive`] and [`bpmn_inductive`] convert the tree to a
//! Petri net or a BPMN diagram, as pm4py's `discover_petri_net_inductive`
//! and `discover_bpmn_inductive` do.
//!
//! [`powl_inductive`] runs the POWL miner (pm4py's `discover_powl`). It uses
//! the same recursion, but builds partial orders where IM builds sequence
//! and parallel nodes, and some variants add a partial-order cut; see
//! [`PowlVariant`].
//!
//! The miner splits the input recursively. At each step it tries base cases,
//! then four cuts on the directly-follows graph (exclusive choice, sequence,
//! concurrency, loop), then fall-throughs. The result is a
//! [`ProcessTree`] that every trace of the input fits (IM, IMd) or most
//! traces fit (IMf).
//!
//! Three variants share one implementation; [`InductiveVariant`] picks one:
//!
//! - IM works on the variants of a log.
//! - IMf (infrequent) also works on the variants of a log. It ignores
//!   infrequent behaviour below a noise threshold.
//! - IMd works on a directly-follows graph only.
//!
//! ```
//! use ichnos_core::{EventKeys, EventLog};
//! use ichnos_discovery::{InductiveOptions, process_tree_inductive};
//!
//! let keys = EventKeys::default();
//! let log = EventLog::from_trace_strings(["a,b,c", "a,c,b", "a,d"], ",", &keys);
//! let tree = process_tree_inductive(&log, &keys, &InductiveOptions::default())?;
//! assert_eq!(tree.to_string(), "->( 'a', X( +( 'b', 'c' ), 'd' ) )");
//! # Ok::<(), ichnos_discovery::Error>(())
//! ```
//!
//! # Differences from pm4py
//!
//! - The result is deterministic. Two of pm4py's choices depend on Python's
//!   set order, so its IMf trees can change between runs. ichnos fixes them:
//!   - Exclusive-choice groups come largest first, then by smallest
//!     activity name. IMf uses this order to break ties when it assigns a
//!     trace to a group. pm4py most often lists the largest group first,
//!     but not always.
//!   - A sequence group is ranked by its smallest activity name, where
//!     pm4py takes any member.
//! - pm4py sorts the children of XOR and parallel nodes by the sum of the
//!   MD5 hashes of their labels (`tree_sort`). ichnos keeps the order the
//!   cuts produce. Both orders give the same language.
//! - pm4py's `multi_processing` option is dropped. The fall-through that
//!   would use it tries candidates in sorted order and stops at the first,
//!   as pm4py does without multiprocessing.
//! - A noise threshold outside `[0, 1]` is an error. pm4py accepts any
//!   value.
//! - `InductiveVariant::Imf { noise_threshold: 0.0 }` runs IMf, with
//!   nothing filtered. pm4py runs IM for a threshold of 0;
//!   [`InductiveOptions::from_noise_threshold`] applies pm4py's rule.
//! - Where pm4py would fail, ichnos returns a tree: a strict sequence cut
//!   that merges into one group counts as no cut (pm4py recurses without
//!   end), and an IMd base case whose only activity is an end activity
//!   gives that activity (pm4py raises `IndexError`).
//!
//! # POWL differences from pm4py
//!
//! - The brute-force variant adds up the counts of the variants that
//!   project to the same trace of a group. pm4py keeps the count of the last
//!   such variant. Only the variant filter reads counts, so the models
//!   differ only when `filtering_weight_factor` is above 0.
//! - When the dynamic-clustering variant would merge a cluster with itself
//!   to make its order transitive, ichnos finds no cut and goes on to the
//!   fall-throughs. pm4py recurses without end.
//! - Options out of range are errors before mining starts. pm4py checks
//!   them only when the miner first needs them.
//! - As with [`Powl::simplify`], each order of the result holds the pairs
//!   that transitivity implies; pm4py's can leave them out. Children of a
//!   choice or a partial order can come in a different order, because
//!   pm4py's cuts list groups in Python's set order.

mod cuts;
mod data;
mod fall_through;
mod miner;
mod powl;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use ichnos_core::{EventKeys, EventLog, Variants};
use ichnos_model::{AcceptingPetriNet, Bpmn, Label, Powl, ProcessTree};

use crate::{Error, Result};
use data::{Act, Dfg, Uvcl, add_trace};
use miner::Miner;
use powl::PowlMiner;
pub use powl::{PowlOptions, PowlVariant};

/// Which inductive miner to run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InductiveVariant {
    /// IM: every trace of the log fits the tree.
    Im,
    /// IMf: drops behaviour below `noise_threshold`, a fraction in
    /// `[0, 1]`. Edges rarer than this fraction of their source's most
    /// frequent outgoing edge, and start activities rarer than this fraction
    /// of the most frequent one, are ignored when no cut holds otherwise.
    /// Empty traces become a skip only above this fraction of the traces.
    ///
    /// A threshold of 0 still runs IMf, with nothing filtered. That differs
    /// from pm4py's `discover_process_tree_inductive`, which runs IM for a
    /// threshold of 0. [`InductiveOptions::from_noise_threshold`] applies
    /// pm4py's rule.
    Imf {
        /// The noise threshold; pm4py's `noise_threshold`.
        noise_threshold: f64,
    },
    /// IMd: mines the directly-follows graph of the log only.
    Imd,
}

/// Options for the inductive miner.
///
/// The default runs IM with every cut and fall-through, as pm4py's
/// `discover_process_tree_inductive` does with its defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InductiveOptions {
    /// The variant to run.
    pub variant: InductiveVariant,
    /// Use only the empty-traces and flower-model fall-throughs (pm4py's
    /// `disable_fallthroughs`). IMd has only those two anyway.
    pub disable_fallthroughs: bool,
    /// Use the plain sequence cut instead of the strict one (pm4py's
    /// `disable_strict_sequence_cut`, an `algorithm.apply` parameter).
    pub disable_strict_sequence_cut: bool,
}

impl Default for InductiveOptions {
    fn default() -> Self {
        Self::new(InductiveVariant::Im)
    }
}

impl InductiveOptions {
    /// Options for `variant`, with every cut and fall-through enabled.
    pub fn new(variant: InductiveVariant) -> Self {
        Self {
            variant,
            disable_fallthroughs: false,
            disable_strict_sequence_cut: false,
        }
    }

    /// The variant pm4py's `discover_process_tree_inductive` runs on a log
    /// for `noise_threshold`: IMf if it is above 0, else IM.
    pub fn from_noise_threshold(noise_threshold: f64) -> Self {
        Self::new(if noise_threshold > 0.0 {
            InductiveVariant::Imf { noise_threshold }
        } else {
            InductiveVariant::Im
        })
    }

    /// Sets [`InductiveOptions::disable_fallthroughs`].
    pub fn with_fallthroughs_disabled(mut self, disabled: bool) -> Self {
        self.disable_fallthroughs = disabled;
        self
    }

    /// Sets [`InductiveOptions::disable_strict_sequence_cut`].
    pub fn with_strict_sequence_cut_disabled(mut self, disabled: bool) -> Self {
        self.disable_strict_sequence_cut = disabled;
        self
    }

    fn miner<'a>(&self, labels: &'a [Label]) -> Miner<'a> {
        Miner {
            labels,
            strict_sequence: !self.disable_strict_sequence_cut,
            fall_throughs: !self.disable_fallthroughs,
        }
    }
}

/// Discovers a process tree with the inductive miner (pm4py's
/// `discover_process_tree_inductive` on a log).
///
/// Reads activities from `keys.activity`. pm4py chooses IMf when the noise
/// threshold is above 0; here [`InductiveOptions::variant`] chooses, and
/// [`InductiveOptions::from_noise_threshold`] applies pm4py's rule.
///
/// Fails if an event has no activity or the noise threshold is not in
/// `[0, 1]`.
pub fn process_tree_inductive(
    log: &EventLog,
    keys: &EventKeys,
    options: &InductiveOptions,
) -> Result<ProcessTree> {
    process_tree_inductive_variants(&log.variants(keys)?, options)
}

/// Discovers a process tree from the variants of a log (pm4py's
/// `inductive_miner.apply` on a UVCL).
///
/// Fails if the noise threshold is not in `[0, 1]`.
pub fn process_tree_inductive_variants(
    variants: &Variants,
    options: &InductiveOptions,
) -> Result<ProcessTree> {
    if let InductiveVariant::Imf { noise_threshold } = options.variant
        && !(0.0..=1.0).contains(&noise_threshold)
    {
        return Err(Error::NoiseThreshold(noise_threshold));
    }
    let (labels, log) = to_uvcl(variants);
    let miner = options.miner(&labels);
    let tree = match options.variant {
        InductiveVariant::Im => miner.im(log),
        InductiveVariant::Imf { noise_threshold } => miner.imf(log, noise_threshold),
        InductiveVariant::Imd => {
            let skip = log.contains_key(&[][..]);
            miner.imd(Dfg::from_log(&log), skip)
        }
    };
    Ok(tree.fold())
}

/// Discovers a process tree from a directly-follows graph with IMd (pm4py's
/// `discover_process_tree_inductive` on a `DFG`).
///
/// pm4py runs IMd on a DFG whatever variant is asked for, and so does this
/// function: it reads only the cut and fall-through switches of `options`.
/// A DFG does not record empty traces, so the tree only skips everything if
/// the DFG implies it.
pub fn process_tree_inductive_dfg(
    dfg: &ichnos_model::Dfg,
    options: &InductiveOptions,
) -> ProcessTree {
    let vertices = dfg.vertices();
    let (labels, ids) = sorted_labels(vertices.iter().map(Label::as_str));
    let id = |l: &Label| ids[l.as_str()];
    let freq = |m: &BTreeMap<Label, u64>| m.iter().map(|(a, &n)| (id(a), n)).collect();
    let inner = Dfg {
        graph: dfg
            .graph
            .iter()
            .map(|((a, b), &n)| ((id(a), id(b)), n))
            .collect(),
        start: freq(&dfg.start_activities),
        end: freq(&dfg.end_activities),
    };
    options.miner(&labels).imd(inner, false).fold()
}

/// Discovers an accepting Petri net with the inductive miner: the tree of
/// [`process_tree_inductive`], converted with
/// [`ProcessTree::to_petri_net`] (pm4py's `discover_petri_net_inductive`).
pub fn petri_net_inductive(
    log: &EventLog,
    keys: &EventKeys,
    options: &InductiveOptions,
) -> Result<AcceptingPetriNet> {
    Ok(process_tree_inductive(log, keys, options)?.to_petri_net())
}

/// Discovers an accepting Petri net from a directly-follows graph: the tree
/// of [`process_tree_inductive_dfg`], converted with
/// [`ProcessTree::to_petri_net`].
pub fn petri_net_inductive_dfg(
    dfg: &ichnos_model::Dfg,
    options: &InductiveOptions,
) -> AcceptingPetriNet {
    process_tree_inductive_dfg(dfg, options).to_petri_net()
}

/// Discovers a BPMN diagram with the inductive miner: the tree of
/// [`process_tree_inductive`], converted with [`ProcessTree::to_bpmn`]
/// (pm4py's `discover_bpmn_inductive`).
///
/// Fails if an event has no activity or the noise threshold is not in
/// `[0, 1]`.
pub fn bpmn_inductive(
    log: &EventLog,
    keys: &EventKeys,
    options: &InductiveOptions,
) -> Result<Bpmn> {
    Ok(to_bpmn(&process_tree_inductive(log, keys, options)?))
}

/// Discovers a BPMN diagram from a directly-follows graph: the tree of
/// [`process_tree_inductive_dfg`], converted with [`ProcessTree::to_bpmn`]
/// (pm4py's `discover_bpmn_inductive` on a `DFG`).
pub fn bpmn_inductive_dfg(dfg: &ichnos_model::Dfg, options: &InductiveOptions) -> Bpmn {
    to_bpmn(&process_tree_inductive_dfg(dfg, options))
}

fn to_bpmn(tree: &ProcessTree) -> Bpmn {
    tree.to_bpmn()
        .expect("the inductive miner builds no interleaving nodes")
}

/// Discovers a POWL model with the POWL inductive miner (pm4py's
/// `discover_powl`).
///
/// Reads activities from `keys.activity`. The result is simplified, as
/// pm4py's is.
///
/// Fails if an event has no activity or an option is out of range (see
/// [`PowlOptions`]).
pub fn powl_inductive(log: &EventLog, keys: &EventKeys, options: &PowlOptions) -> Result<Powl> {
    powl_inductive_variants(&log.variants(keys)?, options)
}

/// Discovers a POWL model from the variants of a log (pm4py's
/// `powl.algorithm.apply` on a UVCL).
///
/// Fails if an option is out of range (see [`PowlOptions`]).
pub fn powl_inductive_variants(variants: &Variants, options: &PowlOptions) -> Result<Powl> {
    if let Some(message) = options.invalid() {
        return Err(Error::InvalidOption(message));
    }
    let (labels, log) = to_uvcl(variants);
    let miner = PowlMiner {
        labels: &labels,
        options: *options,
    };
    Ok(miner.mine(log).simplify())
}

/// The activity names in sorted order, and the variants as traces of their
/// numbers.
fn to_uvcl(variants: &Variants) -> (Vec<Label>, Uvcl) {
    let (labels, ids) = sorted_labels(variants.activities.iter().map(|(_, name)| name));
    let mut log = Uvcl::new();
    for v in variants.iter() {
        let trace = v
            .activities
            .iter()
            .map(|a| ids[variants.activities.name(*a)])
            .collect();
        add_trace(&mut log, trace, v.count() as u64);
    }
    (labels, log)
}

/// Numbers the distinct `names` in sorted order. Returns the labels by
/// number and the number of each name.
fn sorted_labels<'a>(names: impl Iterator<Item = &'a str>) -> (Vec<Label>, BTreeMap<&'a str, Act>) {
    let mut sorted: Vec<&str> = names.collect();
    sorted.sort_unstable();
    sorted.dedup();
    let ids = sorted
        .iter()
        .enumerate()
        .map(|(i, &n)| (n, Act::try_from(i).expect("fewer than 2^32 activities")))
        .collect();
    (sorted.into_iter().map(Label::from).collect(), ids)
}
