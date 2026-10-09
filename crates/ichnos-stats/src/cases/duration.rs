use crate::{
    Error, Result,
    attributes::{Bandwidth, Density, KdeOptions},
    time::{TimeOptions, date, seconds},
};
use ichnos_core::{Event, EventKeys, EventLog, Position, Trace};
use indexmap::IndexMap;

/// Ordering key for case descriptions.
#[derive(Clone, Copy, Debug, Default)]
pub enum CaseSort {
    /// Trace case identifier (text order).
    #[default]
    Id,
    /// First event timestamp.
    Start,
    /// Last event timestamp.
    End,
    /// Elapsed or business duration.
    Duration,
}
/// Case description options.
#[derive(Clone, Debug)]
pub struct CaseOptions {
    /// Timestamp/business-hour settings. Case duration uses completion endpoints.
    pub time: TimeOptions,
    /// Trace ID attribute, default concept:name.
    pub case_id_attribute: String,
    /// Sort descriptions; None retains input order.
    pub sort: Option<CaseSort>,
    /// Reverse the selected ordering.
    pub descending: bool,
    /// Limit the number of nonempty traces described before duplicate IDs collapse.
    pub max_cases: Option<usize>,
}
impl Default for CaseOptions {
    fn default() -> Self {
        Self {
            time: TimeOptions::default(),
            case_id_attribute: "concept:name".to_owned(),
            sort: Some(CaseSort::Id),
            descending: false,
            max_cases: None,
        }
    }
}
/// First and last event times and the corresponding case duration.
#[derive(Clone, Debug)]
pub struct CaseDescription {
    /// First timestamp, Unix seconds.
    pub start_time: f64,
    /// Last timestamp, Unix seconds.
    pub end_time: f64,
    /// Duration in seconds.
    pub case_duration: f64,
}
/// Describe nonempty cases; unnamed cases use EMPTY plus their trace index.
pub fn get_cases_description(
    log: &EventLog,
    keys: &EventKeys,
    options: &CaseOptions,
) -> Result<IndexMap<String, CaseDescription>> {
    let mut rows = Vec::new();
    for (ti, t) in log.traces.iter().enumerate() {
        if let (Some(first), Some(last)) = (t.events.first(), t.events.last()) {
            let start = date(
                first,
                &keys.timestamp,
                Position::Event {
                    trace: ti,
                    event: 0,
                },
            )?;
            let end = date(
                last,
                &keys.timestamp,
                Position::Event {
                    trace: ti,
                    event: t.events.len() - 1,
                },
            )?;
            let id = t
                .attributes
                .get(&options.case_id_attribute)
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("EMPTY{ti}"));
            rows.push((
                id,
                CaseDescription {
                    start_time: seconds(start),
                    end_time: seconds(end),
                    case_duration: options.time.duration(start, end)?,
                },
            ));
        }
    }
    if let Some(sort) = options.sort {
        rows.sort_by(|a, b| {
            let order = match sort {
                CaseSort::Id => a.0.cmp(&b.0),
                CaseSort::Start => a.1.start_time.total_cmp(&b.1.start_time),
                CaseSort::End => a.1.end_time.total_cmp(&b.1.end_time),
                CaseSort::Duration => a.1.case_duration.total_cmp(&b.1.case_duration),
            };
            if options.descending {
                order.reverse()
            } else {
                order
            }
        });
    }
    if let Some(n) = options.max_cases {
        rows.truncate(n);
    }
    Ok(rows.into_iter().collect())
}
/// Index traces by case ID; the final trace wins for duplicate IDs.
pub fn index_log_caseid<'a>(
    log: &'a EventLog,
    attribute: &str,
) -> Result<IndexMap<String, &'a Trace>> {
    log.traces
        .iter()
        .enumerate()
        .map(|(i, t)| {
            Ok((
                t.attributes
                    .get(attribute)
                    .ok_or(Error::MissingCaseId(i))?
                    .to_string(),
                t,
            ))
        })
        .collect()
}
/// Borrow the events of the last case with the requested identifier.
pub fn get_events<'a>(log: &'a EventLog, case_id: &str, attribute: &str) -> Result<&'a [Event]> {
    Ok(&index_log_caseid(log, attribute)?
        .get(case_id)
        .ok_or(Error::InvalidOption("unknown case identifier"))?
        .events)
}
/// Case durations sorted ascending, after description options and duplicate-ID handling.
pub fn get_all_case_durations(
    log: &EventLog,
    keys: &EventKeys,
    options: &CaseOptions,
) -> Result<Vec<f64>> {
    let mut values = get_cases_description(log, keys, options)?
        .into_values()
        .map(|v| v.case_duration)
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    Ok(values)
}
/// Duration of the named case.
pub fn get_case_duration(
    log: &EventLog,
    keys: &EventKeys,
    case_id: &str,
    options: &CaseOptions,
) -> Result<f64> {
    Ok(get_cases_description(log, keys, options)?
        .get(case_id)
        .ok_or(Error::InvalidOption("unknown case identifier"))?
        .case_duration)
}
/// Reference first-quartile helper: selects index floor(3n/4), despite its name.
pub fn get_first_quartile_case_duration(
    log: &EventLog,
    keys: &EventKeys,
    options: &CaseOptions,
) -> Result<f64> {
    let values = get_all_case_durations(log, keys, options)?;
    Ok(if values.is_empty() {
        0.0
    } else {
        values[values.len() * 3 / 4]
    })
}
/// Reference median helper: selects the upper middle element for even sample sizes.
pub fn get_median_case_duration(
    log: &EventLog,
    keys: &EventKeys,
    options: &CaseOptions,
) -> Result<f64> {
    let values = get_all_case_durations(log, keys, options)?;
    Ok(if values.is_empty() {
        0.0
    } else {
        values[values.len() / 2]
    })
}
fn endpoint_average(
    log: &EventLog,
    keys: &EventKeys,
    options: &TimeOptions,
    last: bool,
) -> Result<f64> {
    let mut times = Vec::new();
    for (ti, t) in log.traces.iter().enumerate() {
        if t.events
            .first()
            .is_some_and(|e| e.get(&keys.timestamp).is_some())
        {
            let ei = if last { t.events.len() - 1 } else { 0 };
            times.push(date(
                &t.events[ei],
                &keys.timestamp,
                Position::Event {
                    trace: ti,
                    event: ei,
                },
            )?);
        }
    }
    times.sort();
    let differences = times
        .windows(2)
        .map(|w| options.duration(w[0], w[1]))
        .collect::<Result<Vec<_>>>()?;
    Ok(if differences.is_empty() {
        0.0
    } else {
        differences.iter().sum::<f64>() / differences.len() as f64
    })
}
/// Mean time between sorted case starts, skipping empty/missing first events.
pub fn get_case_arrival_average(
    log: &EventLog,
    keys: &EventKeys,
    options: &TimeOptions,
) -> Result<f64> {
    endpoint_average(log, keys, options, false)
}
/// Mean time between sorted case completions.
pub fn get_case_dispersion_average(
    log: &EventLog,
    keys: &EventKeys,
    options: &TimeOptions,
) -> Result<f64> {
    endpoint_average(log, keys, options, true)
}
/// Canonical arrival helper.
pub use get_case_arrival_average as get_case_arrival_avg;
/// Canonical dispersion helper.
pub use get_case_dispersion_average as get_case_dispersion_avg;

