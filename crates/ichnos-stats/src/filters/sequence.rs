use super::{Retention, select, slice};
use crate::{
    Result,
    attributes::Scalar,
    variants::{Variant, get_variants},
};
use ichnos_core::{EventKeys, EventLog};
use std::collections::{BTreeSet, HashSet};

fn sequences(log: &EventLog, keys: &EventKeys) -> Result<Vec<Vec<String>>> {
    let seq = log.activity_sequences(keys)?;
    Ok(seq
        .traces
        .iter()
        .map(|t| {
            t.iter()
                .map(|a| seq.activities.name(*a).to_owned())
                .collect()
        })
        .collect())
}
fn endpoint(
    log: &EventLog,
    keys: &EventKeys,
    activities: &[String],
    retention: Retention,
    start: bool,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    select(log, |i, _| {
        Ok((if start { seq[i].first() } else { seq[i].last() })
            .is_some_and(|a| retention.accepts(activities.contains(a))))
    })
}
/// Filter nonempty cases by their first activity.
pub fn filter_start_activities(
    log: &EventLog,
    keys: &EventKeys,
    activities: &[String],
    retention: Retention,
) -> Result<EventLog> {
    endpoint(log, keys, activities, retention, true)
}
/// Filter nonempty cases by their final activity.
pub fn filter_end_activities(
    log: &EventLog,
    keys: &EventKeys,
    activities: &[String],
    retention: Retention,
) -> Result<EventLog> {
    endpoint(log, keys, activities, retention, false)
}
/// Filter exact activity sequences, preserving original case order.
pub fn filter_variants(
    log: &EventLog,
    keys: &EventKeys,
    variants: &[Variant],
    retention: Retention,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    let variants = variants.iter().collect::<BTreeSet<_>>();
    select(
        log,
        |i, _| Ok(retention.accepts(variants.contains(&seq[i]))),
    )
}
/// Retain the k most frequent variants; tied counts sort by decreasing variant text.
pub fn filter_variants_top_k(log: &EventLog, keys: &EventKeys, k: usize) -> Result<EventLog> {
    let mut counts = get_variants(log, keys)?.into_iter().collect::<Vec<_>>();
    counts.sort_by(|a, b| (b.1, &b.0).cmp(&(a.1, &a.0)));
    let variants = counts
        .into_iter()
        .take(k)
        .map(|(v, _)| v)
        .collect::<Vec<_>>();
    filter_variants(log, keys, &variants, Retention::Retain)
}
/// Retain each variant individually covering at least the given fraction of all cases.
pub fn filter_variants_by_coverage_percentage(
    log: &EventLog,
    keys: &EventKeys,
    minimum: f64,
) -> Result<EventLog> {
    super::dfg::fraction(minimum)?;
    let variants = get_variants(log, keys)?
        .into_iter()
        .filter(|(_, n)| *n as f64 >= minimum * log.len() as f64)
        .map(|(v, _)| v)
        .collect::<Vec<_>>();
    filter_variants(log, keys, &variants, Retention::Retain)
}
/// Filter cases having any admitted adjacent activity pair.
pub fn filter_directly_follows_relation(
    log: &EventLog,
    keys: &EventKeys,
    relations: &[(String, String)],
    retention: Retention,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    select(log, |i, _| {
        Ok(retention.accepts(
            seq[i]
                .windows(2)
                .any(|w| relations.iter().any(|(a, b)| w[0] == *a && w[1] == *b)),
        ))
    })
}
fn subsequence(sequence: &[String], pattern: &[String]) -> bool {
    let mut next = 0;
    for a in sequence {
        if pattern.get(next) == Some(a) {
            next += 1;
        }
    }
    next == pattern.len()
}
/// Filter cases containing any admitted ordered subsequence, without timestamp conditions.
pub fn filter_eventually_follows_relation(
    log: &EventLog,
    keys: &EventKeys,
    relations: &[Vec<String>],
    retention: Retention,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    select(log, |i, _| {
        Ok(retention.accepts(relations.iter().any(|p| subsequence(&seq[i], p))))
    })
}
/// Retain cases with at least minimum occurrences of an activity (pm4py default: 2).
pub fn filter_activities_rework(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
    minimum: usize,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    select(log, |i, _| {
        Ok(seq[i].iter().filter(|a| a.as_str() == activity).count() >= minimum)
    })
}
/// Choice of the first or last matching activity in a case.
#[derive(Clone, Copy, Debug, Default)]
pub enum Occurrence {
    /// First occurrence.
    #[default]
    First,
    /// Last occurrence.
    Last,
}
/// Options for prefixes and suffixes.
#[derive(Clone, Copy, Debug)]
pub struct SliceOptions {
    /// Exclude the matching activity itself.
    pub strict: bool,
    /// Which matching activity marks the slice boundary.
    pub occurrence: Occurrence,
}
impl Default for SliceOptions {
    fn default() -> Self {
        Self {
            strict: true,
            occurrence: Occurrence::First,
        }
    }
}
fn boundary(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
    options: SliceOptions,
    prefix: bool,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    let mut result = log.filter_traces(|_| false);
    for (i, t) in log.traces.iter().enumerate() {
        let found = match options.occurrence {
            Occurrence::First => seq[i].iter().position(|a| a == activity),
            Occurrence::Last => seq[i].iter().rposition(|a| a == activity),
        };
        if let Some(n) = found {
            let range = if prefix {
                0..n + usize::from(!options.strict)
            } else {
                n + usize::from(options.strict)..t.len()
            };
            result.traces.push(slice(t, range));
        }
    }
    Ok(result)
}
/// Extract prefixes from cases containing an activity; empty prefixes are preserved.
pub fn filter_prefixes(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
    options: SliceOptions,
) -> Result<EventLog> {
    boundary(log, keys, activity, options, true)
}
/// Extract suffixes from cases containing an activity; empty suffixes are preserved.
pub fn filter_suffixes(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
    options: SliceOptions,
) -> Result<EventLog> {
    boundary(log, keys, activity, options, false)
}
/// Subcase identifier options for between filtering.
#[derive(Clone, Debug)]
pub struct BetweenOptions {
    /// Trace attribute storing the subcase identifier (default `concept:name`).
    /// The default updates core case IDs, following pm4py's DataFrame path.
    pub case_id_attribute: String,
    /// Text separating the source ID from the zero-based subcase number.
    pub separator: String,
}
impl Default for BetweenOptions {
    fn default() -> Self {
        Self {
            case_id_attribute: "concept:name".into(),
            separator: "##@@".into(),
        }
    }
}
/// Extract complete, inclusive start-to-end subcases. Equal activity sets share their boundary event.
/// By default each subcase gets a unique core case ID derived from its source ID.
/// Use `case:concept:name` explicitly for pm4py's EventLog-path identifiers.
pub fn filter_between(
    log: &EventLog,
    keys: &EventKeys,
    starts: &[String],
    ends: &[String],
    options: &BetweenOptions,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    let mut result = log.filter_traces(|_| false);
    for (i, t) in log.traces.iter().enumerate() {
        let mut start = None;
        let mut count = 0;
        for (j, a) in seq[i].iter().enumerate() {
            if start.is_none() && starts.contains(a) {
                start = Some(j);
            } else if let Some(s) = start
                && ends.contains(a)
            {
                let mut sub = slice(t, s..j + 1);
                let id = t
                    .attributes
                    .get(&options.case_id_attribute)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| i.to_string());
                sub.attributes.insert(
                    options.case_id_attribute.clone(),
                    format!("{id}{}{count}", options.separator),
                );
                result.traces.push(sub);
                count += 1;
                start = if starts == ends { Some(j) } else { None };
            }
        }
    }
    Ok(result)
}
fn resources(log: &EventLog, keys: &EventKeys, activity: &str) -> Result<Vec<HashSet<Scalar>>> {
    let seq = sequences(log, keys)?;
    log.traces
        .iter()
        .enumerate()
        .map(|(i, t)| {
            t.events
                .iter()
                .enumerate()
                .filter(|(j, _)| seq[i][*j] == activity)
                .filter_map(|(_, e)| e.get(&keys.resource))
                .map(|v| {
                    Scalar::from_value(v).ok_or_else(|| crate::Error::NonScalar {
                        key: keys.resource.clone(),
                    })
                })
                .collect()
        })
        .collect()
}
/// Retain cases whose two activities use disjoint nonempty resource sets; exclusion selects violations.
pub fn filter_four_eyes_principle(
    log: &EventLog,
    keys: &EventKeys,
    first: &str,
    second: &str,
    retention: Retention,
) -> Result<EventLog> {
    let a = resources(log, keys, first)?;
    let b = resources(log, keys, second)?;
    select(log, |i, _| {
        Ok(!a[i].is_empty() && !b[i].is_empty() && retention.accepts(a[i].is_disjoint(&b[i])))
    })
}
/// Retain cases where the activity has more than one distinct resource; exclusion selects at most one.
pub fn filter_activity_done_different_resources(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
    retention: Retention,
) -> Result<EventLog> {
    let r = resources(log, keys, activity)?;
    select(log, |i, _| Ok(retention.accepts(r[i].len() > 1)))
}
/// One token in a complete-trace segment pattern.
#[derive(Clone, Debug)]
pub enum SegmentToken {
    /// Exactly this activity; punctuation is literal.
    Activity(String),
    /// Zero or more activities (pm4py's `...` token).
    AnyActivities,
}
fn segment(sequence: &[String], pattern: &[SegmentToken]) -> bool {
    let mut dp = vec![false; sequence.len() + 1];
    dp[0] = true;
    for token in pattern {
        let mut next = vec![false; dp.len()];
        match token {
            SegmentToken::AnyActivities => {
                let mut seen = false;
                for i in 0..dp.len() {
                    seen |= dp[i];
                    next[i] = seen;
                }
            }
            SegmentToken::Activity(a) => {
                for i in 0..sequence.len() {
                    next[i + 1] = dp[i] && sequence[i] == *a;
                }
            }
        }
        dp = next;
    }
    dp[sequence.len()]
}
/// Filter complete traces against activity/wildcard patterns, without comma-delimited regex conversion.
pub fn filter_trace_segments(
    log: &EventLog,
    keys: &EventKeys,
    patterns: &[Vec<SegmentToken>],
    retention: Retention,
) -> Result<EventLog> {
    let seq = sequences(log, keys)?;
    select(log, |i, _| {
        Ok(retention.accepts(patterns.iter().any(|p| segment(&seq[i], p))))
    })
}
