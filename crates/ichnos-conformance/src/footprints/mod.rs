//! Footprint conformance (pm4py's `algo/conformance/footprints` and
//! `algo/conformance/footprints/util/evaluation`).
//!
//! Footprints compare a log with a model through the pairs of activities
//! that directly follow each other. A pair `(a, b)` of the log is in its
//! *sequence* relation when `b` directly follows `a` but never the reverse,
//! and in its *parallel* relation when both orders occur. A log pair that is
//! in neither relation of the model is a deviation, as are start and end
//! activities the model does not allow.
//!
//! pm4py computes the log side in one of two ways, and so does this module:
//!
//! - From an `EventLog`, pm4py takes the footprints of each trace on its own
//!   (`trace_by_trace`) and checks each trace (`trace_extensive`). This is
//!   [`conformance_diagnostics_footprints`] and [`fitness_footprints`].
//! - From a dataframe, pm4py takes the footprints of the whole log
//!   (`entire_dataframe`) and checks the log once (`log_extensive`). This is
//!   [`conformance_diagnostics_footprints_log`] and
//!   [`fitness_footprints_log`].
//!
//! [`precision_footprints`] gives the same value either way.
//!
//! The model side is a [`ModelFootprints`], built from a Petri net's
//! reachability graph ([`ModelFootprints::of_net`]) or from a process tree
//! ([`ModelFootprints::of_tree`]). Only tree footprints have end activities,
//! activities that always happen and a minimum trace length, so only tree
//! footprints check those.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_core::{EventKeys, EventLog, Variants};
use ichnos_model::footprints::LabelPair;
use ichnos_model::{Footprints, Label, Marking, PetriNet, ProcessTree, TreeFootprints};

use crate::error::Result;

mod net;

/// Footprints of a log or of one trace (pm4py's `entire_event_log` and
/// `trace_by_trace` footprints).
///
/// `ichnos_discovery::LogFootprints` and `ichnos_discovery::TraceFootprints`
/// hold the same footprints as discovery results: they group the activities,
/// start activities and sequence and parallel pairs in a
/// [`Footprints`], and the trace type keeps the
/// trace's activities. This type, with flat fields, is the input of footprint
/// conformance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFootprints {
    /// How often each activity directly follows another.
    pub dfg: BTreeMap<LabelPair, u64>,
    /// Pairs `(a, b)` of the DFG whose reverse `(b, a)` is not in the DFG.
    pub sequence: BTreeSet<LabelPair>,
    /// Pairs of the DFG whose reverse is in the DFG too, self-loops
    /// included.
    pub parallel: BTreeSet<LabelPair>,
    /// The activities that occur.
    pub activities: BTreeSet<Label>,
    /// The first activities of the traces.
    pub start_activities: BTreeSet<Label>,
    /// The last activities of the traces.
    pub end_activities: BTreeSet<Label>,
    /// The length of the shortest trace, 0 for a log without traces.
    pub min_trace_length: usize,
}

impl LogFootprints {
    /// The footprints of the whole log (pm4py's `entire_event_log` and
    /// `entire_dataframe` variants).
    pub fn of_log(log: &EventLog, keys: &EventKeys) -> Result<Self> {
        Ok(Self::of_variants(&log.variants(keys)?))
    }

    /// The footprints of the whole log, from its variants.
    pub fn of_variants(variants: &Variants) -> Self {
        let mut fp = Self::default();
        let mut min = None;
        for v in variants.iter() {
            let trace: Vec<&str> = variants.names(v).collect();
            fp.add(&trace, v.count() as u64);
            min = Some(min.map_or(trace.len(), |m: usize| m.min(trace.len())));
        }
        fp.min_trace_length = min.unwrap_or(0);
        fp.split_dfg();
        fp
    }

    /// The footprints of one trace, given as its activities (one entry of
    /// pm4py's `trace_by_trace` variant).
    pub fn of_trace<S: AsRef<str>>(trace: &[S]) -> Self {
        let mut fp = Self::default();
        fp.add(trace, 1);
        fp.min_trace_length = trace.len();
        fp.split_dfg();
        fp
    }

