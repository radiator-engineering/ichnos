//! Object-centric conformance checking, ported from pm4py's
//! `algo/conformance/ocel`: an object-centric event log, or a graph
//! discovered from one, is compared with a normative graph.
//!
//! - [`conformance_otg`] compares object-type graphs ([`Otg`]).
//! - [`conformance_etot`] compares event type–object type graphs
//!   ([`Etot`]).
//! - [`conformance_ocdfg`] compares the measures of object-centric
//!   directly-follows graphs ([`OcdfgMeasures`]).
//!
//! Each has an `_ocel` form that first discovers the graph from a log, as
//! pm4py does when it is given an `OCEL`.
//!
//! [`discover_otg`] and [`discover_etot`] port pm4py's
//! `algo/discovery/ocel/{otg,etot}`. They live here because
//! ichnos-discovery depends on this crate; ichnos-discovery re-exports
//! them.

mod graphs;

use std::collections::{BTreeMap, BTreeSet};

use ichnos_ocel::Ocel;

use crate::{Error, Result};

pub use graphs::{Etot, OcdfgMeasures, Otg, OtgEdge, discover_etot, discover_otg};
pub use ichnos_ocel::ObjectGraphKind;

/// Options for [`conformance_otg`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct OtgConformanceOptions {
    /// The largest relative frequency difference an edge may have, by
    /// relation. A relation without an entry uses 0.2. Default 0.2 for
    /// each relation.
    pub theta: BTreeMap<ObjectGraphKind, f64>,
    /// The weight of missing object types. Default 1.
    pub alpha: f64,
    /// The weight of missing edges. Default 1.
    pub beta: f64,
    /// The weight of edges whose frequency differs too much. Default 1.
    pub gamma: f64,
}

impl Default for OtgConformanceOptions {
    fn default() -> Self {
        OtgConformanceOptions {
            theta: ObjectGraphKind::ALL.into_iter().map(|r| (r, 0.2)).collect(),
            alpha: 1.0,
            beta: 1.0,
            gamma: 1.0,
        }
    }
}

/// The result of [`conformance_otg`].
#[derive(Debug, Clone, PartialEq)]
pub struct OtgDiagnostics {
    /// Object types of the model that the log lacks.
    pub missing_object_types: BTreeSet<String>,
    /// Object types of the log that the model lacks.
    pub additional_object_types: BTreeSet<String>,
    /// Edges of the model that the log lacks.
    pub missing_edges: BTreeSet<OtgEdge>,
    /// Edges of the log that the model lacks.
    pub additional_edges: BTreeSet<OtgEdge>,
    /// Shared edges whose relative frequency difference is above the
    /// relation's threshold, with that difference. It is infinite when the
    /// model frequency is 0.
    pub non_conforming_edges: BTreeMap<OtgEdge, f64>,
    /// 1 minus the weighted deviations over the weighted size of the model.
    pub fitness: f64,
}

/// Compares an object-type graph with a normative one, as pm4py's
/// `conformance_otg` does.
///
/// The fitness is `1 - (alpha·|missing types| + beta·|missing edges| +
/// gamma·|non-conforming edges|) / (alpha·|model types| + (beta + gamma)·
/// |model edges|)`.
///
/// # Errors
///
/// [`Error::ZeroNormalization`] when the denominator is 0, for example for
/// an empty model; pm4py raises `ZeroDivisionError`.
pub fn conformance_otg(
    real: &Otg,
    model: &Otg,
    options: &OtgConformanceOptions,
) -> Result<OtgDiagnostics> {
    let missing_object_types = difference(&model.object_types, &real.object_types);
    let additional_object_types = difference(&real.object_types, &model.object_types);
    let missing_edges: BTreeSet<OtgEdge> = model
        .edges
        .keys()
        .filter(|e| !real.edges.contains_key(*e))
        .cloned()
        .collect();
    let additional_edges: BTreeSet<OtgEdge> = real
        .edges
        .keys()
        .filter(|e| !model.edges.contains_key(*e))
        .cloned()
        .collect();
    let mut non_conforming_edges = BTreeMap::new();
    for (edge, &f_m) in &model.edges {
        let Some(&f_l) = real.edges.get(edge) else {
            continue;
        };
        let delta = if f_m == 0 {
            f64::INFINITY
        } else {
            f_l.abs_diff(f_m) as f64 / f_m as f64
        };
        let theta = options.theta.get(&edge.relation).copied().unwrap_or(0.2);
        if delta > theta {
            non_conforming_edges.insert(edge.clone(), delta);
        }
    }
    let n = options.alpha * model.object_types.len() as f64
        + (options.beta + options.gamma) * model.edges.len() as f64;
    let numerator = options.alpha * missing_object_types.len() as f64
        + options.beta * missing_edges.len() as f64
        + options.gamma * non_conforming_edges.len() as f64;
    Ok(OtgDiagnostics {
        missing_object_types,
        additional_object_types,
        missing_edges,
        additional_edges,
        non_conforming_edges,
        fitness: 1.0 - divide(numerator, n)?,
    })
}

