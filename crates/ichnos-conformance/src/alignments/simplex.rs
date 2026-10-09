//! A dense dual simplex for `min c·x` subject to `A x = b`, `x >= 0`, with
//! `c >= 0`, that keeps its basis between calls with different `b`.
//!
//! The state-equation heuristic solves this program once per search state.
//! Within one search `A` and `c` never change; only `b` does. A basis that
//! was optimal for one `b` therefore stays dual feasible for every other
//! `b`, so each solve starts from the previous basis and the dual simplex
//! needs only a few pivots to restore primal feasibility. General LP
//! libraries (`good_lp` with `microlp`) rebuild and solve each program from
//! scratch, which made them the bottleneck of A*.
//!
//! Each row has an artificial column fixed at zero. The artificial columns
//! form the first basis, which is dual feasible because `c >= 0`; they keep
//! the basis square when `A` has dependent rows (incidence matrices often
//! do), and the program is infeasible exactly when an artificial cannot be
//! driven to zero.

/// Feasibility tolerance on basic values.
const FEAS_TOL: f64 = 1e-9;
/// Smallest pivot magnitude accepted.
const PIVOT_TOL: f64 = 1e-9;
/// Refactor the tableau from the original matrix after this many pivots.
const REFACTOR_EVERY: usize = 100;
/// Largest residual `|A x - b|` accepted before refactoring and solving again.
const RESIDUAL_TOL: f64 = 1e-6;
/// Most negative reduced cost accepted on a non-basic column at the optimum.
const DUAL_TOL: f64 = 1e-9;

/// An optimal solution.
#[derive(Debug, Clone)]
pub(crate) struct LpSolution {
    pub(crate) objective: f64,
    pub(crate) x: Vec<f64>,
}

/// Why a solve gave no solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LpFailure {
    /// No `x >= 0` satisfies `A x = b`.
    Infeasible,
    /// The solver did not converge, even from a fresh basis with Bland's rule.
    Stalled,
}

#[derive(Debug, Clone)]
pub(crate) struct DualSimplex {
    m: usize,
    n: usize,
    /// The original `A`, row-major `m × n`.
    a: Vec<f64>,
    c: Vec<f64>,
    /// `B⁻¹ [A | I]`, row-major `m × (n + m)`.
    t: Vec<f64>,
    /// Reduced costs of the `n + m` columns.
    d: Vec<f64>,
    /// The basic column of each row.
    basis: Vec<usize>,
    /// The row of each basic column, or `usize::MAX`.
    row_of: Vec<usize>,
    xb: Vec<f64>,
    pivots: usize,
}

impl DualSimplex {
    /// A solver for `A` (row-major `m × n`) and costs `c >= 0`.
    pub(crate) fn new(m: usize, n: usize, a: Vec<f64>, c: Vec<f64>) -> Self {
        debug_assert_eq!(a.len(), m * n);
        debug_assert!(c.iter().all(|&v| v >= 0.0));
        let mut s = Self {
            m,
            n,
            a,
            c,
            t: Vec::new(),
            d: Vec::new(),
            basis: Vec::new(),
            row_of: Vec::new(),
            xb: vec![0.0; m],
            pivots: 0,
        };
        s.reset();
        s
    }

    fn cols(&self) -> usize {
        self.n + self.m
    }

    /// Back to the all-artificial basis.
    fn reset(&mut self) {
        let (m, n, cols) = (self.m, self.n, self.cols());
        self.t = vec![0.0; m * cols];
        for i in 0..m {
            self.t[i * cols..i * cols + n].copy_from_slice(&self.a[i * n..(i + 1) * n]);
            self.t[i * cols + n + i] = 1.0;
        }
        self.d = self.c.clone();
        self.d.resize(cols, 0.0);
        self.basis = (n..n + m).collect();
        self.row_of = vec![usize::MAX; cols];
        for i in 0..m {
            self.row_of[n + i] = i;
        }
        self.pivots = 0;
    }

