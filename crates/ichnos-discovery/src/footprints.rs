//! Footprints of logs and directly-follows graphs, ported from pm4py's
//! `algo/discovery/footprints/log/variants/` and
//! `algo/discovery/footprints/dfg/variants/dfg.py`.
//!
//! pm4py's `discover_footprints` picks a variant from the type of its
//! argument. Here each variant is its own function, and the model variants
//! are methods in `ichnos-model`:
//!
//! | pm4py argument | ichnos |
//! | --- | --- |
//! | `EventLog` (`trace_by_trace`) | [`trace_footprints`] |
//! | DataFrame (`entire_dataframe`), or `entire_event_log` | [`log_footprints`] |
//! | DFG dict (`dfg`) | [`dfg_footprints`] |
//! | `ProcessTree` (`process_tree`) | [`ProcessTree::footprints`](ichnos_model::ProcessTree::footprints) |
//! | `PetriNet` (`petri_reach_graph`) | `ichnos_conformance::footprints::ModelFootprints::of_net` |
//! | `POWL` (`powl`) | [`Powl::footprints`](ichnos_model::Powl::footprints) |
//!
//! pm4py's `polars_lazyframes` variant computes what `entire_dataframe`
//! does, from a Polars frame; [`log_footprints`] covers it.
//!
//! `ModelFootprints::of_net` follows pm4py's reachability search, which can
//! miss pairs that need silent transitions;
//! [`PetriNet::footprints`](ichnos_model::PetriNet::footprints) explores
//! every marking instead.
//!
//! ```
//! use ichnos_core::{EventKeys, EventLog};
//! use ichnos_discovery::log_footprints;
//!
//! let keys = EventKeys::default();
//! let log = EventLog::from_trace_strings(["a,b,c", "a,c,b"], ",", &keys);
//! let fp = log_footprints(&log, &keys)?;
//! assert!(fp.footprints.parallel.contains(&("b".into(), "c".into())));
//! assert!(fp.footprints.sequence.contains(&("a".into(), "b".into())));
//! assert_eq!(fp.min_trace_length, 3);
//! # Ok::<(), ichnos_discovery::Error>(())
//! ```

use std::collections::{BTreeMap, BTreeSet};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::footprints::LabelPair;
use ichnos_model::{Dfg, Footprints, Label};

use crate::Result;

/// Footprints of a whole log (pm4py's `entire_event_log` and
/// `entire_dataframe` variants).
///
/// `ichnos_conformance::footprints::LogFootprints` has the same name and
/// content but flat fields (no [`Footprints`]) and no trace list; it is the
/// input of footprint conformance. This type is the discovery result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFootprints {
    /// How often the second activity directly follows the first.
    pub dfg: BTreeMap<LabelPair, u64>,
    /// The activities, the first activities of the traces, and the
    /// directly-follows pairs split into sequence and parallel.
    pub footprints: Footprints,
    /// The last activities of the traces.
    pub end_activities: BTreeSet<Label>,
    /// The length of the shortest trace; 0 for a log without traces.
    pub min_trace_length: usize,
}

/// Footprints of one trace (pm4py's `trace_by_trace` variant).
///
/// `ichnos_conformance::footprints::LogFootprints::of_trace` gives the same
/// footprints in the conformance type, with flat fields and without
/// [`TraceFootprints::trace`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceFootprints {
    /// How often the second activity directly follows the first.
    pub dfg: BTreeMap<LabelPair, u64>,
    /// The activities, the first activity, and the directly-follows pairs
    /// split into sequence and parallel.
    pub footprints: Footprints,
    /// The last activity, if the trace has events.
    pub end_activities: BTreeSet<Label>,
    /// The length of the trace (pm4py's `min_trace_length`).
    pub min_trace_length: usize,
    /// The activities of the trace, in order.
    pub trace: Vec<Label>,
}

/// Footprints of a directly-follows graph (pm4py's `dfg` variant).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DfgFootprints {
    /// The activities on edges, the activities with outgoing edges but no
    /// incoming ones, and the edges split into sequence and parallel.
    pub footprints: Footprints,
    /// The activities with incoming edges but no outgoing ones.
    pub end_activities: BTreeSet<Label>,
}