/// [`conformance_otg`] on the object-type graph of a log.
///
/// # Errors
///
/// As [`discover_otg`] and [`conformance_otg`].
pub fn conformance_otg_ocel(
    ocel: &Ocel,
    model: &Otg,
    options: &OtgConformanceOptions,
) -> Result<OtgDiagnostics> {
    conformance_otg(&discover_otg(ocel)?, model, options)
}

/// Options for [`conformance_etot`], with pm4py's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EtotConformanceOptions {
    /// The weight of missing activities and object types. Default 1.
    pub alpha: f64,
    /// The weight of missing edges. Default 1.
    pub beta: f64,
    /// The weight of edges whose frequency differs too much. Default 1.
    pub gamma: f64,
    /// The largest relative frequency difference an edge may have
    /// (pm4py's `theta_real`). Default 0.1.
    pub theta_rel: f64,
}

impl Default for EtotConformanceOptions {
    fn default() -> Self {
        EtotConformanceOptions {
            alpha: 1.0,
            beta: 1.0,
            gamma: 1.0,
            theta_rel: 0.1,
        }
    }
}

/// The result of [`conformance_etot`] (pm4py's `details`, plus the
/// fitness).
#[derive(Debug, Clone, PartialEq)]
pub struct EtotDiagnostics {
    /// Activities of the model that the log lacks.
    pub activities_missing: BTreeSet<String>,
    /// Activities of the log that the model lacks.
    pub activities_additional: BTreeSet<String>,
    /// Object types of the model that the log lacks.
    pub object_types_missing: BTreeSet<String>,
    /// Object types of the log that the model lacks.
    pub object_types_additional: BTreeSet<String>,
    /// Edges of the model that the log lacks.
    pub edges_missing: BTreeSet<(String, String)>,
    /// Edges of the log that the model lacks.
    pub edges_additional: BTreeSet<(String, String)>,
    /// The relative frequency difference of each shared edge.
    pub delta_rel: BTreeMap<(String, String), f64>,
    /// 1 minus the weighted deviations over the weighted size of the model.
    pub fitness: f64,
}

