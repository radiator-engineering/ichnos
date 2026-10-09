//! Marking equations, ported from pm4py's
//! `algo/analysis/marking_equation` and
//! `algo/analysis/extended_marking_equation` (classic variants).

use super::AnalysisError;
use super::lp::LinearProgram;
use super::sync_product::SynchronousProduct;
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, TransitionId};

/// Margin for reading an integer off a floating-point optimum.
const EPS: f64 = 1e-6;

/// pm4py's incidence matrix: each arc counts once, whatever its weight or
/// kind, so an inhibitor or reset arc counts as a consuming arc. Places and
/// transitions are in id order.
struct Incidence {
    rows: Vec<Vec<f64>>,
    transitions: Vec<TransitionId>,
    place_index: Vec<Option<usize>>,
}

impl Incidence {
    fn new(net: &PetriNet) -> Self {
        let transitions: Vec<TransitionId> = net.transition_ids().collect();
        let mut t_index = vec![0; net.transition_index_bound()];
        for (j, t) in transitions.iter().enumerate() {
            t_index[t.index()] = j;
        }
        let mut place_index = vec![None; net.place_index_bound()];
        let mut rows = Vec::new();
        for (i, (p, place)) in net.places().enumerate() {
            place_index[p.index()] = Some(i);
            let mut row = vec![0.0; transitions.len()];
            for &a in place.in_arcs() {
                row[t_index[net.arc(a).transition().index()]] += 1.0;
            }
            for &a in place.out_arcs() {
                row[t_index[net.arc(a).transition().index()]] -= 1.0;
            }
            rows.push(row);
        }
        Self {
            rows,
            transitions,
            place_index,
        }
    }

    fn encode(&self, m: &Marking) -> Vec<f64> {
        let mut v = vec![0.0; self.rows.len()];
        for (p, n) in m.iter() {
            if let Some(i) = self.place_index[p.index()] {
                v[i] = f64::from(n);
            }
        }
        v
    }
}

/// pm4py's `pick_chosen_points_list` with the extremes included.
fn pick_points(m: usize, list: &[usize]) -> Vec<usize> {
    let n = list.len();
    let mut points: Vec<usize> = (0..m).map(|i| i * n / m + n / (2 * m)).collect();
    if !points.contains(&0) {
        points.insert(0, 0);
    }
    if !points.contains(&(n - 1)) {
        points.push(n - 1);
    }
    points.into_iter().map(|i| list[i]).collect()
}

impl AcceptingPetriNet {
    /// The marking-equation estimate of the cost to reach the final
    /// marking (pm4py's `solve_marking_equation`): the minimum of
    /// `Σ cost(t)·x(t)` over real `x >= 0` with `A·x = final - initial`,
    /// where `A` is the incidence matrix. `None` when the equation has no
    /// solution.
    ///
    /// pm4py calls this with a cost of 1 per transition by default. It
    /// truncates the optimum to an integer; here the optimum is rounded
    /// down after adding 1e-6, so solver noise just under an integer does
    /// not lose 1. As in pm4py, the incidence matrix counts each arc once,
    /// whatever its weight or kind.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::LinearProgram`] when the solver fails.
    pub fn solve_marking_equation(
        &self,
        cost: impl Fn(TransitionId) -> f64,
    ) -> Result<Option<i64>, AnalysisError> {
        let a = Incidence::new(&self.net);
        let ini = a.encode(&self.initial_marking);
        let fin = a.encode(&self.final_marking);
        let mut lp = LinearProgram::new(a.transitions.iter().map(|&t| cost(t)).collect());
        for (p, row) in a.rows.iter().enumerate() {
            lp.a_eq.push(row.clone());
            lp.b_eq.push(fin[p] - ini[p]);
        }
        Ok(lp.solve()?.map(|x| {
            let h: f64 = x.iter().zip(&lp.c).map(|(x, c)| x * c).sum();
            (h + EPS).floor() as i64
        }))
    }
}

