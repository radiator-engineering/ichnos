use crate::{Error, Result};
use ichnos_core::EventLog;

/// Gaussian bandwidth selection.
#[derive(Clone, Copy, Debug, Default)]
pub enum Bandwidth {
    /// Scott's rule n^(-1/5).
    #[default]
    Scott,
    /// Silverman's rule (3n/4)^(-1/5).
    Silverman,
    /// Explicit covariance factor (not an absolute bandwidth).
    Factor(f64),
}
/// Kernel density options.
#[derive(Clone, Copy, Debug)]
pub struct KdeOptions {
    /// Number of graph points before numeric deduplication.
    pub graph_points: usize,
    /// Number of date samples, in input order, including extremes.
    pub points_to_sample: usize,
    /// Covariance bandwidth factor.
    pub bandwidth: Bandwidth,
}
impl Default for KdeOptions {
    fn default() -> Self {
        Self {
            graph_points: 200,
            points_to_sample: 400,
            bandwidth: Bandwidth::Scott,
        }
    }
}
/// A density curve; dates use Unix seconds on the x axis.
#[derive(Clone, Debug, Default)]
pub struct Density {
    /// Sorted sample positions.
    pub x: Vec<f64>,
    /// Density at each position.
    pub y: Vec<f64>,
}
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            if i + 1 == n {
                b
            } else {
                a + (b - a) * i as f64 / (n - 1) as f64
            }
        })
        .collect()
}
fn estimate(values: &[f64], options: KdeOptions, date: bool) -> Result<Density> {
    if options.graph_points < 2 || options.points_to_sample == 0 {
        return Err(Error::InvalidOption(
            "KDE requires at least two graph points and nonzero sample count",
        ));
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidOption("KDE values must be finite"));
    }
    let factor = match options.bandwidth {
        Bandwidth::Scott => (values.len() as f64).powf(-0.2),
        Bandwidth::Silverman => (values.len() as f64 * 0.75).powf(-0.2),
        Bandwidth::Factor(v) => v,
    };
    if !factor.is_finite() || factor <= 0.0 {
        if values.is_empty() && !matches!(options.bandwidth, Bandwidth::Factor(_)) {
            return Ok(Density::default());
        }
        return Err(Error::InvalidOption(
            "bandwidth must be positive and finite",
        ));
    }
    if values.is_empty() {
        return Ok(Density::default());
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let a = values[0];
    let b = values[values.len() - 1];
    if a == b {
        let eps = if date {
            3600.0
        } else if a == 0.0 {
            1.0
        } else {
            (a.abs() * 0.01).max(1e-6)
        };
        let x = linspace(a - eps, a + eps, options.graph_points);
        let mut y = vec![0.0; x.len()];
        y[x.len() / 2] = 1.0;
        return Ok(Density { x, y });
    }
    let x = if date {
        linspace(a, b, options.graph_points)
    } else {
        let half = (options.graph_points / 2).max(2);
        let mut x = linspace(a, b, half);
        if a * b > 0.0 {
            let (start, end, sign) = if a > 0.0 {
                (a.max(1e-6), b, 1.0)
            } else {
                (a.abs(), b.abs().max(1e-6), -1.0)
            };
            let mut geometric: Vec<_> = linspace(start.log10(), end.log10(), half)
                .into_iter()
                .map(|v| sign * 10.0_f64.powf(v))
                .collect();
            geometric[0] = sign * start;
            geometric[half - 1] = sign * end;
            x.extend(geometric);
        }
        x.extend([a, b]);
        x.sort_by(f64::total_cmp);
        x.dedup();
        x
    };
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sigma = variance.sqrt() * factor;
    let y = x
        .iter()
        .map(|p| {
            values
                .iter()
                .map(|v| (-0.5 * ((p - v) / sigma).powi(2)).exp())
                .sum::<f64>()
                / (n * sigma * (2.0 * std::f64::consts::PI).sqrt())
        })
        .collect();
    Ok(Density { x, y })
}
/// Gaussian KDE with pm4py's combined linear and geometric grid.
pub fn get_kde_numeric_values(values: &[f64], options: KdeOptions) -> Result<Density> {
    estimate(values, options, false)
}
/// Date KDE on Unix seconds, with pm4py's input-order sampling.
pub fn get_kde_date_values(values: &[f64], options: KdeOptions) -> Result<Density> {
    if values.is_empty() {
        return estimate(values, options, true);
    }
    let m = options.points_to_sample;
    if m == 0 {
        return Err(Error::InvalidOption("nonzero sample count required"));
    }
    let n = values.len();
    let mut indices: Vec<_> = (0..m).map(|i| i * n / m + n / (2 * m)).collect();
    if !indices.contains(&0) {
        indices.insert(0, 0);
    }
    if !indices.contains(&(n - 1)) {
        indices.push(n - 1);
    }
    estimate(
        &indices.into_iter().map(|i| values[i]).collect::<Vec<_>>(),
        options,
        true,
    )
}
/// KDE of the present numeric event attributes.
pub fn get_kde_numeric_attribute(
    log: &EventLog,
    attribute: &str,
    options: KdeOptions,
) -> Result<Density> {
    let values = log
        .traces
        .iter()
        .flat_map(|t| &t.events)
        .filter_map(|e| e.get(attribute))
        .map(|v| {
            v.as_f64()
                .ok_or(Error::InvalidOption("numeric attribute required"))
        })
        .collect::<Result<Vec<_>>>()?;
    get_kde_numeric_values(&values, options)
}
/// KDE of present date attributes, interpreting local wall-clock values as UTC.
/// pm4py instead uses the process time zone; the golden generator pins it to UTC.
pub fn get_kde_date_attribute(
    log: &EventLog,
    attribute: &str,
    options: KdeOptions,
) -> Result<Density> {
    let values = log
        .traces
        .iter()
        .flat_map(|t| &t.events)
        .filter_map(|e| e.get(attribute))
        .map(|v| {
            v.as_date()
                .map(|d| {
                    d.naive_local().and_utc().timestamp() as f64
                        + d.timestamp_subsec_nanos() as f64 / 1e9
                })
                .ok_or(Error::InvalidOption("date attribute required"))
        })
        .collect::<Result<Vec<_>>>()?;
    get_kde_date_values(&values, options)
}
