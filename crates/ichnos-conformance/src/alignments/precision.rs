//! Align-ETConformance precision (pm4py's
//! `algo/evaluation/precision/variants/align_etconformance.py`).
//!
//! Every proper prefix of every trace is replayed on the net with
//! synchronous moves and silent model moves only, at the least number of
//! silent moves. In each marking where such a replay ends, the model allows
//! the visible transitions that are enabled now or after silent moves. Those
//! the log never takes after the same prefix are *escaping edges*. Precision
//! is `1 - escaping / allowed`, summed over prefixes weighted by how often
//! they occur, plus the initial marking once per trace.

use std::collections::{BTreeSet, HashMap};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::petri::ReachabilityOptions;
use ichnos_model::{Marking, PetriNet, PlaceId};

use super::costs::{ModelCosts, STD_LOG_MOVE_COST};
use super::marking::{place, tokens};
use super::petri_net::AlignmentOptions;
use super::search::stop_markings;
use super::sync_product::{ModelPart, MoveSet, SyncProduct};
use crate::error::Result;

/// Align-ETConformance precision of `log` on the accepting net (pm4py's
/// `precision_alignments`).
///
/// The prefix replays use pm4py's standard costs, whatever costs an
/// [`Aligner`](super::Aligner) would use. Fails with
/// [`Error::FinalMarkingUnreachable`](crate::Error::FinalMarkingUnreachable) when the net is not easy sound, as
/// pm4py does.
pub fn precision_alignments(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
) -> Result<f64> {
    // pm4py checks easy soundness first; aligning the empty trace does that.
    super::Aligner::new(
        net,
        initial_marking,
        final_marking,
        AlignmentOptions::default(),
    )?;
    let model = ModelPart::new(
        net,
        initial_marking,
        final_marking,
        &ModelCosts::standard(net),
    )?;
    let dense_to_place: Vec<PlaceId> = net.place_ids().collect();

    let variants = log.variants(keys)?;
    // Prefix -> (activities that follow it in the log, number of traces).
    let mut prefixes: HashMap<Vec<&str>, (BTreeSet<&str>, usize)> = HashMap::new();
    let mut start_activities: BTreeSet<&str> = BTreeSet::new();
    for v in variants.iter() {
        let names: Vec<&str> = variants.names(v).collect();
        if let Some(&first) = names.first() {
            start_activities.insert(first);
        }
        for i in 1..names.len() {
            let entry = prefixes.entry(names[..i].to_vec()).or_default();
            entry.0.insert(names[i]);
            entry.1 += v.count();
        }
    }

    let options = ReachabilityOptions::default();
    let mut enabled_cache: HashMap<Marking, BTreeSet<String>> = HashMap::new();
    let mut eventually_enabled = |m: Marking| -> Result<BTreeSet<String>> {
        if let Some(labels) = enabled_cache.get(&m) {
            return Ok(labels.clone());
        }
        let labels: BTreeSet<String> = net
            .visible_transitions_eventually_enabled(&m, options)?
            .into_iter()
            .filter_map(|t| net.transition(t).label.as_ref().map(|l| l.to_string()))
            .collect();
        enabled_cache.insert(m, labels.clone());
        Ok(labels)
    };

    let mut allowed = 0usize;
    let mut escaping = 0usize;
    for (prefix, (next, count)) in &prefixes {
        let costs = vec![STD_LOG_MOVE_COST; prefix.len()];
        let sp = SyncProduct::new(&model, prefix, &costs, MoveSet::SyncAndSilent);
        let ends = stop_markings(&sp);
        if ends.is_empty() {
            continue;
        }
        let mut activated: BTreeSet<String> = BTreeSet::new();
        for end in ends {
            let m: Marking = end
                .iter()
                .filter(|&&e| place(e) < model.place_count())
                .map(|&e| (dense_to_place[place(e) as usize], tokens(e)))
                .collect();
            activated.extend(eventually_enabled(m)?);
        }
        let esc = activated
            .iter()
            .filter(|a| !next.contains(a.as_str()))
            .count();
        allowed += activated.len() * count;
        escaping += esc * count;
    }

    let initial: Marking = initial_marking
        .iter()
        .filter(|&(p, _)| model.dense_place(p).is_some())
        .collect();
    let at_start = eventually_enabled(initial)?;
    let diff = at_start
        .iter()
        .filter(|a| !start_activities.contains(a.as_str()))
        .count();
    let traces = log.traces.len();
    allowed += traces * at_start.len();
    escaping += traces * diff;

    Ok(if allowed > 0 {
        1.0 - escaping as f64 / allowed as f64
    } else {
        1.0
    })
}