/// Compares an event type–object type graph with a normative one, as
/// pm4py's `conformance_etot` does.
///
/// A shared edge deviates when its relative frequency difference is above
/// `theta_rel`. The fitness is `1 - (alpha·(|missing activities| +
/// |missing types|) + beta·|missing edges| + gamma·|deviating edges|) /
/// (alpha·(|model activities| + |model types|) + (beta + gamma)·|model
/// edges|)`.
///
/// # Errors
///
/// [`Error::ZeroEdgeFrequency`] when a shared edge has frequency 0 in the
/// model, and [`Error::ZeroNormalization`] when the denominator is 0. pm4py
/// raises `ZeroDivisionError` in both cases.
pub fn conformance_etot(
    real: &Etot,
    model: &Etot,
    options: &EtotConformanceOptions,
) -> Result<EtotDiagnostics> {
    let edges_missing: BTreeSet<(String, String)> = model
        .edges
        .keys()
        .filter(|e| !real.edges.contains_key(*e))
        .cloned()
        .collect();
    let edges_additional: BTreeSet<(String, String)> = real
        .edges
        .keys()
        .filter(|e| !model.edges.contains_key(*e))
        .cloned()
        .collect();
    let mut delta_rel = BTreeMap::new();
    let mut deviating = 0_usize;
    for (edge, &w_m) in &model.edges {
        let Some(&w_l) = real.edges.get(edge) else {
            continue;
        };
        if w_m == 0 {
            return Err(Error::ZeroEdgeFrequency {
                activity: edge.0.clone(),
                object_type: edge.1.clone(),
            });
        }
        let delta = w_l.abs_diff(w_m) as f64 / w_m as f64;
        if delta > options.theta_rel {
            deviating += 1;
        }
        delta_rel.insert(edge.clone(), delta);
    }
    let activities_missing = difference(&model.activities, &real.activities);
    let object_types_missing = difference(&model.object_types, &real.object_types);
    let n = options.alpha * (model.activities.len() + model.object_types.len()) as f64
        + (options.beta + options.gamma) * model.edges.len() as f64;
    let numerator = options.alpha * (activities_missing.len() + object_types_missing.len()) as f64
        + options.beta * edges_missing.len() as f64
        + options.gamma * deviating as f64;
    Ok(EtotDiagnostics {
        activities_additional: difference(&real.activities, &model.activities),
        object_types_additional: difference(&real.object_types, &model.object_types),
        activities_missing,
        object_types_missing,
        edges_missing,
        edges_additional,
        delta_rel,
        fitness: 1.0 - divide(numerator, n)?,
    })
}

/// [`conformance_etot`] on the event type–object type graph of a log.
///
/// # Errors
///
/// As [`discover_etot`] and [`conformance_etot`].
pub fn conformance_etot_ocel(
    ocel: &Ocel,
    model: &Etot,
    options: &EtotConformanceOptions,
) -> Result<EtotDiagnostics> {
    conformance_etot(&discover_etot(ocel)?, model, options)
}

/// Options for [`conformance_ocdfg`], with pm4py's defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OcdfgConformanceOptions {
    /// The largest difference in events an activity may have. Default 0.
    pub theta_act: f64,
    /// The largest difference in event couples a flow may have. Default 0.
    pub theta_flow: f64,
    /// The weight of missing activities. Default 1.
    pub alpha: f64,
    /// The weight of missing flows. Default 1.
    pub beta: f64,
    /// The weight of activities whose events differ too much. Default 1.
    pub gamma: f64,
    /// The weight of flows whose event couples differ too much. Default 1.
    pub delta: f64,
}

impl Default for OcdfgConformanceOptions {
    fn default() -> Self {
        OcdfgConformanceOptions {
            theta_act: 0.0,
            theta_flow: 0.0,
            alpha: 1.0,
            beta: 1.0,
            gamma: 1.0,
            delta: 1.0,
        }
    }
}

/// The result of [`conformance_ocdfg`].
#[derive(Debug, Clone, PartialEq)]
pub struct OcdfgDiagnostics {
    /// Activities of the model that the log lacks.
    pub missing_activities: BTreeSet<String>,
    /// Activities of the log that the model lacks.
    pub additional_activities: BTreeSet<String>,
    /// Flows of the model that the log lacks.
    pub missing_flows: BTreeSet<(String, String)>,
    /// Flows of the log that the model lacks.
    pub additional_flows: BTreeSet<(String, String)>,
    /// The difference in events of each activity of either graph.
    pub activity_measure_differences: BTreeMap<String, u64>,
    /// Activities whose difference is above `theta_act`.
    pub non_conforming_activities_in_measure: BTreeSet<String>,
    /// The difference in event couples of each flow of either graph.
    pub flow_measure_differences: BTreeMap<(String, String), u64>,
    /// Flows whose difference is above `theta_flow`.
    pub non_conforming_flows_in_measure: BTreeSet<(String, String)>,
    /// 1 minus the weighted deviations over the weighted size of both
    /// graphs, clamped to 0..=1. It is 1 when the size is 0.
    pub fitness: f64,
}

