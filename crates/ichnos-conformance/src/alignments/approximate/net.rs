//! The model as the approximate searches see it: transitions over dense
//! places, in pm4py's order of enabled transitions.

use std::collections::HashSet;

use ichnos_model::petri::ArcEnds;
use ichnos_model::{Marking, PetriNet, TransitionId};

use super::super::costs::{
    ModelCosts, STD_LOG_MOVE_COST, STD_MODEL_MOVE_COST, STD_SILENT_MOVE_COST,
};
use super::super::marking::{Packed, apply_delta, pack, place, tokens};
use crate::error::{Error, Result};

/// One transition of the model.
#[derive(Debug, Clone)]
pub(super) struct NetTransition {
    pub(super) id: TransitionId,
    pub(super) label: Option<Box<str>>,
    pre: Vec<(u32, u32)>,
    post: Vec<(u32, u32)>,
    delta: Vec<(u32, i64)>,
    pub(super) model_cost: u64,
    pub(super) sync_cost: u64,
}

/// The model of an approximate alignment.
///
/// `transitions` is sorted the way pm4py sorts enabled transitions: by the
/// label as Python prints it (`None` for a silent transition), then by name.
#[derive(Debug, Clone)]
pub(super) struct Net {
    pub(super) transitions: Vec<NetTransition>,
    /// For each place, the transitions that consume from it.
    consumers: Vec<Vec<u32>>,
    /// The position in `transitions` of each transition id.
    by_id: Vec<u32>,
    pub(super) initial: Vec<Packed>,
    pub(super) final_marking: Vec<Packed>,
}

/// One step of an approximate alignment (pm4py's `AlignmentStep`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Step<'a> {
    /// The event's activity, unless this is a model move.
    pub(super) log: Option<&'a str>,
    /// The index of the event in the searched trace. Tandem-repeat
    /// expansion inserts events without an index.
    pub(super) log_index: Option<usize>,
    /// The transition (an index into [`Net::transitions`]), unless this is
    /// a log move.
    pub(super) transition: Option<u32>,
    pub(super) cost: u64,
}

impl Net {
    pub(super) fn new(
        net: &PetriNet,
        initial: &Marking,
        final_marking: &Marking,
        costs: &ModelCosts,
    ) -> Result<Self> {
        if net.has_special_arcs() {
            return Err(Error::SpecialArcs);
        }
        let mut dense = vec![u32::MAX; net.place_index_bound()];
        for (i, p) in net.place_ids().enumerate() {
            dense[p.index()] = u32::try_from(i).expect("place count fits u32");
        }
        let mut transitions: Vec<(String, &str, NetTransition)> = net
            .transitions()
            .map(|(id, tr)| {
                let mut pre = Vec::new();
                let mut post = Vec::new();
                for &a in tr.in_arcs().iter().chain(tr.out_arcs()) {
                    let arc = net.arc(a);
                    match arc.ends {
                        ArcEnds::PlaceToTransition(p, _) => {
                            pre.push((dense[p.index()], arc.weight))
                        }
                        ArcEnds::TransitionToPlace(_, p) => {
                            post.push((dense[p.index()], arc.weight));
                        }
                    }
                }
                let pre = merge(pre);
                let post = merge(post);
                let delta = delta(&pre, &post);
                let label = tr.label.as_ref().map(|l| Box::from(l.as_str()));
                let key = tr
                    .label
                    .as_ref()
                    .map_or("None".to_owned(), |l| l.as_str().to_owned());
                let t = NetTransition {
                    id,
                    label,
                    pre,
                    post,
                    delta,
                    model_cost: costs.model_move(id),
                    sync_cost: costs.sync_move(id),
                };
                (key, tr.name.as_str(), t)
            })
            .collect();
        // A stable sort keeps id order between equal keys, where pm4py
        // falls back to object ids.
        transitions.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
        let transitions: Vec<NetTransition> = transitions.into_iter().map(|t| t.2).collect();
        let mut by_id = vec![u32::MAX; net.transition_index_bound()];
        for (i, t) in transitions.iter().enumerate() {
            by_id[t.id.index()] = u32::try_from(i).expect("transition count fits u32");
        }
        let mut consumers = vec![Vec::new(); net.place_count()];
        for (i, t) in transitions.iter().enumerate() {
            for &(p, _) in &t.pre {
                consumers[p as usize].push(u32::try_from(i).expect("transition count fits u32"));
            }
        }
        let encode = |m: &Marking| -> Result<Vec<Packed>> {
            let mut out = Vec::new();
            for (p, n) in m.iter() {
                let d = *dense.get(p.index()).ok_or(Error::UnknownPlace(p))?;
                if d == u32::MAX {
                    return Err(Error::UnknownPlace(p));
                }
                if n > 0 {
                    out.push(pack(d, n));
                }
            }
            out.sort_unstable();
            Ok(out)
        };
        Ok(Self {
            transitions,
            consumers,
            by_id,
            initial: encode(initial)?,
            final_marking: encode(final_marking)?,
        })
    }

    /// The position in [`Net::transitions`] of transition `id`.
    pub(super) fn index_of(&self, id: TransitionId) -> u32 {
        self.by_id[id.index()]
    }

    pub(super) fn transition_count(&self) -> usize {
        self.transitions.len()
    }