    fn add<S: AsRef<str>>(&mut self, trace: &[S], count: u64) {
        let label = |a: &S| Label::new(a.as_ref());
        for pair in trace.windows(2) {
            *self
                .dfg
                .entry((label(&pair[0]), label(&pair[1])))
                .or_default() += count;
        }
        self.activities.extend(trace.iter().map(label));
        if let (Some(first), Some(last)) = (trace.first(), trace.last()) {
            self.start_activities.insert(label(first));
            self.end_activities.insert(label(last));
        }
    }

    /// Sorts the DFG pairs into the sequence and parallel relations.
    fn split_dfg(&mut self) {
        for (a, b) in self.dfg.keys() {
            let pair = (a.clone(), b.clone());
            if self.dfg.contains_key(&(b.clone(), a.clone())) {
                self.parallel.insert(pair);
            } else {
                self.sequence.insert(pair);
            }
        }
    }

    /// The footprints of several traces merged into one, each counted the
    /// given number of times (pm4py's `flatten_fp`): the DFGs add up and
    /// the relations and activity sets are unions.
    fn flatten<'a>(fps: impl IntoIterator<Item = (&'a LogFootprints, u64)>) -> Self {
        let mut flat = Self::default();
        let mut min = None;
        for (fp, count) in fps {
            for (pair, n) in &fp.dfg {
                *flat.dfg.entry(pair.clone()).or_default() += n * count;
            }
            flat.sequence.extend(fp.sequence.iter().cloned());
            flat.parallel.extend(fp.parallel.iter().cloned());
            flat.activities.extend(fp.activities.iter().cloned());
            flat.start_activities
                .extend(fp.start_activities.iter().cloned());
            flat.end_activities
                .extend(fp.end_activities.iter().cloned());
            min = Some(min.map_or(fp.min_trace_length, |m: usize| m.min(fp.min_trace_length)));
        }
        flat.min_trace_length = min.unwrap_or(0);
        flat
    }
}

/// Footprints of a model, as pm4py's conformance checks read them.
///
/// The optional outputs are those pm4py computes only for some models: a
/// check that needs one is skipped when it is `None`, as pm4py skips it
/// when the key is missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelFootprints {
    /// Pairs `(a, b)` where `b` can directly follow `a` but not the reverse.
    pub sequence: BTreeSet<LabelPair>,
    /// Pairs that can follow each other in both orders.
    pub parallel: BTreeSet<LabelPair>,
    /// Activities that can start a trace.
    pub start_activities: BTreeSet<Label>,
    /// Activities that can end a trace.
    pub end_activities: Option<BTreeSet<Label>>,
    /// Activities that occur in every trace.
    pub activities_always_happening: Option<BTreeSet<Label>>,
    /// The length of the shortest trace.
    pub min_trace_length: Option<usize>,
}

impl ModelFootprints {
    /// The footprints of an accepting Petri net, from its reachable markings
    /// (pm4py's `discover_footprints(net, im, fm)`), exploring at most
    /// [`ReachabilityOptions::default`](ichnos_model::petri::ReachabilityOptions)'s
    /// number of markings.
    ///
    /// This follows pm4py, unlike [`PetriNet::footprints`]: arcs of every
    /// kind count as normal arcs, and what can follow a transition comes
    /// from pm4py's `get_visible_transitions_eventually_enabled_by_marking`,
    /// which can miss activities reachable only through silent
    /// transitions. Fails with [`crate::Error::UnknownPlace`] if the
    /// marking puts tokens on a place that is not in the net.
    ///
    /// Fails with [`crate::Error::Reachability`] when the reachable
    /// markings, or the markings one eventually-enabled search visits,
    /// exceed that limit. pm4py has no bound on markings: it stops after
    /// 86400 seconds and returns the footprints found so far, and its
    /// eventually-enabled search can loop forever on a silent cycle that
    /// produces tokens.
    pub fn of_net(net: &PetriNet, initial_marking: &Marking) -> Result<Self> {
        Ok(net::net_footprints(net, initial_marking)?.into())
    }

    /// The footprints of a process tree (pm4py's `discover_footprints(tree)`).
    pub fn of_tree(tree: &ProcessTree) -> Self {
        tree.footprints().into()
    }

    fn configurations(&self) -> BTreeSet<&LabelPair> {
        self.sequence.iter().chain(&self.parallel).collect()
    }
}

