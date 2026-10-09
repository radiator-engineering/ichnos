use crate::{Error, Result};
/// Exact minimum-cost full bipartite assignment, dropping pairs whose second time precedes the first.
///
/// As in pm4py, ineligible edges receive a large cost during assignment and are removed afterwards.
/// Tied optimal assignments may choose different pairs; cardinality and objective are equivalent.
pub fn exact_match_minimum_average(left: &[f64], right: &[f64]) -> Result<Vec<(f64, f64)>> {
    if left.iter().chain(right).any(|v| !v.is_finite()) {
        return Err(Error::InvalidOption("matching times must be finite"));
    }
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    if left
        .iter()
        .any(|a| right.iter().any(|b| a <= b && !(b - a).is_finite()))
    {
        return Err(Error::InvalidOption("matching differences must be finite"));
    }
    let transpose = left.len() > right.len();
    let (n, m) = if transpose {
        (right.len(), left.len())
    } else {
        (left.len(), right.len())
    };
    let cost = |i: usize, j: usize| {
        let (a, b) = if transpose {
            (left[j], right[i])
        } else {
            (left[i], right[j])
        };
        if a <= b { b - a } else { i64::MAX as f64 }
    };
    // Rectangular Hungarian algorithm, one-based potentials and augmenting paths.
    let mut u = vec![0.0; n + 1];
    let mut v = vec![0.0; m + 1];
    let mut assignment = vec![0; m + 1];
    let mut way = vec![0; m + 1];
    for i in 1..=n {
        assignment[0] = i;
        let mut j0 = 0;
        let mut minimum = vec![f64::INFINITY; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let i0 = assignment[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0;
            for j in 1..=m {
                if !used[j] {
                    let current = cost(i0 - 1, j - 1) - u[i0] - v[j];
                    if current < minimum[j] {
                        minimum[j] = current;
                        way[j] = j0;
                    }
                    if minimum[j] < delta {
                        delta = minimum[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[assignment[j]] += delta;
                    v[j] -= delta;
                } else {
                    minimum[j] -= delta;
                }
            }
            j0 = j1;
            if assignment[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            assignment[j0] = assignment[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut result = Vec::new();
    for (j, &i) in assignment.iter().enumerate().skip(1) {
        if i != 0 {
            let (li, ri) = if transpose {
                (j - 1, i - 1)
            } else {
                (i - 1, j - 1)
            };
            if left[li] <= right[ri] {
                result.push((li, left[li], right[ri]));
            }
        }
    }
    result.sort_by_key(|v| v.0);
    Ok(result.into_iter().map(|(_, a, b)| (a, b)).collect())
}
