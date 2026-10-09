//! Optimal transport between stochastic trace languages.
use good_lp::{Expression, ProblemVariables, Solution, SolverModel, microlp, variable};
use ichnos_model::Label;
use std::collections::BTreeMap;

/// Nonnegative mass assigned to each activity sequence.
pub type StochasticLanguage = BTreeMap<Vec<Label>, f64>;

/// An invalid language or unsuccessful transport optimization.
#[derive(Debug, thiserror::Error)]
pub enum EmdError {
    /// Mass must be finite and nonnegative.
    #[error("language masses must be finite and nonnegative")]
    InvalidMass,
    /// Both languages must carry the same total mass.
    #[error("language masses differ: {0} and {1}")]
    UnequalMass(f64, f64),
    /// The solver could not find a transport plan.
    #[error("transport optimization failed: {0}")]
    Solver(String),
}

/// Levenshtein distance divided by the longer trace's length.
/// Two empty traces have distance zero.
pub fn normalized_trace_distance(a: &[Label], b: &[Label]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(old + 1)
                .min(diagonal + usize::from(x != y));
            diagonal = old;
        }
    }
    row[b.len()] as f64 / a.len().max(b.len()) as f64
}

/// Minimum transport cost using normalized activity-sequence edit distance.
///
/// Mass is preserved, rather than silently normalizing unequal distributions.
/// Zero-total languages have distance zero. Zero-mass support is ignored.
pub fn compute_emd(a: &StochasticLanguage, b: &StochasticLanguage) -> Result<f64, EmdError> {
    let mass = |x: &StochasticLanguage| -> Result<f64, EmdError> {
        if x.values().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(EmdError::InvalidMass);
        }
        let total: f64 = x.values().sum();
        if !total.is_finite() {
            return Err(EmdError::InvalidMass);
        }
        Ok(total)
    };
    let ma = mass(a)?;
    let mb = mass(b)?;
    if (ma - mb).abs() > 1e-10 * ma.max(mb).max(1.0) {
        return Err(EmdError::UnequalMass(ma, mb));
    }
    if ma == 0.0 && mb == 0.0 {
        return Ok(0.0);
    }
    let a: Vec<_> = a.iter().filter(|(_, v)| **v > 0.0).collect();
    let b: Vec<_> = b.iter().filter(|(_, v)| **v > 0.0).collect();
    let mut variables = ProblemVariables::new();
    let flows: Vec<Vec<_>> = a
        .iter()
        .map(|_| {
            b.iter()
                .map(|_| variables.add(variable().min(0.0)))
                .collect()
        })
        .collect();
    let cost: Expression = a
        .iter()
        .enumerate()
        .flat_map(|(i, (x, _))| {
            b.iter()
                .enumerate()
                .map(move |(j, (y, _))| (i, j, normalized_trace_distance(x, y)))
        })
        .map(|(i, j, c)| c * flows[i][j])
        .sum();
    let mut problem = variables.minimise(cost.clone()).using(microlp);
    for (i, (_, m)) in a.iter().enumerate() {
        problem = problem.with(flows[i].iter().copied().sum::<Expression>().eq(**m));
    }
    for (j, (_, m)) in b.iter().enumerate() {
        problem = problem.with(flows.iter().map(|r| r[j]).sum::<Expression>().eq(**m));
    }
    let solution = problem
        .solve()
        .map_err(|e| EmdError::Solver(e.to_string()))?;
    Ok(solution.eval(cost))
}