impl From<Footprints> for ModelFootprints {
    fn from(fp: Footprints) -> Self {
        Self {
            sequence: fp.sequence,
            parallel: fp.parallel,
            start_activities: fp.start_activities,
            ..Self::default()
        }
    }
}

impl From<TreeFootprints> for ModelFootprints {
    fn from(fp: TreeFootprints) -> Self {
        Self {
            end_activities: Some(fp.end_activities),
            activities_always_happening: Some(fp.activities_always_happening),
            min_trace_length: Some(fp.min_trace_length),
            ..fp.footprints.into()
        }
    }
}

/// A log's footprints used as the model side, to compare two logs.
impl From<LogFootprints> for ModelFootprints {
    fn from(fp: LogFootprints) -> Self {
        Self {
            sequence: fp.sequence,
            parallel: fp.parallel,
            start_activities: fp.start_activities,
            end_activities: Some(fp.end_activities),
            activities_always_happening: None,
            min_trace_length: Some(fp.min_trace_length),
        }
    }
}

/// The deviations footprint conformance finds in a log or a trace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FootprintsDeviations {
    /// Pairs in the sequence or parallel relation of the log but in
    /// neither relation of the model.
    pub footprints: BTreeSet<LabelPair>,
    /// Start activities of the log the model cannot start with.
    pub start_activities: BTreeSet<Label>,
    /// End activities of the log the model cannot end with.
    pub end_activities: BTreeSet<Label>,
    /// Activities that always happen in the model but not in the trace.
    /// Always empty for a whole log.
    pub activities_always_happening: BTreeSet<Label>,
    /// Whether the shortest trace is at least as long as the model's
    /// shortest trace.
    pub min_length_fit: bool,
    /// Whether there are no deviations at all.
    pub is_footprints_fit: bool,
}

impl FootprintsDeviations {
    /// Checks the footprints of a whole log (pm4py's `log_extensive`).
    pub fn of_log(log: &LogFootprints, model: &ModelFootprints) -> Self {
        let mut d = Self::compare(log, model);
        d.is_footprints_fit = d.footprints.is_empty()
            && d.start_activities.is_empty()
            && d.end_activities.is_empty()
            && d.min_length_fit;
        d
    }

    /// Checks the footprints of one trace (pm4py's `trace_extensive`, with
    /// `enable_act_always_executed`). To skip the check of the activities
    /// that always happen, set the model's `activities_always_happening` to
    /// `None`.
    pub fn of_trace(trace: &LogFootprints, model: &ModelFootprints) -> Self {
        let mut d = Self::compare(trace, model);
        if let Some(always) = &model.activities_always_happening {
            d.activities_always_happening = always
                .iter()
                .filter(|a| !trace.activities.contains(*a))
                .cloned()
                .collect();
        }
        d.is_footprints_fit = d.footprints.is_empty()
            && d.start_activities.is_empty()
            && d.end_activities.is_empty()
            && d.activities_always_happening.is_empty()
            && d.min_length_fit;
        d
    }

    fn compare(log: &LogFootprints, model: &ModelFootprints) -> Self {
        let model_configurations = model.configurations();
        let missing = |acts: &BTreeSet<Label>, allowed: &BTreeSet<Label>| {
            acts.iter()
                .filter(|a| !allowed.contains(*a))
                .cloned()
                .collect()
        };
        Self {
            footprints: log
                .sequence
                .iter()
                .chain(&log.parallel)
                .filter(|p| !model_configurations.contains(p))
                .cloned()
                .collect(),
            start_activities: missing(&log.start_activities, &model.start_activities),
            end_activities: model
                .end_activities
                .as_ref()
                .map(|ends| missing(&log.end_activities, ends))
                .unwrap_or_default(),
            activities_always_happening: BTreeSet::new(),
            min_length_fit: model
                .min_trace_length
                .is_none_or(|min| log.min_trace_length >= min),
            is_footprints_fit: false,
        }
    }

    /// The deviations of several traces merged into one (pm4py's
    /// `flatten_conf`): the unions of the pairs and of the start and end
    /// activities.
    fn flatten<'a>(all: impl IntoIterator<Item = &'a FootprintsDeviations>) -> Self {
        let mut flat = Self::default();
        for d in all {
            flat.footprints.extend(d.footprints.iter().cloned());
            flat.start_activities
                .extend(d.start_activities.iter().cloned());
            flat.end_activities.extend(d.end_activities.iter().cloned());
        }
        flat
    }
}