/// Splits directly-follows pairs: `(a, b)` is parallel when `(b, a)` is
/// also a pair, and sequence otherwise.
fn split(pairs: &BTreeMap<LabelPair, u64>) -> (BTreeSet<LabelPair>, BTreeSet<LabelPair>) {
    pairs
        .keys()
        .cloned()
        .partition(|(a, b)| !pairs.contains_key(&(b.clone(), a.clone())))
}

/// Counts each pair of adjacent activities in `traces`.
fn directly_follows<'a>(traces: impl IntoIterator<Item = &'a [Label]>) -> BTreeMap<LabelPair, u64> {
    let mut dfg = BTreeMap::new();
    for trace in traces {
        for w in trace.windows(2) {
            *dfg.entry((w[0].clone(), w[1].clone())).or_insert(0) += 1;
        }
    }
    dfg
}

/// The activity names of each trace, in log order.
fn named_traces(log: &EventLog, keys: &EventKeys) -> Result<Vec<Vec<Label>>> {
    let seqs = log.activity_sequences(keys)?;
    Ok(seqs
        .traces
        .iter()
        .map(|t| {
            t.iter()
                .map(|&a| Label::from(seqs.activities.name(a)))
                .collect()
        })
        .collect())
}

/// Computes the footprints of a whole log (pm4py's `discover_footprints`
/// on a DataFrame).
///
/// Reads activities from `keys.activity`. Fails if an event has no
/// activity.
pub fn log_footprints(log: &EventLog, keys: &EventKeys) -> Result<LogFootprints> {
    let traces = named_traces(log, keys)?;
    let dfg = directly_follows(traces.iter().map(Vec::as_slice));
    let (sequence, parallel) = split(&dfg);
    Ok(LogFootprints {
        footprints: Footprints {
            activities: traces.iter().flatten().cloned().collect(),
            start_activities: traces.iter().filter_map(|t| t.first().cloned()).collect(),
            sequence,
            parallel,
        },
        end_activities: traces.iter().filter_map(|t| t.last().cloned()).collect(),
        min_trace_length: traces.iter().map(Vec::len).min().unwrap_or(0),
        dfg,
    })
}

/// Computes the footprints of each trace, in log order (pm4py's
/// `discover_footprints` on an `EventLog`).
///
/// Reads activities from `keys.activity`. Fails if an event has no
/// activity.
pub fn trace_footprints(log: &EventLog, keys: &EventKeys) -> Result<Vec<TraceFootprints>> {
    Ok(named_traces(log, keys)?
        .into_iter()
        .map(|trace| {
            let dfg = directly_follows([trace.as_slice()]);
            let (sequence, parallel) = split(&dfg);
            TraceFootprints {
                footprints: Footprints {
                    activities: trace.iter().cloned().collect(),
                    start_activities: trace.first().cloned().into_iter().collect(),
                    sequence,
                    parallel,
                },
                end_activities: trace.last().cloned().into_iter().collect(),
                min_trace_length: trace.len(),
                dfg,
                trace,
            }
        })
        .collect())
}

/// Computes the footprints of a directly-follows graph (pm4py's
/// `discover_footprints` on a DFG dict).
///
/// pm4py's DFG dict holds only the edges, so this reads only
/// [`Dfg::graph`]: the start and end activities are inferred from the
/// edges, and an activity without edges is not an activity here.
pub fn dfg_footprints(dfg: &Dfg) -> DfgFootprints {
    let sources: BTreeSet<&Label> = dfg.graph.keys().map(|(a, _)| a).collect();
    let targets: BTreeSet<&Label> = dfg.graph.keys().map(|(_, b)| b).collect();
    let (sequence, parallel) = split(&dfg.graph);
    DfgFootprints {
        footprints: Footprints {
            activities: sources.union(&targets).map(|&a| a.clone()).collect(),
            start_activities: sources.difference(&targets).map(|&a| a.clone()).collect(),
            sequence,
            parallel,
        },
        end_activities: targets.difference(&sources).map(|&a| a.clone()).collect(),
    }
}
