//! Object-centric directly-follows graphs, ported from pm4py's
//! `visualization/ocel/ocdfg/variants/classic.py`.

use std::collections::BTreeMap;

use ichnos_discovery::dfg::{Aggregation, BusinessHours};

use crate::dot::{Dot, title_label};
use crate::process_tree::md5;
use crate::style::{arc_penwidth, day_seconds, frequency_color, human_readable_stat, py_float};

/// The three counts pm4py keeps for an activity or a start or end activity:
/// how many events, how many distinct objects, and how many objects in all
/// (an object counts once per event).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OcdfgCounts {
    /// Events.
    pub events: u64,
    /// Distinct objects.
    pub unique_objects: u64,
    /// Objects, counted once per event.
    pub total_objects: u64,
}

/// One edge of an object type: its counts and the durations behind them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcdfgEdge {
    /// Pairs of directly-following events.
    pub event_couples: u64,
    /// Distinct objects that follow the edge.
    pub unique_objects: u64,
    /// Objects, counted once per event pair.
    pub total_objects: u64,
    /// The durations of the event couples, in seconds.
    pub event_couples_durations: Vec<f64>,
    /// The durations per object, in seconds.
    pub total_objects_durations: Vec<f64>,
}

/// An object-centric DFG: the counts of pm4py's `discover_ocdfg`
/// dictionary that its drawing reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ocdfg {
    /// Activity counts over all object types (pm4py's `activities_indep`).
    pub activities: BTreeMap<String, OcdfgCounts>,
    /// Start activities by object type, then activity.
    pub start_activities: BTreeMap<String, BTreeMap<String, OcdfgCounts>>,
    /// End activities by object type, then activity.
    pub end_activities: BTreeMap<String, BTreeMap<String, OcdfgCounts>>,
    /// Edges by object type, then (source, target) activity.
    pub edges: BTreeMap<String, BTreeMap<(String, String), OcdfgEdge>>,
}

/// What an activity count counts (pm4py's `act_metric`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OcdfgActivityMetric {
    /// Events (`E=`). The default.
    #[default]
    Events,
    /// Distinct objects (`UO=`).
    UniqueObjects,
    /// Objects, counted once per event (`TO=`).
    TotalObjects,
}

/// What an edge count counts (pm4py's `edge_metric`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OcdfgEdgeMetric {
    /// Pairs of events (`EC=`). The default.
    #[default]
    EventCouples,
    /// Distinct objects (`UO=`). It has no durations, so the performance
    /// annotation draws no edges with it.
    UniqueObjects,
    /// Objects, counted once per event pair (`TO=`).
    TotalObjects,
}

/// What the graph shows (pm4py's `annotation`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OcdfgAnnotation {
    /// Counts on activities and edges. The default.
    #[default]
    Frequency,
    /// Aggregated durations on edges.
    Performance,
}

/// Options for [`ocdfg_dot`], with pm4py's defaults.
#[derive(Debug, Clone)]
pub struct OcdfgDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// Frequency or performance.
    pub annotation: OcdfgAnnotation,
    /// What activity counts count.
    pub act_metric: OcdfgActivityMetric,
    /// What edge counts count.
    pub edge_metric: OcdfgEdgeMetric,
    /// Draw only activities with at least this count. Default 0.
    pub act_threshold: u64,
    /// Draw only edges with at least this count. Default 0.
    pub edge_threshold: u64,
    /// How the performance annotation aggregates durations. Default the
    /// mean. pm4py has no standard deviation here and falls back to the
    /// mean, as this does.
    pub aggregation: Aggregation,
    /// A weekly schedule. Durations then count working days of its mean
    /// length, not calendar days.
    pub business_hours: Option<BusinessHours>,
    /// Colours by object type. Other types get [`object_type_color`].
    pub object_type_colors: BTreeMap<String, String>,
}

impl Default for OcdfgDotOptions {
    fn default() -> Self {
        OcdfgDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            annotation: OcdfgAnnotation::Frequency,
            act_metric: OcdfgActivityMetric::Events,
            edge_metric: OcdfgEdgeMetric::EventCouples,
            act_threshold: 0,
            edge_threshold: 0,
            aggregation: Aggregation::Mean,
            business_hours: None,
            object_type_colors: BTreeMap::new(),
        }
    }
}

/// The colour of an object type when the options give none: the first
/// three bytes of the MD5 digest of its name, as `#RRGGBB`.
///
/// pm4py derives the colour from Python's string hash, which changes from
/// one process to the next.
pub fn object_type_color(object_type: &str) -> String {
    let d = md5(object_type.as_bytes());
    format!("#{:02X}{:02X}{:02X}", d[0], d[1], d[2])
}

/// The colour `colors` gives `object_type`, else [`object_type_color`].
pub(crate) fn color_of(colors: &BTreeMap<String, String>, object_type: &str) -> String {
    colors
        .get(object_type)
        .cloned()
        .unwrap_or_else(|| object_type_color(object_type))
}

