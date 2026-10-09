//! Structural reductions, ported from pm4py's `petri_net/utils/reduction.py`.

use super::{PetriNet, TransitionId};

impl PetriNet {
    /// Merges silent transitions into their neighbours where this keeps the
    /// language (pm4py's `reduction.apply_simple_reduction`).
    ///
    /// A silent transition `t` with one input place `p` is removed with `p`
    /// when `p` has one producer `u` and feeds only `t`; `u` then produces
    /// into `t`'s output places. The mirror rule removes a silent transition
    /// with one output place that has one consumer and is fed only by it.
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
            if u == t {
                continue;
            }
            let targets: Vec<_> = self.postset(t).collect();
            self.remove_transition(t);
            self.remove_place(p);
            for q in targets {
                self.add_output_arc(u, q)
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
            if u == t {
                continue;
            }
            let sources: Vec<_> = self.preset(t).collect();
            self.remove_transition(t);
            self.remove_place(p);
            for q in sources {
                self.add_input_arc(q, u)
                    .expect("both ends are live elements of this net");
            }
            return true;
        }
        false
    }
}
