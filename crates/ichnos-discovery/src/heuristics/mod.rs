//! Classic Heuristics Miner, producing a heuristics net or its Petri net.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::heuristics_net::{HeuristicsEdge, Matrix};
use ichnos_model::{AcceptingPetriNet, HeuristicsNet, Label};

use crate::{Error, Result};

/// Thresholds for the classic Heuristics Miner.
#[derive(Debug, Clone, PartialEq)]
pub struct HeuristicsOptions {
    /// Minimum dependency measure (default 0.5).
    pub dependency_threshold: f64,
    /// Minimum AND measure (default 0.65).
    pub and_measure_threshold: f64,
    /// Minimum activity occurrences (default 1).
    pub min_activity_count: u64,
    /// Minimum directly-follows frequency (default 1).
    pub min_dfg_occurrences: u64,
    /// Remove edges below this fraction of both endpoints' maximum edge
    /// frequency (default 0.05; zero disables cleaning).
    pub dfg_pre_cleaning_noise_threshold: f64,
    /// Minimum length-two loop measure (default 0.5).
    pub loop_length_two_threshold: f64,
}

impl Default for HeuristicsOptions {
    fn default() -> Self {
        Self {
            dependency_threshold: 0.5,
            and_measure_threshold: 0.65,
            min_activity_count: 1,
            min_dfg_occurrences: 1,
            dfg_pre_cleaning_noise_threshold: 0.05,
            loop_length_two_threshold: 0.5,
        }
    }
}

fn get<T: Copy + Default>(m: &Matrix<T>, a: &Label, b: &Label) -> T {
    m.get(a)
        .and_then(|row| row.get(b))
        .copied()
        .unwrap_or_default()
}

/// Discovers a heuristics net from activity order. Timestamps are not needed.
/// Drawing colours and performance decorations belong to visualization and
/// are not stored in the model. All threshold fractions must be finite and
/// within `[0, 1]`.
pub fn heuristics_net(
    log: &EventLog,
    keys: &EventKeys,
    options: &HeuristicsOptions,
) -> Result<HeuristicsNet> {
    for (name, value) in [
        ("dependency_threshold", options.dependency_threshold),
        ("and_measure_threshold", options.and_measure_threshold),
        (
            "dfg_pre_cleaning_noise_threshold",
            options.dfg_pre_cleaning_noise_threshold,
        ),
        (
            "loop_length_two_threshold",
            options.loop_length_two_threshold,
        ),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(Error::HeuristicsThreshold {
                option: name,
                value,
            });
        }
    }
    let sequences = log.activity_sequences(keys)?;
    let mut h = HeuristicsNet {
        start_activities: vec![BTreeMap::new()],
        end_activities: vec![BTreeMap::new()],
        ..HeuristicsNet::default()
    };
    for trace in sequences.traces {
        let trace: Vec<Label> = trace
            .iter()
            .map(|&a| Label::from(sequences.activities.name(a)))
            .collect();
        for a in &trace {
            h.activities.insert(a.clone());
            *h.activity_occurrences.entry(a.clone()).or_default() += 1;
        }
        if let (Some(a), Some(b)) = (trace.first(), trace.last()) {
            *h.start_activities[0].entry(a.clone()).or_default() += 1;
            *h.end_activities[0].entry(b.clone()).or_default() += 1;
        }
        for p in trace.windows(2) {
            *h.dfg.entry((p[0].clone(), p[1].clone())).or_default() += 1;
        }
        for p in trace.windows(3) {
            *h.dfg_window_2_matrix
                .entry(p[0].clone())
                .or_default()
                .entry(p[2].clone())
                .or_default() += 1;
            if p[0] == p[2] && p[0] != p[1] {
                *h.freq_triples_matrix
                    .entry(p[0].clone())
                    .or_default()
                    .entry(p[1].clone())
                    .or_default() += 1;
            }
        }
    }
    let mut maxima: BTreeMap<Label, u64> = BTreeMap::new();
    for ((a, b), &n) in &h.dfg {
        for x in [a, b] {
            maxima
                .entry(x.clone())
                .and_modify(|v| *v = (*v).max(n))
                .or_insert(n);
        }
    }
    h.dfg.retain(|(a, b), n| {
        *n as f64 >= maxima[a].min(maxima[b]) as f64 * options.dfg_pre_cleaning_noise_threshold
    });
    for ((a, b), &n) in &h.dfg {
        h.dfg_matrix
            .entry(a.clone())
            .or_default()
            .insert(b.clone(), n);
        let reverse = h.dfg.get(&(b.clone(), a.clone())).copied().unwrap_or(0);
        let dependency = if a == b {
            n as f64 / (n as f64 + 1.0)
        } else {
            (n as f64 - reverse as f64) / (n as f64 + reverse as f64 + 1.0)
        };
        h.dependency_matrix
            .entry(a.clone())
            .or_default()
            .insert(b.clone(), dependency);
    }
    let eligible = |a: &Label, b: &Label, h: &HeuristicsNet| {
        h.activity_occurrences.get(a).copied().unwrap_or(0) >= options.min_activity_count
            && h.activity_occurrences.get(b).copied().unwrap_or(0) >= options.min_activity_count
            && get(&h.dfg_matrix, a, b) >= options.min_dfg_occurrences
    };
    let edges: Vec<_> = h
        .dfg
        .iter()
        .filter(|((a, b), _)| {
            eligible(a, b, &h) && get(&h.dependency_matrix, a, b) >= options.dependency_threshold
        })
        .map(|((a, b), &frequency)| {
            (
                a.clone(),
                b.clone(),
                HeuristicsEdge {
                    frequency,
                    dependency: get(&h.dependency_matrix, a, b),
                },
            )
        })
        .collect();
    for (a, b, edge) in edges {
        h.add_output_connection(&a, &b, edge);
        h.add_input_connection(&b, &a, edge);
    }
    // Measures are calculated before adding loop connections, as in pm4py.
    h.calculate_node_measures(
        options.and_measure_threshold,
        options.loop_length_two_threshold,
    );
    let loops: Vec<_> = h
        .nodes
        .iter()
        .flat_map(|(a, node)| {
            node.loop_length_two
                .keys()
                .map(|b| (a.clone(), b.clone()))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut added = BTreeSet::new();
    for (a, b) in loops {
        if eligible(&a, &b, &h)
            && get(&h.dependency_matrix, &a, &b) < options.dependency_threshold
            && get(&h.dependency_matrix, &b, &a) < options.dependency_threshold
        {
            for (from, to) in [(&a, &b), (&b, &a)] {
                if added.insert((from.clone(), to.clone())) {
                    h.add_output_connection(
                        from,
                        to,
                        HeuristicsEdge {
                            dependency: 0.0,
                            frequency: get(&h.dfg_matrix, from, to),
                        },
                    );
                    // The source uses the reverse frequency on loop inputs.
                    h.add_input_connection(
                        to,
                        from,
                        HeuristicsEdge {
                            dependency: 0.0,
                            frequency: get(&h.dfg_matrix, to, from),
                        },
                    );
                }
            }
        }
    }
    if h.nodes.is_empty() {
        for a in h.activities.clone() {
            h.node_mut(&a);
        }
    }
    Ok(h)
}

/// Discovers a heuristics Petri net using the same options and model conversion
/// as [`heuristics_net`]. The result need not be sound or bounded.
pub fn petri_net_heuristics(
    log: &EventLog,
    keys: &EventKeys,
    options: &HeuristicsOptions,
) -> Result<AcceptingPetriNet> {
    Ok(heuristics_net(log, keys, options)?.to_petri_net())
}