/// The DOT text of an object-centric DFG, as pm4py's `save_vis_ocdfg` draws
/// it with the `classic` variant.
///
/// Activities are boxes. With the frequency annotation they show their
/// count and are shaded from white to blue; edges show the object type and
/// their count, with pen widths scaled per object type. With the
/// performance annotation, edges show the aggregated duration instead.
/// Each object type has a colour, an ellipse that leads to its start
/// activities, and an underlined name its end activities lead to. Start and
/// end edges show the activity metric's count with the edge metric's
/// prefix, as in pm4py.
pub fn ocdfg_dot(ocdfg: &Ocdfg, options: &OcdfgDotOptions) -> String {
    let s = |v: &str| Some(v.to_owned());
    let mut dot = Dot::new(true, false, "ocdfg", vec![("bgcolor", s(&options.bgcolor))]);
    dot.defaults(
        "node",
        vec![("shape", s("ellipse")), ("fixedsize", s("false"))],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, 10))),
            ("labelloc", s("top")),
        ]);
    }
    let act = |c: &OcdfgCounts| match options.act_metric {
        OcdfgActivityMetric::Events => c.events,
        OcdfgActivityMetric::UniqueObjects => c.unique_objects,
        OcdfgActivityMetric::TotalObjects => c.total_objects,
    };
    let act_prefix = match options.act_metric {
        OcdfgActivityMetric::Events => "E=",
        OcdfgActivityMetric::UniqueObjects => "UO=",
        OcdfgActivityMetric::TotalObjects => "TO=",
    };
    let edge_count = |e: &OcdfgEdge| match options.edge_metric {
        OcdfgEdgeMetric::EventCouples => e.event_couples,
        OcdfgEdgeMetric::UniqueObjects => e.unique_objects,
        OcdfgEdgeMetric::TotalObjects => e.total_objects,
    };
    let edge_prefix = match options.edge_metric {
        OcdfgEdgeMetric::EventCouples => "EC=",
        OcdfgEdgeMetric::UniqueObjects => "UO=",
        OcdfgEdgeMetric::TotalObjects => "TO=",
    };
    let frequency = options.annotation == OcdfgAnnotation::Frequency;
    let day = day_seconds(options.business_hours.as_ref());

    // The pen-width range of each object type with edges: over its edges
    // and its start and end activities.
    let mut range: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
    for (ot, edges) in &ocdfg.edges {
        let counts = edges
            .values()
            .map(edge_count)
            .chain(
                ocdfg
                    .start_activities
                    .get(ot)
                    .into_iter()
                    .flatten()
                    .map(|(_, c)| act(c)),
            )
            .chain(
                ocdfg
                    .end_activities
                    .get(ot)
                    .into_iter()
                    .flatten()
                    .map(|(_, c)| act(c)),
            );
        if let Some(r) = counts.fold(None, |r: Option<(u64, u64)>, c| {
            Some(r.map_or((c, c), |(lo, hi)| (lo.min(c), hi.max(c))))
        }) {
            range.insert(ot, r);
        }
    }

    let (min_act, max_act) = ocdfg
        .activities
        .values()
        .map(act)
        .fold((u64::MAX, 0), |(lo, hi), c| (lo.min(c), hi.max(c)));
    let mut nodes: BTreeMap<&str, String> = BTreeMap::new();
    for (name, counts) in &ocdfg.activities {
        let count = act(counts);
        if count < options.act_threshold {
            continue;
        }
        let id = format!("a{}", nodes.len());
        if frequency {
            dot.node(
                &id,
                Some(&format!("{name}\n{act_prefix}{count}")),
                vec![
                    ("shape", s("box")),
                    ("style", s("filled")),
                    (
                        "fillcolor",
                        Some(frequency_color(
                            count as f64,
                            min_act as f64,
                            max_act as f64,
                        )),
                    ),
                ],
            );
        } else {
            dot.node(&id, Some(name), vec![("shape", s("box"))]);
        }
        nodes.insert(name, id);
    }

    for (ot, edges) in &ocdfg.edges {
        let color = color_of(&options.object_type_colors, ot);
        for ((a, b), edge) in edges {
            let (Some(tail), Some(head)) = (nodes.get(a.as_str()), nodes.get(b.as_str())) else {
                continue;
            };
            let count = edge_count(edge);
            if count < options.edge_threshold {
                continue;
            }
            if frequency {
                let (lo, hi) = range[ot.as_str()];
                dot.edge(
                    tail,
                    head,
                    Some(&format!("{ot} {edge_prefix}{count}")),
                    vec![
                        ("fontsize", s("8")),
                        (
                            "penwidth",
                            Some(py_float(arc_penwidth(count as f64, lo as f64, hi as f64))),
                        ),
                        ("color", Some(color.clone())),
                        ("fontcolor", Some(color.clone())),
                    ],
                );
            } else {
                let durations = match options.edge_metric {
                    OcdfgEdgeMetric::EventCouples => &edge.event_couples_durations,
                    OcdfgEdgeMetric::TotalObjects => &edge.total_objects_durations,
                    OcdfgEdgeMetric::UniqueObjects => continue,
                };
                // pm4py has no standard deviation here and takes the mean.
                let how = match options.aggregation {
                    Aggregation::StandardDeviation => Aggregation::Mean,
                    how => how,
                };
                let value = aggregate(durations, how);
                dot.edge(
                    tail,
                    head,
                    Some(&format!(
                        "{ot} {edge_prefix}{}",
                        human_readable_stat(value, day)
                    )),
                    vec![
                        ("fontsize", s("8")),
                        ("color", Some(color.clone())),
                        ("fontcolor", Some(color.clone())),
                    ],
                );
            }
        }
    }

    for (side, map) in [
        ("start", &ocdfg.start_activities),
        ("end", &ocdfg.end_activities),
    ] {
        let mut endpoints: BTreeMap<&str, String> = BTreeMap::new();
        for (ot, activities) in map {
            let color = color_of(&options.object_type_colors, ot);
            for (name, counts) in activities {
                let Some(activity) = nodes.get(name.as_str()) else {
                    continue;
                };
                let count = act(counts);
                if count < options.edge_threshold {
                    continue;
                }
                let endpoint = endpoints.entry(ot).or_insert_with(|| {
                    let id = format!("{side}{}", ot_index(map, ot));
                    if side == "start" {
                        dot.node(
                            &id,
                            Some(ot),
                            vec![
                                ("shape", s("ellipse")),
                                ("style", s("filled")),
                                ("fillcolor", Some(color.clone())),
                            ],
                        );
                    } else {
                        dot.node(
                            &id,
                            Some(ot),
                            vec![
                                ("shape", s("underline")),
                                ("fontcolor", Some(color.clone())),
                            ],
                        );
                    }
                    id
                });
                let (lo, hi) = range.get(ot.as_str()).copied().unwrap_or((count, count));
                let label = if frequency {
                    format!("{ot} {edge_prefix}{count}")
                } else {
                    String::new()
                };
                let (tail, head) = if side == "start" {
                    (endpoint.as_str(), activity.as_str())
                } else {
                    (activity.as_str(), endpoint.as_str())
                };
                dot.edge(
                    tail,
                    head,
                    Some(&label),
                    vec![
                        ("fontsize", s("8")),
                        (
                            "penwidth",
                            Some(py_float(arc_penwidth(count as f64, lo as f64, hi as f64))),
                        ),
                        ("fontcolor", Some(color.clone())),
                        ("color", Some(color.clone())),
                    ],
                );
            }
        }
    }
    dot.set(vec![("rankdir", s(&options.rankdir))]);
    dot.finish()
}