/// Compares the measures of an object-centric DFG with a normative one, as
/// pm4py's `conformance_ocdfg` does.
///
/// The fitness is `1 - (alpha·|missing activities| + beta·|missing flows| +
/// gamma·|deviating activities| + delta·|deviating flows|) / ((alpha +
/// gamma)·|activities| + (beta + delta)·|flows|)`, where the activities and
/// flows are those of either graph.
pub fn conformance_ocdfg(
    real: &OcdfgMeasures,
    model: &OcdfgMeasures,
    options: &OcdfgConformanceOptions,
) -> OcdfgDiagnostics {
    let all_activities: BTreeSet<&String> =
        real.activities.iter().chain(&model.activities).collect();
    let all_flows: BTreeSet<&(String, String)> =
        real.flows.keys().chain(model.flows.keys()).collect();
    let mut activity_measure_differences = BTreeMap::new();
    let mut non_conforming_activities_in_measure = BTreeSet::new();
    for &a in &all_activities {
        let count = |m: &OcdfgMeasures| m.events.get(a).copied().unwrap_or(0);
        let diff = count(real).abs_diff(count(model));
        if diff as f64 > options.theta_act {
            non_conforming_activities_in_measure.insert(a.clone());
        }
        activity_measure_differences.insert(a.clone(), diff);
    }
    let mut flow_measure_differences = BTreeMap::new();
    let mut non_conforming_flows_in_measure = BTreeSet::new();
    for &f in &all_flows {
        let count = |m: &OcdfgMeasures| m.flows.get(f).copied().unwrap_or(0);
        let diff = count(real).abs_diff(count(model));
        if diff as f64 > options.theta_flow {
            non_conforming_flows_in_measure.insert(f.clone());
        }
        flow_measure_differences.insert(f.clone(), diff);
    }
    let missing_activities = difference(&model.activities, &real.activities);
    let missing_flows: BTreeSet<(String, String)> = model
        .flows
        .keys()
        .filter(|f| !real.flows.contains_key(*f))
        .cloned()
        .collect();
    let n = (options.alpha + options.gamma) * all_activities.len() as f64
        + (options.beta + options.delta) * all_flows.len() as f64;
    let numerator = options.alpha * missing_activities.len() as f64
        + options.beta * missing_flows.len() as f64
        + options.gamma * non_conforming_activities_in_measure.len() as f64
        + options.delta * non_conforming_flows_in_measure.len() as f64;
    let fitness = if n == 0.0 {
        1.0
    } else {
        (1.0 - numerator / n).clamp(0.0, 1.0)
    };
    OcdfgDiagnostics {
        additional_activities: difference(&real.activities, &model.activities),
        additional_flows: real
            .flows
            .keys()
            .filter(|f| !model.flows.contains_key(*f))
            .cloned()
            .collect(),
        missing_activities,
        missing_flows,
        activity_measure_differences,
        non_conforming_activities_in_measure,
        flow_measure_differences,
        non_conforming_flows_in_measure,
        fitness,
    }
}

/// [`conformance_ocdfg`] on the measures of a log's object-centric DFG.
///
/// # Errors
///
/// As [`OcdfgMeasures::from_ocel`].
pub fn conformance_ocdfg_ocel(
    ocel: &Ocel,
    model: &OcdfgMeasures,
    options: &OcdfgConformanceOptions,
) -> Result<OcdfgDiagnostics> {
    Ok(conformance_ocdfg(
        &OcdfgMeasures::from_ocel(ocel)?,
        model,
        options,
    ))
}

fn difference(a: &BTreeSet<String>, b: &BTreeSet<String>) -> BTreeSet<String> {
    a.difference(b).cloned().collect()
}

/// `numerator / n`, failing where Python raises `ZeroDivisionError`.
fn divide(numerator: f64, n: f64) -> Result<f64> {
    if n == 0.0 {
        Err(Error::ZeroNormalization)
    } else {
        Ok(numerator / n)
    }
}

#[cfg(test)]
mod tests;
