// Rectangular shortest-augmenting-path assignment, O(min(n,m)^2 max(n,m)).
// Each source trace is used at most once; excess query traces have no match.
pub(crate) fn assign(query: &[Vec<String>], source: &[Vec<String>]) -> Vec<Option<usize>> {
    let transpose = query.len() > source.len();
    let (rows, columns) = if transpose {
        (source, query)
    } else {
        (query, source)
    };
    let (n, m) = (rows.len(), columns.len());
    let mut u = vec![0i64; n + 1];
    let mut v = vec![0i64; m + 1];
    let mut p = vec![0usize; m + 1];
    let mut way = vec![0usize; m + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut min = vec![i64::MAX; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = i64::MAX;
            let mut j1 = 0;
            for j in 1..=m {
                if used[j] {
                    continue;
                }
                let cost = distance(&rows[i0 - 1], &columns[j - 1]) as i64 - u[i0] - v[j];
                if cost < min[j] {
                    min[j] = cost;
                    way[j] = j0;
                }
                if min[j] < delta {
                    delta = min[j];
                    j1 = j;
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    min[j] = min[j].saturating_sub(delta);
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut matches = vec![None; query.len()];
    for j in 1..=m {
        if p[j] > 0 {
            if transpose {
                matches[j - 1] = Some(p[j] - 1);
            } else {
                matches[p[j] - 1] = Some(j - 1);
            }
        }
    }
    matches
}
fn distance(a: &[String], b: &[String]) -> usize {
    let mut row: Vec<_> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(old + 1)
                .min(previous + usize::from(x != y));
            previous = old;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seq(s: &str) -> Vec<String> {
        s.chars().map(|c| c.to_string()).collect()
    }
    #[test]
    fn global_assignment_and_unmatched_rows() {
        let source = vec![seq("a"), seq("b")];
        let query = vec![seq("ab"), seq("a"), seq("b")];
        let m = assign(&query, &source);
        assert_eq!(m, vec![None, Some(0), Some(1)]);
        let query = vec![seq("b")];
        assert_eq!(assign(&query, &source), vec![Some(1)]);
    }
}
