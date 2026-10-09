//! The marking-equation heuristic of A* alignments.
//!
//! From a marking `m`, every firing sequence that reaches the final marking
//! `f` has a Parikh vector `x >= 0` with `C x = f - m`, where `C` is the
//! incidence matrix of the synchronous product. The cheapest real solution,
//! `min c·x`, is a lower bound on the remaining cost. pm4py solves the same
//! linear program with cvxopt or scipy.
//!
//! Within one search only the right-hand side changes, so ichnos solves it
//! with its own warm-started dual simplex ([`DualSimplex`]). If that solver
//! stalls, the program is solved from scratch with `good_lp` and the
//! pure-Rust `microlp` solver.
//!
//! Costs are integers, so the remaining cost is at least the LP optimum
//! rounded up. The heuristic returns that integer.

use good_lp::{
    Expression, ProblemVariables, ResolutionError, Solution, SolverModel, Variable, constraint,
    microlp, variable,
};

use super::marking::{Packed, place, tokens};
use super::simplex::{DualSimplex, LpFailure};
use super::sync_product::SyncProduct;
use crate::error::{Error, Result};

/// Slack for floating-point noise in LP optima and solution vectors.
const EPS: f64 = 1e-6;

/// The marking equation of one synchronous product.
#[derive(Debug, Clone)]
pub(crate) struct StateEquation {
    /// One row per place: `(move, coefficient)` pairs.
    rows: Vec<Vec<(u32, f64)>>,
    final_marking: Vec<i64>,
    costs: Vec<f64>,
    /// Places with a non-empty row, in the order of the simplex rows.
    used: Vec<usize>,
    simplex: DualSimplex,
}

/// A heuristic value and the LP solution it came from.
#[derive(Debug, Clone)]
pub(crate) struct Estimate {
    pub(crate) h: u64,
    /// The LP optimum before rounding.
    pub(crate) objective: f64,
    pub(crate) x: Vec<f64>,
}

impl StateEquation {
    pub(crate) fn new(sp: &SyncProduct) -> Self {
        let mut rows = vec![Vec::new(); sp.place_count];
        for t in 0..sp.len() {
            for &(p, d) in sp.delta(t) {
                rows[p as usize].push((u32::try_from(t).expect("moves fit u32"), d as f64));
            }
        }
        let mut final_marking = vec![0i64; sp.place_count];
        for &e in &sp.final_marking {
            final_marking[place(e) as usize] = i64::from(tokens(e));
        }
        let costs: Vec<f64> = sp.cost.iter().map(|&c| c as f64).collect();
        let used: Vec<usize> = (0..rows.len()).filter(|&p| !rows[p].is_empty()).collect();
        let n = costs.len();
        let mut a = vec![0.0; used.len() * n];
        for (i, &p) in used.iter().enumerate() {
            for &(t, c) in &rows[p] {
                a[i * n + t as usize] += c;
            }
        }
        let simplex = DualSimplex::new(used.len(), n, a, costs.clone());
        Self {
            rows,
            final_marking,
            costs,
            used,
            simplex,
        }
    }

    /// Solves the marking equation from `marking`. Returns `None` when it
    /// has no non-negative solution: then the final marking is unreachable.
    pub(crate) fn solve(&mut self, marking: &[Packed]) -> Result<Option<Estimate>> {
        let mut rhs = self.final_marking.clone();
        for &e in marking {
            rhs[place(e) as usize] -= i64::from(tokens(e));
        }
        if rhs
            .iter()
            .zip(&self.rows)
            .any(|(&b, row)| row.is_empty() && b != 0)
        {
            return Ok(None);
        }
        let b: Vec<f64> = self.used.iter().map(|&p| rhs[p] as f64).collect();
        match self.simplex.solve(&b) {
            Ok(s) => Ok(Some(Estimate {
                h: Self::round_up(s.objective),
                objective: s.objective,
                x: s.x,
            })),
            Err(LpFailure::Infeasible) => Ok(None),
            Err(LpFailure::Stalled) => self.solve_from_scratch(&rhs),
        }
    }

