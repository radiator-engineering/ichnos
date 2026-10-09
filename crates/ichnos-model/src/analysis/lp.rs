//! A dense linear program in pm4py's `solver.apply` form, solved with
//! `good_lp` and the pure-Rust `microlp` solver.

use good_lp::{
    Expression, ProblemVariables, ResolutionError, Solution, SolverModel, Variable, microlp,
    variable,
};

use super::AnalysisError;

/// Minimise `c·x` subject to `a_ub·x <= b_ub` and `a_eq·x == b_eq`.
///
/// Every variable is at least 0, as with scipy's default bounds.
pub(crate) struct LinearProgram {
    pub c: Vec<f64>,
    pub a_ub: Vec<Vec<f64>>,
    pub b_ub: Vec<f64>,
    pub a_eq: Vec<Vec<f64>>,
    pub b_eq: Vec<f64>,
    /// Restrict every variable to integers.
    pub integer: bool,
}

impl LinearProgram {
    /// An empty program over `n` variables with the given costs.
    pub fn new(c: Vec<f64>) -> Self {
        Self {
            c,
            a_ub: Vec::new(),
            b_ub: Vec::new(),
            a_eq: Vec::new(),
            b_eq: Vec::new(),
            integer: false,
        }
    }

    /// The optimal point, or `None` when the program is infeasible.
    pub fn solve(&self) -> Result<Option<Vec<f64>>, AnalysisError> {
        let mut vars = ProblemVariables::new();
        let x: Vec<Variable> = (0..self.c.len())
            .map(|_| {
                let v = variable().min(0);
                vars.add(if self.integer { v.integer() } else { v })
            })
            .collect();
        let row = |coefs: &[f64]| -> Expression {
            coefs
                .iter()
                .zip(&x)
                .filter(|(c, _)| **c != 0.0)
                .map(|(&c, &v)| c * v)
                .sum()
        };
        let mut problem = vars.minimise(row(&self.c)).using(microlp);
        for (a, &b) in self.a_ub.iter().zip(&self.b_ub) {
            problem = problem.with(row(a).leq(b));
        }
        for (a, &b) in self.a_eq.iter().zip(&self.b_eq) {
            problem = problem.with(row(a).eq(b));
        }
        match problem.solve() {
            Ok(s) => Ok(Some(x.iter().map(|&v| s.value(v)).collect())),
            Err(ResolutionError::Infeasible) => Ok(None),
            Err(e) => Err(AnalysisError::LinearProgram(e.to_string())),
        }
    }
}
