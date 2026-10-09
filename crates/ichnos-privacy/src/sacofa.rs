use crate::{PrivacyError, PrivacyOptions, mechanisms};
use ichnos_core::EventLog;
use rand::{Rng, seq::SliceRandom};
use std::collections::{BTreeMap, BTreeSet};
type Prefix = Vec<Option<String>>;
fn follows(trace: &[String], a: &str) -> BTreeSet<String> {
    trace
        .iter()
        .position(|x| x == a)
        .map(|i| trace[i + 1..].iter().cloned().collect())
        .unwrap_or_default()
}
fn precedes(trace: &[String], a: &str) -> BTreeSet<String> {
    trace
        .iter()
        .rposition(|x| x == a)
        .map(|i| trace[..i].iter().cloned().collect())
        .unwrap_or_default()
}
#[derive(Clone, Copy, PartialEq)]
enum Relation {
    Always,
    Never,
    Sometimes,
}
type Relations = BTreeMap<(String, String), Relation>;
fn relations(traces: &[Vec<String>], activities: &[String], reverse: bool) -> Relations {
    let all: BTreeSet<_> = activities.iter().cloned().collect();
    let mut result = BTreeMap::new();
    for a in activities {
        let mut always = all.clone();
        let mut never = all.clone();
        for t in traces.iter().filter(|t| t.contains(a)) {
            let seen = if reverse {
                precedes(t, a)
            } else {
                follows(t, a)
            };
            always = always.intersection(&seen).cloned().collect();
            never = never.difference(&seen).cloned().collect();
        }
        for b in activities {
            result.insert(
                (a.clone(), b.clone()),
                if never.contains(b) {
                    Relation::Never
                } else if always.contains(b) {
                    Relation::Always
                } else {
                    Relation::Sometimes
                },
            );
        }
    }
    result
}
fn violations(
    prefix: &Prefix,
    activities: &[String],
    starts: &BTreeSet<String>,
    follow: &Relations,
    precede: &Relations,
) -> usize {
    if prefix.len() == 1 {
        return usize::from(prefix[0].as_ref().is_none_or(|a| !starts.contains(a)));
    }
    let complete = prefix.last() == Some(&None);
    let trace: Vec<_> = prefix.iter().flatten().cloned().collect();
    let mut score = 0;
    for a in trace.iter().collect::<BTreeSet<_>>() {
        let f = follows(&trace, a);
        let p = precedes(&trace, a);
        for b in activities {
            let key = (a.clone(), b.clone());
            score += usize::from(f.contains(b) && follow[&key] == Relation::Never);
            score += usize::from(p.contains(b) && precede[&key] == Relation::Never);
            score += usize::from(complete && follow[&key] == Relation::Always && !f.contains(b));
            score += usize::from(precede[&key] == Relation::Always && !p.contains(b));
        }
    }
    score
}
pub(crate) fn query<R: Rng + ?Sized>(
    log: &EventLog,
    o: &PrivacyOptions,
    rng: &mut R,
) -> Result<Vec<Vec<String>>, PrivacyError> {
    let mut traces = Vec::new();
    let mut activities = Vec::new();
    let mut seen = BTreeSet::new();
    let mut known: BTreeMap<Prefix, u64> = BTreeMap::new();
    for trace in &log.traces {
        let mut labels = Vec::new();
        let mut prefix = Vec::new();
        for event in &trace.events {
            let a = event
                .get("concept:name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PrivacyError::Input("missing activity".into()))?
                .to_string();
            if seen.insert(a.clone()) {
                activities.push(a.clone());
            }
            labels.push(a.clone());
            prefix.push(Some(a));
            *known.entry(prefix.clone()).or_default() += 1;
        }
        prefix.push(None);
        *known.entry(prefix).or_default() += 1;
        traces.push(labels);
    }
    if activities.is_empty() {
        return Err(PrivacyError::Input("no nonempty traces".into()));
    }
    let starts = traces.iter().filter_map(|t| t.first().cloned()).collect();
    let follow = relations(&traces, &activities, false);
    let precede = relations(&traces, &activities, true);
    let mut frontier = vec![Vec::new()];
    let mut finished = BTreeMap::new();
    let mut expansions = 0;
    for depth in 1..=o.max_prefix_length {
        let mut candidates = Vec::new();
        let mut violating = Vec::new();
        for p in &frontier {
            for activity in activities
                .iter()
                .cloned()
                .map(Some)
                .chain(std::iter::once(None))
            {
                expansions += 1;
                if expansions > o.max_prefixes {
                    return Err(PrivacyError::Limit("prefix expansions"));
                }
                let mut ext = p.clone();
                ext.push(activity);
                let conform = violations(&ext, &activities, &starts, &follow, &precede) == 0;
                let count = known.get(&ext).copied().unwrap_or(0);
                if !conform {
                    violating.push(candidates.len());
                }
                candidates.push((ext, count, conform));
            }
        }
        let selected = mechanisms::universe(violating.len(), o.epsilon, rng);
        violating.shuffle(rng);
        for &i in violating.iter().take(selected) {
            candidates[i].2 = true;
        }
        frontier.clear();
        for (prefix, count, noise) in candidates {
            let count = if noise {
                (count as f64 + mechanisms::laplace(1. / o.epsilon, rng).trunc()).max(0.) as u64
            } else {
                count
            };
            if depth < o.max_prefix_length
                && prefix.last() != Some(&None)
                && count < o.pruning_count
            {
                continue;
            }
            if prefix.last() == Some(&None) || depth == o.max_prefix_length {
                let trace: Vec<_> = prefix.into_iter().flatten().collect();
                if !trace.is_empty() {
                    finished.insert(trace, count);
                }
            } else {
                frontier.push(prefix);
            }
        }
        if frontier.is_empty() {
            break;
        }
    }
    let total = finished
        .values()
        .try_fold(0usize, |n, &count| {
            usize::try_from(count).ok().and_then(|c| n.checked_add(c))
        })
        .ok_or(PrivacyError::Limit("trace count"))?;
    if total > o.max_traces {
        return Err(PrivacyError::Limit("trace count"));
    }
    if total == 0 {
        return Err(PrivacyError::EmptyQuery);
    }
    let mut output = Vec::with_capacity(total);
    for (trace, count) in finished {
        output.extend(std::iter::repeat_n(trace, count as usize));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn behavioral_relations_match_repeated_activity_goldens() {
        let g = ichnos_golden::golden("simulation", "privacy-behavioral-relations");
        let e = &g.expected;
        let traces: Vec<Vec<String>> = serde_json::from_value(e["input"].clone()).unwrap();
        let activities: Vec<String> = serde_json::from_value(e["activities"].clone()).unwrap();
        let follow = relations(&traces, &activities, false);
        let precede = relations(&traces, &activities, true);
        let starts = traces.iter().filter_map(|t| t.first().cloned()).collect();
        for row in e["rows"].as_array().unwrap() {
            let seq: Vec<String> = serde_json::from_value(row["prefix"].clone()).unwrap();
            let mut prefix: Prefix = seq.into_iter().map(Some).collect();
            if row["complete"].as_bool().unwrap() {
                prefix.push(None);
            }
            if prefix.len() == 1 {
                continue;
            }
            assert_eq!(
                violations(&prefix, &activities, &starts, &follow, &precede),
                row["violations"].as_u64().unwrap() as usize,
                "{row}"
            );
        }
    }
}