    /// Solves the program with `good_lp` and `microlp`, without a warm start.
    fn solve_from_scratch(&self, rhs: &[i64]) -> Result<Option<Estimate>> {
        let mut vars = ProblemVariables::new();
        let x: Vec<Variable> = vars.add_vector(variable().min(0), self.costs.len());
        let objective: Expression = x.iter().zip(&self.costs).map(|(&v, &c)| c * v).sum();
        let mut problem = vars.minimise(objective).using(microlp);
        for (row, &b) in self.rows.iter().zip(rhs) {
            if row.is_empty() {
                if b != 0 {
                    return Ok(None);
                }
                continue;
            }
            let lhs: Expression = row.iter().map(|&(t, c)| c * x[t as usize]).sum();
            problem = problem.with(constraint!(lhs == b as f64));
        }
        let solution = match problem.solve() {
            Ok(s) => s,
            Err(ResolutionError::Infeasible) => return Ok(None),
            Err(e) => return Err(Error::LinearProgram(e.to_string())),
        };
        let values: Vec<f64> = x.iter().map(|&v| solution.value(v)).collect();
        let objective: f64 = values.iter().zip(&self.costs).map(|(v, c)| v * c).sum();
        Ok(Some(Estimate {
            h: Self::round_up(objective),
            objective,
            x: values,
        }))
    }

    /// The LP optimum rounded up to the next integer cost.
    fn round_up(objective: f64) -> u64 {
        (objective - EPS).ceil().max(0.0) as u64
    }

    /// Whether `x` minus one firing of `t` is still a solution, so the
    /// child's estimate is exact (pm4py's `__trust_solution` with its
    /// tolerance of 0.001).
    pub(crate) fn derived_is_exact(x: &[f64], t: usize) -> bool {
        x[t] >= 1.0 - 1e-3
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    use ichnos_model::{Marking, PetriNet};

    use super::*;
    use crate::alignments::costs::ModelCosts;
    use crate::alignments::marking::apply_delta;
    use crate::alignments::sync_product::{ModelPart, MoveSet};

    /// The warm-started simplex agrees with `good_lp` on every reachable
    /// marking of a small synchronous product with a silent loop.
    #[test]
    fn warm_start_matches_from_scratch() {
        let mut net = PetriNet::new("n");
        let source = net.add_place("source");
        let p1 = net.add_place("p1");
        let p2 = net.add_place("p2");
        let sink = net.add_place("sink");
        let arcs = [
            (source, "a", Some("a"), p1),
            (p1, "b", Some("b"), p2),
            (p1, "skip", None, p2),
            (p2, "back", None, p1),
            (p2, "c", Some("c"), sink),
        ];
        for (from, name, label, to) in arcs {
            let t = net.add_transition(name, label);
            net.add_input_arc(from, t).unwrap();
            net.add_output_arc(t, to).unwrap();
        }
        let im = Marking::from([(source, 1)]);
        let fm = Marking::from([(sink, 1)]);
        let model = ModelPart::new(&net, &im, &fm, &ModelCosts::standard(&net)).unwrap();
        let trace = ["b", "a", "c", "b", "c"];
        let sp = SyncProduct::new(&model, &trace, &[10_000; 5], MoveSet::All);
        let mut se = StateEquation::new(&sp);

        let mut seen = HashSet::from([sp.initial.clone()]);
        let mut queue = VecDeque::from([sp.initial.clone()]);
        let mut next = Vec::new();
        while let Some(m) = queue.pop_front() {
            let rhs: Vec<i64> = {
                let mut r = se.final_marking.clone();
                for &e in &m {
                    r[place(e) as usize] -= i64::from(tokens(e));
                }
                r
            };
            let warm = se.solve(&m).unwrap().map(|e| e.h);
            let cold = se.solve_from_scratch(&rhs).unwrap().map(|e| e.h);
            assert_eq!(warm, cold, "marking {m:?}");
            for t in 0..sp.len() {
                let enabled = sp
                    .pre(t)
                    .iter()
                    .all(|&(p, w)| m.iter().any(|&e| place(e) == p && tokens(e) >= w));
                // Bound the silent loop.
                if enabled && seen.len() < 500 {
                    apply_delta(&m, sp.delta(t), &mut next);
                    if seen.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        assert!(seen.len() > 20);
    }
}