    /// Recomputes the tableau for the current basis from `A`. Falls back to
    /// the artificial basis if the basis matrix is numerically singular.
    fn refactor(&mut self) {
        let (m, n, cols) = (self.m, self.n, self.cols());
        let column = |j: usize, i: usize| -> f64 {
            if j < n {
                self.a[i * n + j]
            } else if j - n == i {
                1.0
            } else {
                0.0
            }
        };
        // Gauss-Jordan on [B | I] with partial pivoting gives B⁻¹.
        let w = 2 * m;
        let mut g = vec![0.0; m * w];
        for i in 0..m {
            for (k, &j) in self.basis.iter().enumerate() {
                g[i * w + k] = column(j, i);
            }
            g[i * w + m + i] = 1.0;
        }
        for k in 0..m {
            let p = (k..m)
                .max_by(|&x, &y| g[x * w + k].abs().total_cmp(&g[y * w + k].abs()))
                .expect("rows left");
            if g[p * w + k].abs() < PIVOT_TOL {
                self.reset();
                return;
            }
            if p != k {
                for c in 0..w {
                    g.swap(p * w + c, k * w + c);
                }
            }
            let piv = g[k * w + k];
            for c in 0..w {
                g[k * w + c] /= piv;
            }
            for i in 0..m {
                if i != k {
                    let f = g[i * w + k];
                    if f != 0.0 {
                        for c in 0..w {
                            g[i * w + c] -= f * g[k * w + c];
                        }
                    }
                }
            }
        }
        // Row k of B⁻¹ belongs to the basic column basis[k].
        let mut t = vec![0.0; m * cols];
        for k in 0..m {
            let inv = &g[k * w + m..k * w + w];
            let row = &mut t[k * cols..(k + 1) * cols];
            for (i, &v) in inv.iter().enumerate() {
                if v != 0.0 {
                    for (r, &a) in row[..n].iter_mut().zip(&self.a[i * n..(i + 1) * n]) {
                        *r += v * a;
                    }
                    row[n + i] += v;
                }
            }
        }
        self.t = t;
        self.d = self.c.clone();
        self.d.resize(cols, 0.0);
        for k in 0..m {
            let cb = self.cost(self.basis[k]);
            if cb != 0.0 {
                for j in 0..cols {
                    self.d[j] -= cb * self.t[k * cols + j];
                }
            }
        }
        self.pivots = 0;
    }

    fn cost(&self, j: usize) -> f64 {
        if j < self.n { self.c[j] } else { 0.0 }
    }

    fn pivot(&mut self, r: usize, j: usize) {
        let cols = self.cols();
        let piv = self.t[r * cols + j];
        for v in &mut self.t[r * cols..(r + 1) * cols] {
            *v /= piv;
        }
        self.xb[r] /= piv;
        let (before, rest) = self.t.split_at_mut(r * cols);
        let (pivot_row, after) = rest.split_at_mut(cols);
        for (i, row) in before
            .chunks_exact_mut(cols)
            .chain(after.chunks_exact_mut(cols))
            .enumerate()
        {
            let i = if i < r { i } else { i + 1 };
            let f = row[j];
            if f != 0.0 {
                for (v, &p) in row.iter_mut().zip(pivot_row.iter()) {
                    *v -= f * p;
                }
                self.xb[i] -= f * self.xb[r];
            }
        }
        let f = self.d[j];
        if f != 0.0 {
            for (v, &p) in self.d.iter_mut().zip(pivot_row.iter()) {
                *v -= f * p;
            }
        }
        let leaving = self.basis[r];
        self.row_of[leaving] = usize::MAX;
        self.row_of[j] = r;
        self.basis[r] = j;
        self.pivots += 1;
    }

    /// Sets `xb = B⁻¹ b`.
    fn load(&mut self, b: &[f64]) {
        let (m, n, cols) = (self.m, self.n, self.cols());
        for i in 0..m {
            let inv = &self.t[i * cols + n..(i + 1) * cols];
            self.xb[i] = inv.iter().zip(b).map(|(v, bv)| v * bv).sum();
        }
    }

    /// Runs dual simplex iterations until the basis is primal feasible.
    fn iterate(&mut self, bland: bool, limit: usize) -> Result<(), LpFailure> {
        let (n, cols) = (self.n, self.cols());
        for _ in 0..limit {
            if self.pivots >= REFACTOR_EVERY {
                let b = self.current_b();
                self.refactor();
                self.load(&b);
            }
            // Leaving row: the most infeasible basic value (Bland: lowest column).
            let mut leave: Option<(usize, f64)> = None;
            for i in 0..self.m {
                let v = self.xb[i];
                let infeas = if self.basis[i] >= n { v.abs() } else { -v };
                if infeas <= FEAS_TOL {
                    continue;
                }
                let better = match leave {
                    None => true,
                    Some((k, best)) => {
                        if bland {
                            self.basis[i] < self.basis[k]
                        } else {
                            infeas > best
                        }
                    }
                };
                if better {
                    leave = Some((i, infeas));
                }
            }
            let Some((r, _)) = leave else {
                return Ok(());
            };
            let increase = self.xb[r] < 0.0;
            // Entering column: the dual ratio test over structural columns.
            let row = &self.t[r * cols..r * cols + n];
            let mut enter: Option<(usize, f64, f64)> = None;
            for (j, &alpha) in row.iter().enumerate() {
                if self.row_of[j] != usize::MAX {
                    continue;
                }
                let a = if increase { -alpha } else { alpha };
                if a <= PIVOT_TOL {
                    continue;
                }
                let ratio = self.d[j].max(0.0) / a;
                let better = match enter {
                    None => true,
                    Some((_, best, best_a)) => {
                        if bland {
                            ratio < best - 1e-12
                        } else {
                            ratio < best - 1e-12 || (ratio <= best + 1e-12 && a > best_a)
                        }
                    }
                };
                if better {
                    enter = Some((j, ratio, a));
                }
            }
            let Some((j, _, _)) = enter else {
                return Err(LpFailure::Infeasible);
            };
            self.pivot(r, j);
        }
        Err(LpFailure::Stalled)
    }

