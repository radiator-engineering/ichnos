//! ETConformance precision with token replay (pm4py's
//! `algo/evaluation/precision/variants/etconformance_token`).

use std::collections::{BTreeSet, HashMap};

use ichnos_core::{ActivityId, EventKeys, EventLog};
use ichnos_model::{Marking, PetriNet};

use super::{TokenReplayOptions, TokenReplayer};
use crate::error::Result;

/// ETConformance precision of `log` on the accepting net, with token replay
/// (pm4py's `precision_token_based_replay`).
///
/// Each prefix of each trace is replayed with
/// [`TokenReplayOptions::for_prefixes`]. For a prefix that replays without
/// missing tokens, the activities the model allows next are its *activated*
/// labels; those no trace in the log takes after that prefix are *escaping*.
/// Precision is `1 - escaping / activated`, summed over prefixes weighted by
/// how often they occur. The empty prefix counts once per trace, with the
/// log's start activities as the activities taken. Precision is 1 when
/// nothing is activated.
///
/// The activated labels come from pm4py's
/// `get_visible_transitions_eventually_enabled_by_marking`, quirk included
/// (see [`super::TraceReplay::enabled_transitions_in_marking`]).
pub fn precision_token_based_replay(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
) -> Result<f64> {
    let replayer = TokenReplayer::new(
        net,
        initial_marking,
        final_marking,
        TokenReplayOptions::for_prefixes(),
    )?;
    let variants = log.variants(keys)?;

    // Each prefix with the activities that follow it and its count.
    let mut prefixes: HashMap<&[ActivityId], (BTreeSet<ActivityId>, u64)> = HashMap::new();
    let mut start_activities = BTreeSet::new();
    for v in variants.iter() {
        let count = v.count() as u64;
        if let Some(&first) = v.activities.first() {
            start_activities.insert(variants.activities.name(first));
        }
        for i in 1..v.activities.len() {
            let entry = prefixes.entry(&v.activities[..i]).or_default();
            entry.0.insert(v.activities[i]);
            entry.1 += count;
        }
    }

    let label = |t| net.transition(t).label.as_deref();
    let traces = log.traces.len() as u64;
    let initially: BTreeSet<&str> = replayer
        .eventually_enabled(&replayer.initial)?
        .into_iter()
        .filter_map(label)
        .collect();
    let mut activated = traces * initially.len() as u64;
    let mut escaping = traces * initially.difference(&start_activities).count() as u64;

    for (prefix, (next, count)) in &prefixes {
        let names: Vec<&str> = prefix
            .iter()
            .map(|&a| variants.activities.name(a))
            .collect();
        let replay = replayer.replay(&names)?;
        if !replay.is_fit {
            continue;
        }
        let enabled: BTreeSet<&str> = replay
            .enabled_transitions_in_marking
            .iter()
            .filter_map(|&t| label(t))
            .collect();
        let next: BTreeSet<&str> = next.iter().map(|&a| variants.activities.name(a)).collect();
        activated += enabled.len() as u64 * count;
        escaping += enabled.difference(&next).count() as u64 * count;
    }
    Ok(if activated > 0 {
        1.0 - escaping as f64 / activated as f64
    } else {
        1.0
    })
}