impl SynchronousProduct {
    /// The extended marking-equation estimate of the alignment cost
    /// (pm4py's `solve_extended_marking_equation`, after van Dongen,
    /// "Efficiently computing alignments", BPM 2018), with pm4py's
    /// standard costs ([`SynchronousProduct::standard_cost`]). `None` when
    /// the integer program has no solution.
    ///
    /// The trace is split before each event index in `split_points`. The
    /// default is every index from 1 to the trace length minus 1. With more
    /// than 5 split points, pm4py keeps 5 spread evenly plus the first and
    /// last; so does this.
    ///
    /// The integer program is solved with `microlp`; pm4py uses CVXOPT
    /// with GLPK. pm4py truncates its floating-point optimum, so it can
    /// return 1 less than the optimum when GLPK lands just below an
    /// integer; this rounds as [`AcceptingPetriNet::solve_marking_equation`]
    /// does. As there, each arc counts once.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::LinearProgram`] when the solver fails.
    pub fn solve_extended_marking_equation(
        &self,
        split_points: Option<&[usize]>,
    ) -> Result<Option<i64>, AnalysisError> {
        const MAX_K: usize = 5;
        let default: Vec<usize> = (1..self.trace.len()).collect();
        let mut split = split_points.map_or(default, <[usize]>::to_vec);
        if split.len() > MAX_K {
            split = pick_points(MAX_K, &split);
        }
        let k = if split.len() > 1 { split.len() } else { 2 };

        let net = &self.net.net;
        let inc = Incidence::new(net);
        let ts = &inc.transitions;
        let nt = ts.len();
        let np = inc.rows.len();
        let ini = inc.encode(&self.net.initial_marking);
        let fin = inc.encode(&self.net.final_marking);
        let a = |p: usize, t: usize| inc.rows[p][t];
        let consumes: Vec<Vec<f64>> = net
            .place_ids()
            .map(|p| {
                let post: Vec<TransitionId> = net.place_postset(p).collect();
                ts.iter()
                    .map(|t| if post.contains(t) { -1.0 } else { 0.0 })
                    .collect()
            })
            .collect();

        // Variables: x_0..x_{k-1}, then y_0..y_{k-2}, then xy_0..xy_{k-2},
        // each one block of `nt`.
        let x = |i: usize, j: usize| i * nt + j;
        let y = |i: usize, j: usize| (k + i) * nt + j;
        let xy = |i: usize, j: usize| (2 * k - 1 + i) * nt + j;
        let width = (3 * k - 2) * nt;
        let costs: Vec<f64> = ts.iter().map(|&t| self.standard_cost(t) as f64).collect();
        let mut c = vec![0.0; width];
        for j in 0..nt {
            for i in 0..k {
                c[x(i, j)] = costs[j];
            }
            for i in 0..k - 1 {
                c[y(i, j)] = costs[j];
            }
        }
        let mut lp = LinearProgram::new(c);
        lp.integer = true;
        let empty = || vec![0.0; width];

        for p in 0..np {
            let mut row = empty();
            for j in 0..nt {
                row[x(0, j)] = a(p, j);
                for i in 0..k - 1 {
                    row[xy(i, j)] = a(p, j);
                }
            }
            lp.a_eq.push(row);
            lp.b_eq.push(fin[p] - ini[p]);
        }
        for i in 1..k {
            for j in 0..nt {
                let mut row = empty();
                row[x(i, j)] = 1.0;
                row[y(i - 1, j)] = 1.0;
                row[xy(i - 1, j)] = -1.0;
                lp.a_eq.push(row);
                lp.b_eq.push(0.0);
            }
        }
        for i in 0..k - 1 {
            let mut row = empty();
            for j in 0..nt {
                row[y(i, j)] = 1.0;
            }
            lp.a_eq.push(row);
            lp.b_eq.push(1.0);
        }
        // y_i may only fire a transition that consumes the event at the
        // i-th split point.
        for (i, &at) in split.iter().enumerate().take(k - 1) {
            let activity = self.trace.get(at);
            let mut row = empty();
            for (j, &t) in ts.iter().enumerate() {
                let consumes_event = match self.move_of(t) {
                    super::SyncMove::Log(e) | super::SyncMove::Sync(e, _) => {
                        Some(&self.trace[e]) == activity
                    }
                    super::SyncMove::Model(_) => false,
                };
                if !consumes_event {
                    row[y(i, j)] = 1.0;
                }
            }
            lp.a_eq.push(row);
            lp.b_eq.push(0.0);
        }
        // Before each split, the marking reached must let y_i fire:
        // A·x_0 + Σ_{j<i} A·xy_j + C·y_i >= -initial.
        for i in 0..k - 1 {
            for p in 0..np {
                let mut row = empty();
                for j in 0..nt {
                    row[x(0, j)] = -a(p, j);
                    row[y(i, j)] = -consumes[p][j];
                    for l in 0..i {
                        row[xy(l, j)] = -a(p, j);
                    }
                }
                lp.a_ub.push(row);
                lp.b_ub.push(ini[p]);
            }
        }
        Ok(lp.solve()?.map(|v| {
            let h: f64 = v.iter().zip(&lp.c).map(|(v, c)| v * c).sum();
            (h + EPS).floor() as i64
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::pick_points;

    #[test]
    fn picks_points_like_pm4py() {
        let list: Vec<usize> = (1..10).collect();
        // pm4py: [0, 1, 3, 5, 7] plus the last index 8.
        assert_eq!(pick_points(5, &list), vec![1, 2, 4, 6, 8, 9]);
    }
}