/// One variant's frequency and optional mean case duration.
#[derive(Clone, Debug)]
pub struct VariantStatistics {
    /// Activity sequence.
    pub variant: Vec<String>,
    /// Number of traces.
    pub count: usize,
    /// Mean case duration, when requested.
    pub case_duration: Option<f64>,
}
/// Options for variant case statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct VariantStatisticsOptions {
    /// Include the mean endpoint duration of each variant.
    pub include_durations: bool,
    /// Maximum variants returned. None or zero retains all variants, as in pm4py.
    pub maximum: Option<usize>,
}
/// Count-then-variant descending statistics, optionally including durations and a limit.
pub fn get_variant_statistics(
    log: &EventLog,
    keys: &EventKeys,
    options: VariantStatisticsOptions,
) -> Result<Vec<VariantStatistics>> {
    let counts = crate::variants::get_variants(log, keys)?;
    let durations = if options.include_durations {
        Some(crate::variants::get_variants_along_with_case_durations(
            log, keys,
        )?)
    } else {
        None
    };
    let mut result = crate::variants::get_variants_sorted_by_count(&counts)
        .into_iter()
        .map(|(variant, count)| {
            let case_duration = durations
                .as_ref()
                .map(|d| d[&variant].durations.iter().sum::<f64>() / count as f64);
            VariantStatistics {
                variant,
                count,
                case_duration,
            }
        })
        .collect::<Vec<_>>();
    if let Some(n) = options.maximum.filter(|n| *n > 0) {
        result.truncate(n);
    }
    Ok(result)
}
/// A case-level row replacing dataframe variant columns.
#[derive(Clone, Debug)]
pub struct CaseVariant {
    /// Input trace index.
    pub trace: usize,
    /// Case ID as text.
    pub case_id: String,
    /// Activity sequence.
    pub variant: Vec<String>,
    /// Last minus first completion time, or zero for an empty trace.
    pub case_duration: f64,
}
/// Variant and duration rows in case-ID order, without dataframe backend dependencies.
pub fn get_variants_df_with_case_duration(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<Vec<CaseVariant>> {
    let descriptions = get_cases_description(log, keys, &CaseOptions::default())?;
    let groups = crate::variants::get_variants_from_log_trace_idx(log, keys)?;
    let mut result = Vec::new();
    for (variant, indices) in groups {
        for trace in indices {
            let case_id = log.traces[trace]
                .case_id()
                .ok_or(Error::MissingCaseId(trace))?
                .to_string();
            let case_duration = descriptions
                .get(&case_id)
                .map(|d| d.case_duration)
                .unwrap_or(0.0);
            result.push(CaseVariant {
                trace,
                case_id,
                variant: variant.clone(),
                case_duration,
            });
        }
    }
    result.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    Ok(result)
}
/// Canonical typed case/variant rows.
pub use get_variants_df_with_case_duration as get_variants_df;
/// Return typed case/variant rows together with sorted variant counts.
pub fn get_variants_df_and_list(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<(Vec<CaseVariant>, Vec<VariantStatistics>)> {
    Ok((
        get_variants_df(log, keys)?,
        get_variant_statistics(log, keys, VariantStatisticsOptions::default())?,
    ))
}
/// Case-duration KDE retaining pm4py's duplicate linear/geometric grid points.
pub fn get_kde_case_duration_values(values: &[f64], options: KdeOptions) -> Result<Density> {
    if values.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidOption("finite case durations required"));
    }
    if values.is_empty() {
        return Ok(Density::default());
    }
    if values.len() == 1 {
        return Ok(Density {
            x: values.to_vec(),
            y: vec![1.0],
        });
    }
    if options.graph_points < 4 {
        return Err(Error::InvalidOption(
            "case KDE requires at least four graph points",
        ));
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let factor = match options.bandwidth {
        Bandwidth::Scott => n.powf(-0.2),
        Bandwidth::Silverman => (n * 0.75).powf(-0.2),
        Bandwidth::Factor(v) => v,
    };
    if max <= 0.0 || variance <= 0.0 || !factor.is_finite() || factor <= 0.0 {
        return Err(Error::InvalidOption(
            "nonsingular case KDE and positive maximum required",
        ));
    }
    let half = options.graph_points / 2;
    let start = min.max(0.001);
    let mut x = Vec::new();
    for i in 0..half {
        let t = i as f64 / (half - 1) as f64;
        x.push(if i + 1 == half {
            max
        } else {
            min + (max - min) * t
        });
        x.push(if i == 0 {
            start
        } else if i + 1 == half {
            max
        } else {
            10.0_f64.powf(start.log10() + (max.log10() - start.log10()) * t)
        });
    }
    x.sort_by(f64::total_cmp);
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
/// KDE of described case durations.
pub fn get_kde_caseduration(
    log: &EventLog,
    keys: &EventKeys,
    cases: &CaseOptions,
    kde: KdeOptions,
) -> Result<Density> {
    get_kde_case_duration_values(&get_all_case_durations(log, keys, cases)?, kde)
}
