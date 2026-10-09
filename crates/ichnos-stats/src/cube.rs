//! Process cubes over numeric features and one-hot feature prefixes.
use std::collections::{BTreeMap, BTreeSet};

/// Column data, preserving caller column order for one-hot axes.
#[derive(Debug, Clone)]
pub struct FeatureTable {
    /// Case ID for each row; repeated IDs are allowed.
    pub case_ids: Vec<String>,
    /// Numeric feature columns; None and NaN represent missing values.
    pub columns: Vec<(String, Vec<Option<f64>>)>,
}
/// Aggregation applied to nonmissing cell values.
#[derive(Debug, Clone, Copy, Default)]
pub enum Aggregation {
    /// Arithmetic mean.
    #[default]
    Mean,
    /// Sum (an observed all-missing cell sums to zero).
    Sum,
    /// Minimum.
    Min,
    /// Maximum.
    Max,
}
/// Cube axis bins.
#[derive(Debug, Clone, PartialEq)]
pub enum Bin {
    /// Right-closed numeric interval, with an inclusive lowest edge.
    Numeric {
        /// Lower boundary.
        lower: f64,
        /// Upper boundary.
        upper: f64,
        /// Whether the lower boundary is inclusive.
        include_lower: bool,
    },
    /// One-hot column name.
    Column(String),
}
/// Axis options.
#[derive(Debug, Clone)]
pub struct Axis {
    /// Exact numeric column, or prefix for `name_*` one-hot columns.
    pub column: String,
    /// Number of equal-width divisions (default 4).
    pub divisions: usize,
    /// Optional sorted/deduplicated numeric boundaries.
    pub boundaries: Option<Vec<f64>>,
}
impl Axis {
    /// Creates an axis with four divisions.
    pub fn new(column: impl Into<String>) -> Self {
        Self {
            column: column.into(),
            divisions: 4,
            boundaries: None,
        }
    }
}
/// Pivot values and case membership, indexed as [y][x].
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessCube {
    /// Column bins.
    pub x: Vec<Bin>,
    /// Row bins.
    pub y: Vec<Bin>,
    /// Missing/unobserved cells are None.
    pub values: Vec<Vec<Option<f64>>>,
    /// Membership includes rows whose aggregation value is missing.
    pub cases: Vec<Vec<BTreeSet<String>>>,
}
/// Invalid table shape, column or bin configuration.
#[derive(Debug, thiserror::Error)]
#[error("invalid process cube: {0}")]
pub struct CubeError(pub String);
type Membership = Vec<Vec<usize>>;
fn column<'a>(t: &'a FeatureTable, name: &str) -> Option<&'a [Option<f64>]> {
    t.columns
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_slice())
}
fn axis(t: &FeatureTable, a: &Axis) -> Result<(Vec<Bin>, Membership), CubeError> {
    if let Some(values) = column(t, &a.column) {
        let mut bounds = if let Some(b) = &a.boundaries {
            b.clone()
        } else {
            if a.divisions == 0 || a.divisions > 10_000 {
                return Err(CubeError("divisions must be in 1..=10000".into()));
            }
            let valid: Vec<_> = values
                .iter()
                .flatten()
                .copied()
                .filter(|x| x.is_finite())
                .collect();
            if valid.is_empty() {
                return Ok((Vec::new(), vec![Vec::new(); t.case_ids.len()]));
            }
            let min = valid.iter().copied().fold(f64::INFINITY, f64::min);
            let max = valid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if min == max {
                vec![min - 0.5, max + 0.5]
            } else {
                (0..=a.divisions)
                    .map(|i| min + (max - min) * i as f64 / a.divisions as f64)
                    .collect()
            }
        };
        if bounds.iter().any(|v| !v.is_finite()) || bounds.len() > 10_001 {
            return Err(CubeError("finite bounded bin boundaries required".into()));
        }
        bounds.sort_by(f64::total_cmp);
        bounds.dedup();
        if bounds.len() < 2 {
            return Err(CubeError(
                "at least two distinct boundaries required".into(),
            ));
        }
        let bins: Vec<_> = bounds
            .windows(2)
            .enumerate()
            .map(|(i, w)| Bin::Numeric {
                lower: w[0],
                upper: w[1],
                include_lower: i == 0,
            })
            .collect();
        let memberships = values
            .iter()
            .map(|v| {
                v.and_then(|v| {
                    bounds
                        .windows(2)
                        .enumerate()
                        .position(|(i, w)| (v > w[0] || i == 0 && v == w[0]) && v <= w[1])
                })
                .into_iter()
                .collect()
            })
            .collect();
        Ok((bins, memberships))
    } else {
        let prefix = format!("{}_", a.column);
        let columns: Vec<_> = t
            .columns
            .iter()
            .filter(|(n, _)| n.starts_with(&prefix))
            .collect();
        let bins = columns
            .iter()
            .map(|(n, _)| Bin::Column(n.clone()))
            .collect();
        let memberships = (0..t.case_ids.len())
            .map(|row| {
                columns
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (_, v))| v[row].filter(|x| *x >= 1.).map(|_| i))
                    .collect()
            })
            .collect();
        Ok((bins, memberships))
    }
}
/// Computes a process cube and case membership for each cell.
pub fn get_process_cube(
    t: &FeatureTable,
    x: &Axis,
    y: &Axis,
    aggregation_column: &str,
    aggregation: Aggregation,
) -> Result<ProcessCube, CubeError> {
    let mut names = BTreeSet::new();
    if t.columns.iter().any(|(n, v)| {
        !names.insert(n)
            || v.len() != t.case_ids.len()
            || v.iter().flatten().any(|v| v.is_infinite())
    }) {
        return Err(CubeError(
            "duplicate columns, row mismatch or infinity".into(),
        ));
    }
    let values = column(t, aggregation_column)
        .ok_or_else(|| CubeError(format!("unknown aggregation column {aggregation_column}")))?;
    let (xb, xm) = axis(t, x)?;
    let (yb, ym) = axis(t, y)?;
    let mut groups: BTreeMap<(usize, usize), (Vec<f64>, BTreeSet<String>)> = BTreeMap::new();
    for row in 0..t.case_ids.len() {
        for &xi in &xm[row] {
            for &yi in &ym[row] {
                let (v, c) = groups.entry((yi, xi)).or_default();
                if let Some(value) = values[row].filter(|v| v.is_finite()) {
                    v.push(value);
                }
                c.insert(t.case_ids[row].clone());
            }
        }
    }
    if groups.is_empty() {
        return Ok(ProcessCube {
            x: Vec::new(),
            y: Vec::new(),
            values: Vec::new(),
            cases: Vec::new(),
        });
    }
    let mut cube = ProcessCube {
        values: vec![vec![None; xb.len()]; yb.len()],
        cases: vec![vec![BTreeSet::new(); xb.len()]; yb.len()],
        x: xb,
        y: yb,
    };
    for ((yi, xi), (v, c)) in groups {
        let value = match aggregation {
            Aggregation::Sum => Some(v.iter().sum()),
            _ if v.is_empty() => None,
            Aggregation::Mean => Some(v.iter().sum::<f64>() / v.len() as f64),
            Aggregation::Min => Some(v.into_iter().fold(f64::INFINITY, f64::min)),
            Aggregation::Max => Some(v.into_iter().fold(f64::NEG_INFINITY, f64::max)),
        };
        cube.values[yi][xi] = value;
        cube.cases[yi][xi] = c;
    }
    Ok(cube)
}