/// The position of `ot` among the object types of `map`, for a stable
/// node name.
fn ot_index<V>(map: &BTreeMap<String, V>, ot: &str) -> usize {
    map.keys().position(|k| k == ot).unwrap_or(0)
}

/// pm4py's aggregate of durations with Python's `statistics` module: the
/// sample standard deviation for [`Aggregation::StandardDeviation`], 0 for
/// it with fewer than two values. An empty list gives 0.
pub(crate) fn aggregate(values: &[f64], how: Aggregation) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    match how {
        Aggregation::Median => {
            let mut sorted = values.to_vec();
            sorted.sort_by(f64::total_cmp);
            let n = sorted.len();
            if n % 2 == 1 {
                sorted[n / 2]
            } else {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            }
        }
        Aggregation::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        Aggregation::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        Aggregation::Sum => exact_sum(values),
        Aggregation::StandardDeviation => {
            if values.len() < 2 {
                return 0.0;
            }
            let m = mean(values);
            let squares: Vec<f64> = values.iter().map(|v| (v - m) * (v - m)).collect();
            (exact_sum(&squares) / (values.len() - 1) as f64).sqrt()
        }
        _ => mean(values),
    }
}

/// Python's `statistics.mean`, rounded once from the exact sum.
pub(crate) fn mean(values: &[f64]) -> f64 {
    exact_sum(values) / values.len() as f64
}

/// The sum of `values`, rounded once (Shewchuk's algorithm, as Python's
/// `math.fsum`).
pub(crate) fn exact_sum(values: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &v in values {
        let mut x = v;
        let mut kept = 0;
        for i in 0..partials.len() {
            let mut y = partials[i];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let lo = y - (hi - x);
            if lo != 0.0 {
                partials[kept] = lo;
                kept += 1;
            }
            x = hi;
        }
        partials.truncate(kept);
        partials.push(x);
    }
    partials.iter().rev().fold(0.0, |acc, p| acc + p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_colours_are_stable_hex() {
        // md5("order") starts 70a17ffa.
        assert_eq!(object_type_color("order"), "#70A17F");
    }

    #[test]
    fn sums_are_exact() {
        assert_eq!(exact_sum(&[0.1; 10]), 1.0);
        assert_eq!(mean(&[1e16, 1.0, -1e16]), 1.0 / 3.0);
        assert_eq!(aggregate(&[3.0, 1.0, 2.0, 10.0], Aggregation::Median), 2.5);
        assert_eq!(aggregate(&[], Aggregation::Mean), 0.0);
    }
}
