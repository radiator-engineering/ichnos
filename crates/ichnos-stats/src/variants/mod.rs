mod entropy;
use crate::{Error, Result};
pub use entropy::*;
use ichnos_core::{EventKeys, EventLog, Trace};
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

/// Activity sequence identifying a process variant.
pub type Variant = Vec<String>;
/// Group variants by trace index, in first-appearance order.
pub fn get_variants_from_log_trace_idx(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<IndexMap<Variant, Vec<usize>>> {
    let variants = log.variants(keys)?;
    Ok(variants
        .iter()
        .map(|v| {
            (
                variants.names(v).map(str::to_owned).collect(),
                v.traces.clone(),
            )
        })
        .collect())
}
/// Variant occurrence counts.
pub fn get_variants(log: &EventLog, keys: &EventKeys) -> Result<IndexMap<Variant, usize>> {
    Ok(get_variants_from_log_trace_idx(log, keys)?
        .into_iter()
        .map(|(v, t)| (v, t.len()))
        .collect())
}
/// Variant-derived stochastic language.
pub use get_stochastic_language as get_language;
/// Variant counts use sequences, so tuple and ordinary variants have one representation.
pub use get_variants as get_variants_as_tuples;
/// Canonical count implementation for table backends.
pub use get_variants as get_variants_count;
/// Set of distinct activity sequences.
pub fn get_variants_set(log: &EventLog, keys: &EventKeys) -> Result<BTreeSet<Variant>> {
    Ok(get_variants(log, keys)?.into_keys().collect())
}
/// Borrow traces grouped by their activity sequence.
pub fn get_variant_traces<'a>(
    log: &'a EventLog,
    keys: &EventKeys,
) -> Result<IndexMap<Variant, Vec<&'a Trace>>> {
    convert_variants_trace_idx_to_trace_obj(log, &get_variants_from_log_trace_idx(log, keys)?)
}
/// Resolve trace indices to borrowed traces, rejecting invalid indices.
pub fn convert_variants_trace_idx_to_trace_obj<'a>(
    log: &'a EventLog,
    indices: &IndexMap<Variant, Vec<usize>>,
) -> Result<IndexMap<Variant, Vec<&'a Trace>>> {
    indices
        .iter()
        .map(|(v, ids)| {
            Ok((
                v.clone(),
                ids.iter()
                    .map(|&i| {
                        log.traces
                            .get(i)
                            .ok_or(Error::InvalidOption("trace index out of range"))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ))
        })
        .collect()
}
/// Sort variants descending by count, then descending lexicographically.
pub fn get_variants_sorted_by_count(counts: &IndexMap<Variant, usize>) -> Vec<(Variant, usize)> {
    let mut result: Vec<_> = counts.iter().map(|(v, &n)| (v.clone(), n)).collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    result
}
/// Normalized variant frequencies. An empty log gives an empty language.
pub fn get_stochastic_language(log: &EventLog, keys: &EventKeys) -> Result<IndexMap<Variant, f64>> {
    Ok(get_variants(log, keys)?
        .into_iter()
        .map(|(v, n)| (v, n as f64 / log.len() as f64))
        .collect())
}
/// Split into metadata-preserving logs grouped by variant.
pub fn split_by_process_variant(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<IndexMap<Variant, EventLog>> {
    Ok(get_variants_from_log_trace_idx(log, keys)?
        .into_iter()
        .map(|(v, ids)| {
            let subset = EventLog {
                attributes: log.attributes.clone(),
                extensions: log.extensions.clone(),
                globals: log.globals.clone(),
                classifiers: log.classifiers.clone(),
                traces: ids.into_iter().map(|i| log.traces[i].clone()).collect(),
            };
            (v, subset)
        })
        .collect())
}
/// Trace membership and duration in seconds for one variant.
#[derive(Clone, Debug)]
pub struct VariantDurations {
    /// Indices into the original log.
    pub traces: Vec<usize>,
    /// Last minus first timestamp, or zero for empty or missing endpoints.
    pub durations: Vec<f64>,
}
fn seconds(
    a: &ichnos_core::Event,
    b: &ichnos_core::Event,
    keys: &EventKeys,
) -> Result<Option<f64>> {
    let (Some(a), Some(b)) = (a.get(&keys.timestamp), b.get(&keys.timestamp)) else {
        return Ok(None);
    };
    let (Some(a), Some(b)) = (a.as_date(), b.as_date()) else {
        return Err(Error::InvalidOption("date timestamp required"));
    };
    let delta = b.signed_duration_since(a);
    Ok(Some(
        delta.num_seconds() as f64
            + (delta - ichnos_core::chrono::Duration::seconds(delta.num_seconds()))
                .num_nanoseconds()
                .unwrap() as f64
                / 1e9,
    ))
}
/// Variant membership together with case durations.
pub fn get_variants_along_with_case_durations(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<IndexMap<Variant, VariantDurations>> {
    get_variants_from_log_trace_idx(log, keys)?
        .into_iter()
        .map(|(v, indices)| {
            let durations = indices
                .iter()
                .map(|&i| {
                    let t = &log.traces[i];
                    match (t.events.first(), t.events.last()) {
                        (Some(a), Some(b)) => Ok(seconds(a, b, keys)?.unwrap_or(0.0)),
                        _ => Ok(0.0),
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((
                v,
                VariantDurations {
                    traces: indices,
                    durations,
                },
            ))
        })
        .collect()
}
/// Count cases in which an activity occurs more than once.
pub fn get_rework(log: &EventLog, keys: &EventKeys) -> Result<BTreeMap<String, usize>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = BTreeMap::new();
    for t in seq.traces {
        let mut counts = BTreeMap::new();
        for a in t {
            *counts.entry(a).or_insert(0) += 1;
        }
        for (a, n) in counts {
            if n > 1 {
                *result.entry(seq.activities.name(a).to_owned()).or_default() += 1;
            }
        }
    }
    Ok(result)
}
/// Rework summary for one case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseRework {
    /// Total number of activities.
    pub number_activities: usize,
    /// Occurrences beyond the first of each activity.
    pub rework: usize,
}
/// Rework by case identifier; duplicate identifiers retain the last case.
pub fn get_rework_cases(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<IndexMap<super::attributes::Scalar, CaseRework>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = IndexMap::new();
    for (i, t) in seq.traces.iter().enumerate() {
        let id = log.traces[i]
            .case_id()
            .ok_or(ichnos_core::Error::MissingCaseId(i))?;
        let id = super::attributes::Scalar::from_value(id).ok_or_else(|| Error::NonScalar {
            key: "concept:name".to_owned(),
        })?;
        result.insert(
            id,
            CaseRework {
                number_activities: t.len(),
                rework: t.len() - t.iter().collect::<BTreeSet<_>>().len(),
            },
        );
    }
    Ok(result)
}
/// Frequent subsequences with support counted once per trace. Markers match pm4py.
pub fn get_frequent_trace_segments(
    log: &EventLog,
    keys: &EventKeys,
    minimum: usize,
) -> Result<BTreeMap<Variant, usize>> {
    if minimum == 0 {
        return Err(Error::InvalidOption("minimum support must be positive"));
    }
    let seq = log.activity_sequences(keys)?;
    fn extend(
        prefix: Vec<ichnos_core::ActivityId>,
        projected: &[(usize, usize)],
        traces: &[Vec<ichnos_core::ActivityId>],
        minimum: usize,
        out: &mut Vec<(Vec<ichnos_core::ActivityId>, usize)>,
    ) {
        let mut next: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for &(ti, start) in projected {
            let mut seen = BTreeSet::new();
            for (offset, &a) in traces[ti][start..].iter().enumerate() {
                if seen.insert(a) {
                    next.entry(a).or_default().push((ti, start + offset + 1));
                }
            }
        }
        for (a, p) in next {
            if p.len() >= minimum {
                let mut pattern = prefix.clone();
                pattern.push(a);
                out.push((pattern.clone(), p.len()));
                extend(pattern, &p, traces, minimum, out);
            }
        }
    }
    let mut patterns = Vec::new();
    extend(
        Vec::new(),
        &(0..seq.traces.len()).map(|i| (i, 0)).collect::<Vec<_>>(),
        &seq.traces,
        minimum,
        &mut patterns,
    );
    Ok(patterns
        .into_iter()
        .map(|(p, n)| {
            let mut v = vec!["...".to_owned()];
            for a in p {
                v.push(seq.activities.name(a).to_owned());
                v.push("...".to_owned());
            }
            (v, n)
        })
        .collect())
}
/// Aggregation of path durations.
#[derive(Clone, Copy, Debug, Default)]
pub enum DurationAggregation {
    /// Arithmetic mean.
    #[default]
    Mean,
    /// Median.
    Median,
    /// Minimum.
    Min,
    /// Maximum.
    Max,
    /// Sum.
    Sum,
}
/// Duration at a directly-follows position in a variant.
#[derive(Clone, Debug)]
pub struct VariantPathDuration {
    /// Activity sequence.
    pub variant: Variant,
    /// Number of cases with the variant.
    pub variant_count: usize,
    /// Source position in the trace.
    pub position: usize,
    /// Zero-based occurrence number of this activity pair in the case.
    pub cumulative_occurrence: usize,
    /// Aggregated duration in seconds.
    pub duration: f64,
}
/// Aggregate nonnegative directly-follows durations at each variant position.
pub fn get_variants_paths_duration(
    log: &EventLog,
    keys: &EventKeys,
    aggregation: DurationAggregation,
) -> Result<Vec<VariantPathDuration>> {
    let mut result = Vec::new();
    for (variant, indices) in get_variants_from_log_trace_idx(log, keys)? {
        let has_start = log
            .traces
            .iter()
            .flat_map(|t| &t.events)
            .any(|e| e.get(&keys.start_timestamp).is_some());
        let mut samples: BTreeMap<usize, Vec<(f64, usize)>> = BTreeMap::new();
        for &i in &indices {
            let t = &log.traces[i];
            let mut occurrences = BTreeMap::new();
            for position in 0..t.events.len().saturating_sub(1) {
                let source = t.events[position]
                    .get(&keys.timestamp)
                    .and_then(|v| v.as_date())
                    .ok_or(Error::InvalidOption("path timestamp missing or not a date"))?;
                for target in position + 1..t.events.len() {
                    let target_time = t.events[target]
                        .get(if has_start {
                            &keys.start_timestamp
                        } else {
                            &keys.timestamp
                        })
                        .and_then(|v| v.as_date());
                    if let Some(target_time) = target_time.filter(|d| *d >= source) {
                        let delta = target_time.signed_duration_since(source);
                        let duration = delta.num_seconds() as f64
                            + (delta - ichnos_core::chrono::Duration::seconds(delta.num_seconds()))
                                .num_nanoseconds()
                                .unwrap() as f64
                                / 1e9;
                        let occurrence = occurrences
                            .entry((&variant[position], &variant[target]))
                            .or_insert(0);
                        samples
                            .entry(position)
                            .or_default()
                            .push((duration, *occurrence));
                        *occurrence += 1;
                        break;
                    }
                }
            }
        }
        for position in 0..variant.len().saturating_sub(1) {
            let data = samples.remove(&position).unwrap_or_default();
            let occurrence = data.iter().map(|d| d.1).min().unwrap_or(0);
            let mut durations = data.into_iter().map(|d| d.0).collect::<Vec<_>>();
            if !durations.is_empty() {
                durations.sort_by(f64::total_cmp);
                let n = durations.len();
                let duration = match aggregation {
                    DurationAggregation::Mean => durations.iter().sum::<f64>() / n as f64,
                    DurationAggregation::Sum => durations.iter().sum(),
                    DurationAggregation::Min => durations[0],
                    DurationAggregation::Max => durations[n - 1],
                    DurationAggregation::Median => {
                        (durations[(n - 1) / 2] + durations[n / 2]) / 2.0
                    }
                };
                result.push(VariantPathDuration {
                    variant: variant.clone(),
                    variant_count: indices.len(),
                    position,
                    cumulative_occurrence: occurrence,
                    duration,
                });
            }
        }
    }
    result.sort_by(|a, b| {
        b.variant_count
            .cmp(&a.variant_count)
            .then_with(|| b.variant.cmp(&a.variant))
            .then_with(|| a.position.cmp(&b.position))
    });
    Ok(result)
}
