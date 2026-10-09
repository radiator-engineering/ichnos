//! Footprints of a Petri net as pm4py finds them (pm4py's
//! `algo/discovery/footprints/petri/variants/reach_graph`).

use std::collections::{BTreeSet, HashSet};

use ichnos_model::footprints::LabelPair;
use ichnos_model::petri::{ReachabilityError, ReachabilityOptions};
use ichnos_model::{Footprints, Label, Marking, PetriNet, TransitionId};
use rustc_hash::FxHashMap;

use crate::error::{Error, Result};
use crate::token_replay::net::{ReplayNet, Tokens};

/// The footprints of the accepting net, from every marking reachable from
/// `initial` (pm4py's `marking_flow_petri`, then its `reach_graph` rules).
///
/// What can follow a visible transition comes from pm4py's
/// `get_visible_transitions_eventually_enabled_by_marking`, quirk included
/// (see [`ReplayNet::eventually_enabled`]). Arcs of every kind count as
/// normal arcs, as in pm4py's classic semantics.
pub(crate) fn net_footprints(net: &PetriNet, initial: &Marking) -> Result<Footprints> {
    if let Some((p, _)) = initial.iter().find(|&(p, _)| !net.contains_place(p)) {
        return Err(Error::UnknownPlace(p));
    }
    let rn = ReplayNet::structure(net);
    let limit = ReachabilityOptions::default().max_markings;

    // The reachable markings, with the transitions into and out of each.
    let start = rn.tokens(initial);
    let mut markings: Vec<Tokens> = vec![start.clone()];
    let mut index: FxHashMap<Tokens, usize> = FxHashMap::default();
    index.insert(start, 0);
    let mut incoming: Vec<BTreeSet<TransitionId>> = vec![BTreeSet::new()];
    let mut outgoing: Vec<BTreeSet<TransitionId>> = Vec::new();
    let mut i = 0;
    while i < markings.len() {
        let m = markings[i].clone();
        let enabled: BTreeSet<TransitionId> = rn.enabled_by_name(&m).collect();
        for &t in &enabled {
            let mut next = m.clone();
            rn.fire(t, &mut next);
            let j = match index.get(&next) {
                Some(&j) => j,
                None => {
                    if markings.len() >= limit {
                        return Err(ReachabilityError::TooManyMarkings(limit).into());
                    }
                    index.insert(next.clone(), markings.len());
                    markings.push(next);
                    incoming.push(BTreeSet::new());
                    markings.len() - 1
                }
            };
            incoming[j].insert(t);
        }
        outgoing.push(enabled);
        i += 1;
    }

    let visible = |t: &TransitionId| !rn.silent[t.index()];
    let label = |t: TransitionId| {
        net.transition(t)
            .label
            .clone()
            .expect("only visible transitions are mapped to labels")
    };
    let mut sequence: HashSet<(TransitionId, TransitionId)> = HashSet::new();
    let mut s1: HashSet<(TransitionId, TransitionId)> = HashSet::new();
    let mut s2: HashSet<(TransitionId, TransitionId)> = HashSet::new();
    let mut start_activities = BTreeSet::new();
    for (i, m) in markings.iter().enumerate() {
        let input: Vec<TransitionId> = incoming[i].iter().copied().filter(visible).collect();
        let output: Vec<TransitionId> = outgoing[i].iter().copied().filter(visible).collect();
        let eventually = rn.eventually_enabled(m)?;
        if i == 0 {
            start_activities = eventually.iter().map(|&t| label(t)).collect();
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
    let activities: BTreeSet<Label> = net
        .transitions()
        .filter_map(|(_, t)| t.label.clone())
        .collect();
    Ok(Footprints {
        activities,
        start_activities,
        sequence: seq,
        parallel,
    })
}
