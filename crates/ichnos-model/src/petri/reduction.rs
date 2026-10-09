//! Structural reductions, ported from pm4py's `petri_net/utils/reduction.py`.

use super::{ArcEnds, ArcId, ArcKind, PetriNet, TransitionId};

impl PetriNet {
    /// Merges silent transitions into their neighbours where this keeps the
    /// language (pm4py's `reduction.apply_simple_reduction`).
    ///
    /// A silent transition `t` with one input place `p` is removed with `p`
    /// when `p` has one producer `u` and feeds only `t`; `u` then produces
    /// into `t`'s output places. The mirror rule removes a silent transition
    /// with one output place that has one consumer and is fed only by it.
    ///
    /// Both arcs on the removed place must be normal arcs of weight 1, and
    /// every other arc of the removed transition must be a normal arc. Its
    /// weights carry over to the merged arcs. pm4py checks neither, so on a
    /// net with inhibitor, reset or weighted arcs it can change behaviour.
    /// A silent self-loop (`u == t`) is never merged; pm4py would remove `t`
    /// and then add an arc from it.
    ///
    /// Candidates are tried in transition-id order. pm4py iterates a hash
    /// set, so on some nets it can pick a different, equally valid
    /// reduction.
    pub fn apply_simple_reduction(&mut self) {
        loop {
            let before = (self.transition_count(), self.place_count());
            while self.reduce_single_entry_once() {}
            while self.reduce_single_exit_once() {}
            if (self.transition_count(), self.place_count()) == before {
                return;
            }
        }
    }

    fn is_plain(&self, a: ArcId) -> bool {
        let arc = self.arc(a);
        arc.kind == ArcKind::Normal && arc.weight == 1
    }

    fn all_normal(&self, arcs: &[ArcId]) -> bool {
        arcs.iter().all(|&a| self.arc(a).kind == ArcKind::Normal)
    }

    fn silent_transitions(&self) -> Vec<TransitionId> {
        self.transitions()
            .filter(|(_, t)| t.label.is_none())
            .map(|(id, _)| id)
            .collect()
    }

    fn reduce_single_entry_once(&mut self) -> bool {
        for t in self.silent_transitions() {
            let tr = self.transition(t);
            if tr.in_arcs().len() != 1 {
                continue;
            }
            let p = self.arc(tr.in_arcs()[0]).place();
            let place = self.place(p);
            if place.in_arcs().len() != 1 || !self.place_postset(p).all(|x| x == t) {
                continue;
            }
            let u = self.arc(place.in_arcs()[0]).transition();
            if u == t
                || !self.is_plain(tr.in_arcs()[0])
                || !self.is_plain(place.in_arcs()[0])
                || !self.all_normal(tr.out_arcs())
            {
                continue;
            }
            let targets: Vec<_> = tr
                .out_arcs()
                .iter()
                .map(|&a| (self.arc(a).place(), self.arc(a).weight))
                .collect();
            self.remove_transition(t);
            self.remove_place(p);
            for (q, w) in targets {
                self.add_arc(ArcEnds::TransitionToPlace(u, q), w, ArcKind::Normal)
                    .expect("both ends are live elements of this net");
            }
            return true;
        }
        false
    }

    fn reduce_single_exit_once(&mut self) -> bool {
        for t in self.silent_transitions() {
            let tr = self.transition(t);
            if tr.out_arcs().len() != 1 {
                continue;
            }
            let p = self.arc(tr.out_arcs()[0]).place();
            let place = self.place(p);
            if place.out_arcs().len() != 1 || !self.place_preset(p).all(|x| x == t) {
                continue;
            }
            let u = self.arc(place.out_arcs()[0]).transition();
            if u == t
                || !self.is_plain(tr.out_arcs()[0])
                || !self.is_plain(place.out_arcs()[0])
                || !self.all_normal(tr.in_arcs())
            {
                continue;
            }
            let sources: Vec<_> = tr
                .in_arcs()
                .iter()
                .map(|&a| (self.arc(a).place(), self.arc(a).weight))
                .collect();
            self.remove_transition(t);
            self.remove_place(p);
            for (q, w) in sources {
                self.add_arc(ArcEnds::PlaceToTransition(q, u), w, ArcKind::Normal)
                    .expect("both ends are live elements of this net");
            }
            return true;
        }
        false
    }
}
