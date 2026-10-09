//! Implicit-place removal, ported from pm4py's
//! `objects/petri_net/utils/murata.py` (`apply_reduction`).

use std::collections::{BTreeSet, HashMap};

use super::AnalysisError;
use super::lp::LinearProgram;
use crate::petri::{AcceptingPetriNet, PlaceId};

impl AcceptingPetriNet {
    /// Removes implicit places (pm4py's `reduce_petri_net_implicit_places`)
    /// and returns them, sorted by name.
    ///
    /// Places are tried in name order, skipping marked ones. A place `p` is
    /// implicit when the integer program of Berthelot's structural test is
    /// feasible: weights `a_q >= 0` for the other remaining places, `a_p >=
    /// 1`, and `k >= 0` with
    ///
    /// - `a_p·m0(p) - Σ a_q·m0(q) = k` for the initial marking,
    /// - `a_p·C(p, t) - Σ a_q·C(q, t) = 0` for each transition `t`,
    /// - `a_p·W(p, t) - Σ a_q·W(q, t) <= k` for each transition `t`,
    ///
    /// where `C` is the incidence matrix and `W` the input weights, both over
    /// the places not yet found implicit. All implicit places are removed
    /// at the end; the markings do not change.
    ///
    /// The programs are solved with `microlp`; pm4py uses scipy or PuLP.
    /// Feasibility does not depend on the solver.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::LinearProgram`] when the solver fails.
    pub fn reduce_implicit_places(&mut self) -> Result<Vec<PlaceId>, AnalysisError> {
        let net = &self.net;
        let mut places: Vec<PlaceId> = net.place_ids().collect();
        places.sort_by(|&a, &b| net.place(a).name.cmp(&net.place(b).name));
        let mut redundant: BTreeSet<PlaceId> = BTreeSet::new();
        let mut found = Vec::new();
        for &place in &places {
            if self.initial_marking.get(place) > 0 || self.final_marking.get(place) > 0 {
                continue;
            }
            let active: Vec<PlaceId> = places
                .iter()
                .copied()
                .filter(|p| !redundant.contains(p))
                .collect();
            let index: HashMap<PlaceId, usize> =
                active.iter().enumerate().map(|(i, &p)| (p, i)).collect();
            let k = active.len();
            let sign = |p: PlaceId| if p == place { 1.0 } else { -1.0 };
            let mut lp = LinearProgram::new(vec![1.0; k + 1]);
            lp.integer = true;

            let mut eq = vec![0.0; k + 1];
            for &p in &active {
                eq[index[&p]] += sign(p) * f64::from(self.initial_marking.get(p));
            }
            eq[k] = -1.0;
            lp.a_eq.push(eq);
            lp.b_eq.push(0.0);
            for t in net.transition_ids() {
                let mut change = vec![0.0; k + 1];
                let mut consume = vec![0.0; k + 1];
                for &a in net.transition(t).in_arcs() {
                    let arc = net.arc(a);
                    if let Some(&i) = index.get(&arc.place()) {
                        let w = f64::from(arc.weight);
                        change[i] -= sign(arc.place()) * w;
                        consume[i] += sign(arc.place()) * w;
                    }
                }
                for &a in net.transition(t).out_arcs() {
                    let arc = net.arc(a);
                    if let Some(&i) = index.get(&arc.place()) {
                        change[i] += sign(arc.place()) * f64::from(arc.weight);
                    }
                }
                lp.a_eq.push(change);
                lp.b_eq.push(0.0);
                consume[k] = -1.0;
                lp.a_ub.push(consume);
                lp.b_ub.push(0.0);
            }
            lp.a_ub.push({
                let mut row = vec![0.0; k + 1];
                row[index[&place]] = -1.0;
                row
            });
            lp.b_ub.push(-1.0);
            if lp.solve()?.is_some() {
                redundant.insert(place);
                found.push(place);
            }
        }
        for &p in &found {
            self.net.remove_place(p);
        }
        Ok(found)
    }
}