    /// `b` recovered from the basic values: `b = B xb`.
    fn current_b(&self) -> Vec<f64> {
        let (m, n) = (self.m, self.n);
        let mut b = vec![0.0; m];
        for (k, &j) in self.basis.iter().enumerate() {
            let v = self.xb[k];
            if v == 0.0 {
                continue;
            }
            if j < n {
                for (i, bi) in b.iter_mut().enumerate() {
                    *bi += self.a[i * n + j] * v;
                }
            } else {
                b[j - n] += v;
            }
        }
        b
    }

    fn solution(&self) -> LpSolution {
        let mut x = vec![0.0; self.n];
        let mut objective = 0.0;
        for (k, &j) in self.basis.iter().enumerate() {
            if j < self.n {
                let v = self.xb[k].max(0.0);
                x[j] = v;
                objective += self.c[j] * v;
            }
        }
        LpSolution { objective, x }
    }

    fn residual(&self, x: &[f64], b: &[f64]) -> f64 {
        let n = self.n;
        (0..self.m)
            .map(|i| {
                let ax: f64 = self.a[i * n..(i + 1) * n]
                    .iter()
                    .zip(x)
                    .map(|(a, x)| a * x)
                    .sum();
                (ax - b[i]).abs()
            })
            .fold(0.0, f64::max)
    }

    /// Solves for right-hand side `b`, starting from the last basis.
    pub(crate) fn solve(&mut self, b: &[f64]) -> Result<LpSolution, LpFailure> {
        let limit = 50 * (self.m + self.n) + 1000;
        self.load(b);
        let mut outcome = self.iterate(false, limit);
        // Accept a basis only if it is primal and dual feasible. The ratio
        // test clamps negative reduced costs, so drift could otherwise hide
        // a basis that is not optimal and overestimate the heuristic.
        let solution_ok = |s: &Self| {
            let dual_ok = (0..s.n).all(|j| s.row_of[j] != usize::MAX || s.d[j] >= -DUAL_TOL);
            let sol = s.solution();
            (dual_ok && s.residual(&sol.x, b) <= RESIDUAL_TOL).then_some(sol)
        };
        if outcome.is_ok() {
            if let Some(sol) = solution_ok(self) {
                return Ok(sol);
            }
            outcome = Err(LpFailure::Stalled);
        }
        if outcome == Err(LpFailure::Infeasible) {
            // Confirm infeasibility on a freshly factored tableau: drift
            // can hide an entering column.
            self.refactor();
            self.load(b);
            outcome = self.iterate(false, limit);
            if outcome.is_ok() {
                if let Some(sol) = solution_ok(self) {
                    return Ok(sol);
                }
            } else if outcome == Err(LpFailure::Infeasible) {
                return Err(LpFailure::Infeasible);
            }
        }
        // Last resort: start over with Bland's rule, which cannot cycle.
        self.reset();
        self.load(b);
        self.iterate(true, 20 * limit)?;
        solution_ok(self).ok_or(LpFailure::Stalled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_and_warm_starts() {
        // x0 - x1 = b0, x1 + x2 = b1; costs 1, 2, 3.
        let a = vec![1.0, -1.0, 0.0, 0.0, 1.0, 1.0];
        let mut s = DualSimplex::new(2, 3, a, vec![1.0, 2.0, 3.0]);
        let sol = s.solve(&[1.0, 2.0]).unwrap();
        // x1 = 2, x0 = 3: cost 7; or x2 = 2, x0 = 1: cost 7.
        assert!((sol.objective - 7.0).abs() < 1e-9);
        let sol = s.solve(&[-1.0, 1.0]).unwrap();
        // x1 = 1, x0 = 0: cost 2.
        assert!((sol.objective - 2.0).abs() < 1e-9);
        assert_eq!(s.solve(&[-1.0, 0.0]).unwrap_err(), LpFailure::Infeasible);
        let sol = s.solve(&[0.0, 0.0]).unwrap();
        assert!(sol.objective.abs() < 1e-9);
    }

    #[test]
    fn rejects_a_basis_that_is_not_optimal() {
        // x0 + x1 = 1; costs 1, 2. Force x1 into the basis: it is primal
        // feasible with cost 2, but x0 has reduced cost -1.
        let mut s = DualSimplex::new(1, 2, vec![1.0, 1.0], vec![1.0, 2.0]);
        s.load(&[1.0]);
        s.pivot(0, 1);
        assert!(s.d[0] < 0.0);
        let sol = s.solve(&[1.0]).unwrap();
        assert!((sol.objective - 1.0).abs() < 1e-9);
    }

    #[test]
    fn dependent_rows() {
        // Row 2 = row 0 + row 1.
        let a = vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let mut s = DualSimplex::new(3, 2, a, vec![1.0, 1.0]);
        let sol = s.solve(&[1.0, 2.0, 3.0]).unwrap();
        assert!((sol.objective - 3.0).abs() < 1e-9);
        assert_eq!(
            s.solve(&[1.0, 2.0, 4.0]).unwrap_err(),
            LpFailure::Infeasible
        );
    }
}
