use crate::Result;
use ichnos_core::{EventKeys, EventLog};
use std::collections::{BTreeMap, BTreeSet};

/// Minimum number of intervening events between repeated occurrences of each activity.
pub fn get_minimum_self_distances(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<BTreeMap<String, usize>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = BTreeMap::new();
    for t in &seq.traces {
        let mut last = BTreeMap::new();
        for (i, &a) in t.iter().enumerate() {
            if let Some(previous) = last.insert(a, i) {
                let distance = i - previous - 1;
                let name = seq.activities.name(a).to_owned();
                result
                    .entry(name)
                    .and_modify(|v: &mut usize| *v = (*v).min(distance))
                    .or_insert(distance);
            }
        }
    }
    Ok(result)
}
/// Activities between occurrences achieving each positive minimum self-distance.
/// Activities with zero distance or no repeated occurrence are omitted.
pub fn get_minimum_self_distance_witnesses(
    log: &EventLog,
    keys: &EventKeys,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let distances = get_minimum_self_distances(log, keys)?;
    let seq = log.activity_sequences(keys)?;
    let mut result = distances
        .iter()
        .filter(|(_, d)| **d > 0)
        .map(|(a, _)| (a.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for t in &seq.traces {
        let mut last = BTreeMap::new();
        for (i, &a) in t.iter().enumerate() {
            if let Some(previous) = last.insert(a, i) {
                let name = seq.activities.name(a);
                if distances.get(name) == Some(&(i - previous - 1))
                    && let Some(witnesses) = result.get_mut(name)
                {
                    witnesses.extend(
                        t[previous + 1..i]
                            .iter()
                            .map(|&a| seq.activities.name(a).to_owned()),
                    );
                }
            }
        }
    }
    Ok(result)
}
/// Occurrences of an activity by zero-based position within its trace.
pub fn get_activity_position_summary(
    log: &EventLog,
    keys: &EventKeys,
    activity: &str,
) -> Result<BTreeMap<usize, usize>> {
    let seq = log.activity_sequences(keys)?;
    let mut result = BTreeMap::new();
    for t in seq.traces {
        for (i, a) in t.into_iter().enumerate() {
            if seq.activities.name(a) == activity {
                *result.entry(i).or_default() += 1;
            }
        }
    }
    Ok(result)
}