/// The pairs of the log that the model does not allow (pm4py's `log_model`
/// variant). Loosely, a pair deviates when it is in neither relation of the
/// model. With `strict`, a sequence pair deviates when it is not a sequence
/// pair of the model, and a parallel pair when it is not a parallel pair.
pub fn footprint_violations(
    log: &LogFootprints,
    model: &ModelFootprints,
    strict: bool,
) -> BTreeSet<LabelPair> {
    if strict {
        let s1 = log.sequence.difference(&model.sequence);
        let s2 = log.parallel.difference(&model.parallel);
        s1.chain(s2).cloned().collect()
    } else {
        let model_configurations = model.configurations();
        log.sequence
            .iter()
            .chain(&log.parallel)
            .filter(|p| !model_configurations.contains(p))
            .cloned()
            .collect()
    }
}

/// Footprints fitness of a log from its footprints and their deviations
/// (pm4py's `fp_fitness` on a whole log). 1 for a log without directly
/// follows pairs.
///
/// With `f` the frequency in the DFG of the deviating pairs over the
/// frequency of all pairs, `n` the number of sequence and parallel pairs,
/// and `s` and `e` the numbers of start and end activities,
///
/// ```text
/// fitness = ((1 - f) * n + s + e - deviating s - deviating e) / (n + s + e)
/// ```
pub fn footprints_fitness(log: &LogFootprints, deviations: &FootprintsDeviations) -> f64 {
    if log.dfg.is_empty() {
        return 1.0;
    }
    let sum_dfg: u64 = log.dfg.values().sum();
    let sum_dev: u64 = deviations
        .footprints
        .iter()
        .map(|p| log.dfg.get(p).copied().unwrap_or(0))
        .sum();
    let configurations = log.sequence.len() + log.parallel.len();
    let boundary = log.start_activities.len() + log.end_activities.len();
    let boundary_dev = deviations.start_activities.len() + deviations.end_activities.len();
    ((1.0 - sum_dev as f64 / sum_dfg as f64) * configurations as f64
        + (boundary as f64 - boundary_dev as f64))
        / (configurations + boundary) as f64
}

/// Footprints precision (pm4py's `fp_precision`): the share of the model's
/// sequence and parallel pairs that are sequence or parallel pairs of the
/// log. 1 for a model without pairs.
pub fn footprints_precision(log: &LogFootprints, model: &ModelFootprints) -> f64 {
    let model_configurations = model.configurations();
    if model_configurations.is_empty() {
        return 1.0;
    }
    let shared = log
        .sequence
        .iter()
        .chain(&log.parallel)
        .collect::<BTreeSet<_>>()
        .intersection(&model_configurations)
        .count();
    shared as f64 / model_configurations.len() as f64
}

/// Footprints fitness of a log checked trace by trace (pm4py's
/// `fitness_footprints` on an `EventLog`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootprintsFitness {
    /// Percentage of traces without deviations, between 0 and 100. 0 for a
    /// log without traces.
    pub percentage_of_fitting_traces: f64,
    /// [`footprints_fitness`] of the merged footprints and deviations of
    /// the traces.
    pub log_fitness: f64,
}

/// The footprints and deviations of each variant of a log.
struct TraceCheck {
    variants: Variants,
    footprints: Vec<LogFootprints>,
    deviations: Vec<FootprintsDeviations>,
}

impl TraceCheck {
    fn new(log: &EventLog, keys: &EventKeys, model: &ModelFootprints) -> Result<Self> {
        let variants = log.variants(keys)?;
        let footprints: Vec<LogFootprints> = variants
            .iter()
            .map(|v| LogFootprints::of_trace(&variants.names(v).collect::<Vec<_>>()))
            .collect();
        let deviations = footprints
            .iter()
            .map(|fp| FootprintsDeviations::of_trace(fp, model))
            .collect();
        Ok(Self {
            variants,
            footprints,
            deviations,
        })
    }
}

