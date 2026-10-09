//! Directly- and eventually-follows discovery over event logs.
//!
//! Frequency and performance DFGs follow input event order (pm4py's EventLog
//! path); eventually-follows counts stably sort each trace by start time.
//! Empty traces contribute no boundaries. Activities use core's stringification
//! rules. Performance gaps are clipped to zero; elapsed gaps retain nanoseconds.
//! Typed and ordinary DFG discovery share one implementation and model type.

mod performance;
pub use ichnos_stats::time::{Aggregation, BusinessHours};
pub use performance::{PerformanceDfg, PerformanceDfgOptions, PerformanceSummary, performance_dfg};

use crate::{Error, Result};
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{Dfg, Label};
use std::collections::{BTreeMap, BTreeSet};

/// Frequency DFG options.
#[derive(Clone, Copy, Debug)]
pub struct DfgOptions {
    /// Distance between paired event positions; one means adjacent events.
    /// Zero counts each event's self-pair, as in pm4py's native backend.
    pub window: usize,
    /// Count each distinct edge at most once per trace.
    pub keep_once_per_case: bool,
}
impl Default for DfgOptions {
    fn default() -> Self {
        Self {
            window: 1,
            keep_once_per_case: false,
        }
    }
}

/// Discover edge frequencies and actual trace-boundary frequencies.
/// The input log is not sorted or changed. A window larger than a trace yields
/// no edges for that trace, but its start and end are still counted.
pub fn dfg(log: &EventLog, keys: &EventKeys, options: &DfgOptions) -> Result<Dfg> {
    let sequences = log.activity_sequences(keys)?;
    let labels: Vec<Label> = sequences
        .activities
        .iter()
        .map(|(_, name)| Label::from(name))
        .collect();
    let mut result = Dfg::new();
    for trace in &sequences.traces {
        if let Some(&first) = trace.first() {
            result.add_start(labels[first.index()].clone(), 1);
            result.add_end(labels[trace.last().unwrap().index()].clone(), 1);
        }
        let mut seen = BTreeSet::new();
        for i in options.window..trace.len() {
            let edge = (trace[i - options.window], trace[i]);
            if !options.keep_once_per_case || seen.insert(edge) {
                result.add_edge(
                    labels[edge.0.index()].clone(),
                    labels[edge.1.index()].clone(),
                    1,
                );
            }
        }
    }
    Ok(result)
}

/// Typed DFG discovery, sharing [`fn@dfg`]'s model and behavior.
/// Accepts the canonical EventLog, including empty and boundary-only traces.
pub fn dfg_typed(log: &EventLog, keys: &EventKeys, options: &DfgOptions) -> Result<Dfg> {
    dfg(log, keys, options)
}

/// Alternate name for [`fn@dfg`], matching pm4py's public alias.
pub fn directly_follows_graph(
    log: &EventLog,
    keys: &EventKeys,
    options: &DfgOptions,
) -> Result<Dfg> {
    dfg(log, keys, options)
}

/// Minimum intervening-event count for each activity repeated within a trace.
/// Activities without a within-trace repetition are omitted, rather than
/// assigned infinity. Adjacent repetitions have distance zero.
pub fn derive_minimum_self_distance(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<BTreeMap<String, usize>> {
    ichnos_stats::cases::get_minimum_self_distances(log, keys).map_err(stats_error)
}

/// Eventually-follows discovery options.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventuallyFollowsOptions {
    /// Use `keys.start_timestamp` for sorting and eligibility; otherwise use
    /// completion timestamps as starts, as pm4py does by default.
    pub use_start_timestamp: bool,
    /// Count only the first temporally eligible later event for each source
    /// event. This is not one follower per activity; the default counts all.
    pub keep_first_following: bool,
}

/// Count event pairs whose source completes at or before the target starts.
/// Equal start times preserve input order. Interval overlaps are excluded.
/// Counts retain multiplicity, even for identical activities in one case.
pub fn eventually_follows_graph(
    log: &EventLog,
    keys: &EventKeys,
    options: &EventuallyFollowsOptions,
) -> Result<BTreeMap<(Label, Label), u64>> {
    let options = ichnos_stats::time::RelationOptions {
        time: ichnos_stats::time::TimeOptions {
            use_start_timestamp: options.use_start_timestamp,
            ..Default::default()
        },
        keep_first_following: options.keep_first_following,
        ..Default::default()
    };
    Ok(
        ichnos_stats::time::get_eventually_follows(log, keys, &options)
            .map_err(stats_error)?
            .into_iter()
            .map(|((a, b), n)| ((Label::from(a), Label::from(b)), n as u64))
            .collect(),
    )
}

fn stats_error(error: ichnos_stats::Error) -> Error {
    match error {
        ichnos_stats::Error::Core(error) => Error::Core(error),
        error => Error::Stats(error),
    }
}
