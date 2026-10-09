//! Organizational roles, ported from pm4py's
//! `algo/organizational_mining/roles` (Burattin, Sperduti and Veluscek,
//! "Business models enhancement through discovery of roles", CIDM 2013).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use ichnos_core::{EventKeys, EventLog};
use indexmap::IndexMap;

use crate::error::{Result, event_str};

/// Roles merge while their similarity exceeds this (pm4py's default
/// `roles_threshold_parameter`).
const THRESHOLD: f64 = 0.65;

/// A role: activities done by a similar multiset of resources (pm4py's
/// `Role`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// The activities, sorted once roles merge.
    pub activities: Vec<String>,
    /// How many events of the role's activities each resource did.
    pub originator_importance: BTreeMap<String, u64>,
}

/// A role while merging: its activities and its resources in pm4py's
/// insertion order.
type Draft = (Vec<String>, IndexMap<String, u64>);

/// The organizational roles of a log, as pm4py's
/// `discover_organizational_roles` finds them.
///
/// Each activity starts as its own role, holding the resources that do it
/// and how often. The two most similar roles merge while their similarity,
/// the weighted Jaccard index of their normalized resource multisets, is
/// above 0.65. Roles come with the most activities first, then the most
/// resources, then by activity names in reverse.
///
/// # Errors
///
/// An event without the activity or resource attribute.
pub fn discover_organizational_roles(log: &EventLog, keys: &EventKeys) -> Result<Vec<Role>> {
    let mut couples: IndexMap<(String, String), u64> = IndexMap::new();
    for (t, trace) in log.traces.iter().enumerate() {
        for e in 0..trace.events.len() {
            let res = event_str(log, t, e, &keys.resource)?;
            let act = event_str(log, t, e, &keys.activity)?;
            *couples.entry((res, act)).or_default() += 1;
        }
    }
    let mut by_activity: IndexMap<String, IndexMap<String, u64>> = IndexMap::new();
    for ((res, act), count) in couples {
        by_activity.entry(act).or_default().insert(res, count);
    }
    let mut roles: Vec<Draft> = by_activity
        .into_iter()
        .map(|(act, res)| (vec![act], res))
        .collect();
    sort_by_size(&mut roles);
    while merge_most_similar(&mut roles) {}
    sort_by_size(&mut roles);
    Ok(roles
        .into_iter()
        .map(|(activities, res)| Role {
            activities,
            originator_importance: res.into_iter().collect(),
        })
        .collect())
}

/// pm4py's activity-set key: the names joined by commas.
fn joined(activities: &[String]) -> String {
    activities.join(",")
}

/// Sorts by activity count, resource count and sorted joined names, all
/// descending.
fn sort_by_size(roles: &mut [Draft]) {
    roles.sort_by_cached_key(|(acts, res)| {
        let mut sorted = acts.clone();
        sorted.sort();
        Reverse((acts.len(), res.len(), joined(&sorted)))
    });
}

/// One round of pm4py's `aggregate_roles_iteration`. Returns whether two
/// roles merged.
fn merge_most_similar(roles: &mut Vec<Draft>) -> bool {
    let mut best: Option<(f64, usize, usize)> = None;
    let key = |i: usize, j: usize| (joined(&roles[i].0), joined(&roles[j].0));
    for i in 0..roles.len() {
        for j in i + 1..roles.len() {
            let sim = similarity(&roles[i].1, &roles[j].1);
            let better = match best {
                None => true,
                Some((s, bi, bj)) => sim > s || (sim == s && key(i, j) < key(bi, bj)),
            };
            if better {
                best = Some((sim, i, j));
            }
        }
    }
    let Some((sim, i, j)) = best else {
        return false;
    };
    if sim <= THRESHOLD {
        return false;
    }
    let (acts2, res2) = roles.remove(j);
    let (acts1, res1) = roles.remove(i);
    let acts: Vec<String> = acts1
        .into_iter()
        .chain(acts2)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut res = res1;
    for (r, c) in res2 {
        *res.entry(r).or_default() += c;
    }
    roles.push((acts, res));
    roles.sort_by_cached_key(|(acts, _)| joined(acts));
    true
}

/// pm4py's `find_role_similarity`: the sum of the minimums over the sum of
/// the maximums of the two normalized multisets.
fn similarity(a: &IndexMap<String, u64>, b: &IndexMap<String, u64>) -> f64 {
    let normalize = |m: &IndexMap<String, u64>| {
        let total = m.values().sum::<u64>() as f64;
        m.iter()
            .map(|(k, &v)| (k.clone(), v as f64 / total))
            .collect::<IndexMap<_, _>>()
    };
    let (a, b) = (normalize(a), normalize(b));
    let mut num = 0.0;
    let mut den = 0.0;
    for (k, &va) in &a {
        match b.get(k) {
            Some(&vb) => {
                num += va.min(vb);
                den += va.max(vb);
            }
            None => den += va,
        }
    }
    for (k, &vb) in &b {
        if !a.contains_key(k) {
            den += vb;
        }
    }
    num / den
}