/// Footprint conformance of each trace of `log` (pm4py's
/// `conformance_diagnostics_footprints` on an `EventLog`, variant
/// `trace_extensive`), in log order.
pub fn conformance_diagnostics_footprints(
    log: &EventLog,
    keys: &EventKeys,
    model: &ModelFootprints,
) -> Result<Vec<FootprintsDeviations>> {
    let check = TraceCheck::new(log, keys, model)?;
    let mut out = vec![FootprintsDeviations::default(); log.traces.len()];
    for (v, d) in check.variants.iter().zip(&check.deviations) {
        for &t in &v.traces {
            out[t] = d.clone();
        }
    }
    Ok(out)
}

/// Footprint conformance of the whole log (pm4py's
/// `conformance_diagnostics_footprints` on a dataframe, variant
/// `log_extensive`).
pub fn conformance_diagnostics_footprints_log(
    log: &EventLog,
    keys: &EventKeys,
    model: &ModelFootprints,
) -> Result<FootprintsDeviations> {
    Ok(FootprintsDeviations::of_log(
        &LogFootprints::of_log(log, keys)?,
        model,
    ))
}

/// Footprints fitness of `log`, checked trace by trace (pm4py's
/// `fitness_footprints` on an `EventLog`).
pub fn fitness_footprints(
    log: &EventLog,
    keys: &EventKeys,
    model: &ModelFootprints,
) -> Result<FootprintsFitness> {
    let check = TraceCheck::new(log, keys, model)?;
    let traces = log.traces.len();
    let fit: usize = check
        .variants
        .iter()
        .zip(&check.deviations)
        .filter(|(_, d)| d.is_footprints_fit)
        .map(|(v, _)| v.count())
        .sum();
    let flat = LogFootprints::flatten(
        check
            .variants
            .iter()
            .zip(&check.footprints)
            .map(|(v, fp)| (fp, v.count() as u64)),
    );
    let deviations = FootprintsDeviations::flatten(&check.deviations);
    Ok(FootprintsFitness {
        percentage_of_fitting_traces: if traces > 0 {
            fit as f64 / traces as f64 * 100.0
        } else {
            0.0
        },
        log_fitness: footprints_fitness(&flat, &deviations),
    })
}

/// Footprints fitness of the whole log (pm4py's `fitness_footprints` on a
/// dataframe).
pub fn fitness_footprints_log(
    log: &EventLog,
    keys: &EventKeys,
    model: &ModelFootprints,
) -> Result<f64> {
    let fp = LogFootprints::of_log(log, keys)?;
    Ok(footprints_fitness(
        &fp,
        &FootprintsDeviations::of_log(&fp, model),
    ))
}

/// Footprints precision of `log` against the model (pm4py's
/// `precision_footprints`).
///
/// The sequence and parallel pairs of the whole log are the pairs of its
/// DFG, and so are the merged pairs of its traces, so pm4py gives the same
/// value for an `EventLog` and for a dataframe.
pub fn precision_footprints(
    log: &EventLog,
    keys: &EventKeys,
    model: &ModelFootprints,
) -> Result<f64> {
    Ok(footprints_precision(
        &LogFootprints::of_log(log, keys)?,
        model,
    ))
}

#[cfg(test)]
mod tests;

/// Compare two typed models by their sequence and parallel footprints.
/// Petri nets reuse the conformance extractor, including silent routing.
/// DFG inputs return a model-comparison error, as pm4py does.
pub fn behavioral_similarity(
    a: &ichnos_model::comparison::Model,
    b: &ichnos_model::comparison::Model,
) -> Result<f64> {
    use ichnos_model::comparison::Model;
    let extract = |m: &Model| -> Result<ichnos_model::Footprints> {
        Ok(match m {
            Model::Tree(t) => t.footprints().footprints,
            Model::Petri(n) => net::net_footprints(&n.net, &n.initial_marking)?,
            Model::Powl(p) => p.footprints().footprints,
            Model::Dfg(_) => {
                return Err(crate::Error::ModelComparison(
                    "behavioral similarity does not support DFG models".into(),
                ));
            }
            Model::Bpmn(_) => {
                m.to_petri_net()
                    .map_err(|e| crate::Error::ModelComparison(e.to_string()))?
                    .net
                    .to_powl()
                    .map_err(|e| crate::Error::ModelComparison(e.to_string()))?
                    .footprints()
                    .footprints
            }
        })
    };
    Ok(ichnos_model::comparison::behavioral_similarity(
        &extract(a)?,
        &extract(b)?,
    ))
}
