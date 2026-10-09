//! Firing rules, ported from pm4py's `petri_net/semantics.py` and
//! `petri_net/inhibitor_reset/semantics.py`.
//!
//! One rule set covers all nets. Normal arcs need and consume `weight` tokens.
//! Inhibitor arcs need an empty place. Reset arcs empty the place on firing.
//! For nets with only normal arcs this is pm4py's `ClassicSemantics`.

use std::collections::{BTreeSet, HashSet, VecDeque};

use super::{
    ArcEnds, ArcKind, Marking, PetriNet, ReachabilityError, ReachabilityOptions, TransitionId,
};

/// Error returned by [`PetriNet::fire`] when the transition is not enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("transition {0:?} is not enabled in the given marking")]
pub struct NotEnabled(pub TransitionId);

impl PetriNet {
    /// Returns `true` if `t` may fire in marking `m`.
    pub fn is_enabled(&self, t: TransitionId, m: &Marking) -> bool {
        self.transition(t).in_arcs().iter().all(|&a| {
            let arc = self.arc(a);
            let tokens = m.get(arc.place());
            match arc.kind {
                ArcKind::Normal => tokens >= arc.weight,
                ArcKind::Inhibitor => tokens == 0,
                ArcKind::Reset => true,
            }
        })
    }

    /// Fires `t` in `m` and returns the new marking, or [`NotEnabled`].
    ///
    /// Matches pm4py's `semantics.execute`.
    pub fn fire(&self, t: TransitionId, m: &Marking) -> Result<Marking, NotEnabled> {
        if self.is_enabled(t, m) {
            Ok(self.weak_fire(t, m))
        } else {
            Err(NotEnabled(t))
        }
    }

    /// Fires `t` in `m` without checking that it is enabled. Places that would
    /// go below zero stay at zero.
    ///
    /// Matches pm4py's `semantics.weak_execute`.
    pub fn weak_fire(&self, t: TransitionId, m: &Marking) -> Marking {
        let mut out = m.clone();
        let tr = self.transition(t);
        for &a in tr.in_arcs() {
            let arc = self.arc(a);
            match arc.kind {
                ArcKind::Normal => out.remove(arc.place(), arc.weight),
                ArcKind::Inhibitor => {}
                ArcKind::Reset => out.set(arc.place(), 0),
            }
        }
        for &a in tr.out_arcs() {
            let arc = self.arc(a);
            if let ArcEnds::TransitionToPlace(_, p) = arc.ends {
                out.add(p, arc.weight);
            }
        }
        out
    }

    /// All transitions enabled in `m`, in id order.
    pub fn enabled_transitions(&self, m: &Marking) -> Vec<TransitionId> {
        self.transition_ids()
            .filter(|&t| self.is_enabled(t, m))
            .collect()
    }

    /// Visible transitions that become enabled from `m` after firing zero or
    /// more silent transitions.
    ///
    /// pm4py's `align_utils.get_visible_transitions_eventually_enabled_by_marking`
    /// computes the same set but can miss markings when one silent transition
    /// is reached along two paths. This version explores every marking
    /// reachable through silent transitions.
    ///
    /// Fails with [`ReachabilityError::TooManyMarkings`] once more than
    /// `options.max_markings` markings are reachable through silent
    /// transitions alone, so it ends even when that state space is infinite.
    pub fn visible_transitions_eventually_enabled(
        &self,
        m: &Marking,
        options: ReachabilityOptions,
    ) -> Result<BTreeSet<TransitionId>, ReachabilityError> {
        let mut visible = BTreeSet::new();
        let mut seen: HashSet<Marking> = HashSet::new();
        let mut queue = VecDeque::from([m.clone()]);
        seen.insert(m.clone());
        while let Some(cur) = queue.pop_front() {
            for t in self.enabled_transitions(&cur) {
                if self.transition(t).label.is_some() {
                    visible.insert(t);
                } else {
                    let next = self.weak_fire(t, &cur);
                    if seen.insert(next.clone()) {
                        if seen.len() > options.max_markings {
                            return Err(ReachabilityError::TooManyMarkings(options.max_markings));
                        }
                        queue.push_back(next);
                    }
                }
            }
        }
        Ok(visible)
    }
}