    /// Whether transition `t` is enabled in `m`.
    pub(super) fn is_enabled(&self, t: u32, m: &[Packed]) -> bool {
        self.transitions[t as usize]
            .pre
            .iter()
            .all(|&(p, w)| tokens_at(m, p) >= w)
    }

    /// The enabled transitions in pm4py's order.
    pub(super) fn enabled(&self, m: &[Packed], out: &mut Vec<u32>) {
        out.clear();
        out.extend((0..self.transitions.len() as u32).filter(|&t| self.is_enabled(t, m)));
    }

    /// Fires an enabled transition.
    pub(super) fn fire(&self, t: u32, m: &[Packed]) -> Vec<Packed> {
        let mut out = Vec::with_capacity(m.len() + 1);
        apply_delta(m, &self.transitions[t as usize].delta, &mut out);
        out
    }

    /// pm4py's `weak_execute`: tokens are taken without checking that they
    /// exist, and a place never goes below zero.
    pub(super) fn weak_fire(&self, t: u32, m: &mut [i64]) {
        let tr = &self.transitions[t as usize];
        for &(p, w) in &tr.pre {
            let v = &mut m[p as usize];
            *v = (*v - i64::from(w)).max(0);
        }
        for &(p, w) in &tr.post {
            m[p as usize] += i64::from(w);
        }
    }

    /// A marking as one count per place.
    pub(super) fn dense(&self, m: &[Packed]) -> Vec<i64> {
        let mut out = vec![0; self.consumers.len()];
        for &e in m {
            out[place(e) as usize] = i64::from(tokens(e));
        }
        out
    }

    /// The label of transition `t`.
    pub(super) fn label(&self, t: u32) -> Option<&str> {
        self.transitions[t as usize].label.as_deref()
    }

    /// pm4py's `structurally_reachable_labels`: the visible labels of the
    /// transitions without input arcs and of the transitions reached
    /// forward from the marked places. Like pm4py, it does not follow the
    /// output places of transitions without input arcs.
    pub(super) fn reachable_labels(&self, m: &[Packed]) -> HashSet<&str> {
        let mut places = vec![false; self.consumers.len()];
        let mut reached: Vec<bool> = self.transitions.iter().map(|t| t.pre.is_empty()).collect();
        let mut frontier: Vec<u32> = m.iter().map(|&e| place(e)).collect();
        for &p in &frontier {
            places[p as usize] = true;
        }
        while let Some(p) = frontier.pop() {
            for &t in &self.consumers[p as usize] {
                if reached[t as usize] {
                    continue;
                }
                reached[t as usize] = true;
                for &(q, _) in &self.transitions[t as usize].post {
                    if !places[q as usize] {
                        places[q as usize] = true;
                        frontier.push(q);
                    }
                }
            }
        }
        self.transitions
            .iter()
            .zip(reached)
            .filter(|(_, r)| *r)
            .filter_map(|(t, _)| t.label.as_deref())
            .collect()
    }

    /// pm4py's `validate_steps`: the steps replay the trace, fire only
    /// enabled transitions, pair events with transitions of the same label
    /// and end in the final marking.
    pub(super) fn validate<S: AsRef<str>>(&self, labels: &[S], steps: &[Step<'_>]) -> bool {
        let mut projected = Vec::new();
        let mut m = self.initial.clone();
        for s in steps {
            if let Some(l) = s.log {
                projected.push(l);
            }
            if let Some(t) = s.transition {
                if !self.is_enabled(t, &m) {
                    return false;
                }
                m = self.fire(t, &m);
                if let Some(l) = s.log
                    && self.label(t) != Some(l)
                {
                    return false;
                }
            }
        }
        projected.len() == labels.len()
            && projected.iter().zip(labels).all(|(a, b)| *a == b.as_ref())
            && m == self.final_marking
    }

    /// pm4py's `standard_cost` of the steps.
    pub(super) fn standard_cost(&self, steps: &[Step<'_>]) -> u64 {
        steps
            .iter()
            .map(|s| match (s.log, s.transition) {
                (Some(_), None) => STD_LOG_MOVE_COST,
                (None, Some(t)) if self.label(t).is_some() => STD_MODEL_MOVE_COST,
                (None, Some(_)) => STD_SILENT_MOVE_COST,
                _ => 0,
            })
            .sum()
    }
}

/// The tokens on place `p`.
fn tokens_at(m: &[Packed], p: u32) -> u32 {
    m.binary_search_by_key(&p, |&e| place(e))
        .map_or(0, |i| tokens(m[i]))
}

fn merge(mut v: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    v.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::with_capacity(v.len());
    for (p, w) in v {
        match out.last_mut() {
            Some(last) if last.0 == p => last.1 += w,
            _ => out.push((p, w)),
        }
    }
    out
}

/// The token change of a transition, sorted by place, without zeros.
fn delta(pre: &[(u32, u32)], post: &[(u32, u32)]) -> Vec<(u32, i64)> {
    let mut d: Vec<(u32, i64)> = pre
        .iter()
        .map(|&(p, w)| (p, -i64::from(w)))
        .chain(post.iter().map(|&(p, w)| (p, i64::from(w))))
        .collect();
    d.sort_unstable_by_key(|e| e.0);
    let mut out: Vec<(u32, i64)> = Vec::with_capacity(d.len());
    for (p, v) in d {
        match out.last_mut() {
            Some(last) if last.0 == p => last.1 += v,
            _ => out.push((p, v)),
        }
    }
    out.retain(|e| e.1 != 0);
    out
}
