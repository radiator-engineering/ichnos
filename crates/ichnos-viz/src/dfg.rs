//! Directly-follows graphs, ported from pm4py's `visualization/dfg`
//! (`util/dfg_gviz.py` and the `frequency` and `performance` variants).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ichnos_discovery::PerformanceDfg;
use ichnos_discovery::dfg::{Aggregation, BusinessHours};
use ichnos_model::dfg::ActivityCounts;
use ichnos_model::{Dfg, Label};

use crate::dot::{Dot, title_label};
use crate::petri_net::{END_SYMBOL, START_SYMBOL};
use crate::style::{
    arc_penwidth, day_seconds, frequency_color, human_readable_stat, min_max, py_float,
    service_time_color, value_to_color,
};

/// Options for [`dfg_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfgDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// The font size. Default 12.
    pub font_size: u32,
    /// Keep only this many edges, the most frequent ones. Default
    /// unlimited.
    pub max_num_edges: usize,
    /// Activity frequencies for the node labels and colours. By default
    /// each activity counts the frequencies of its incoming edges plus its
    /// start frequency.
    pub activities_count: Option<ActivityCounts>,
}

impl Default for DfgDotOptions {
    fn default() -> Self {
        DfgDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            font_size: 12,
            max_num_edges: usize::MAX,
            activities_count: None,
        }
    }
}

/// Options for [`performance_dfg_dot`], with pm4py's defaults.
#[derive(Debug, Clone)]
pub struct PerformanceDfgDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// The font size. Default 12.
    pub font_size: u32,
    /// Keep only this many edges, the slowest ones. Default unlimited.
    pub max_num_edges: usize,
    /// Which duration aggregate the edges show. Default the mean.
    pub aggregation: Aggregation,
    /// Service times in seconds. An activity with a non-negative service
    /// time shows it and is coloured by it.
    pub serv_time: Option<BTreeMap<Label, f64>>,
    /// A weekly schedule. Durations then count working days of its mean
    /// length, not calendar days. By default the DFG's own schedule.
    pub business_hours: Option<BusinessHours>,
}

impl Default for PerformanceDfgDotOptions {
    fn default() -> Self {
        PerformanceDfgDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            font_size: 12,
            max_num_edges: usize::MAX,
            aggregation: Aggregation::Mean,
            serv_time: None,
            business_hours: None,
        }
    }
}

/// pm4py's `DEFAULT_ARTIFICIAL_START_ACTIVITY` and `_END_ACTIVITY` keys,
/// kept apart from the activities here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    Start,
    End,
}

/// What `graphviz_visualization` draws, with each variant's inputs
/// resolved.
struct Input<'a> {
    edges: Vec<((Label, Label), f64)>,
    /// The activities pm4py counts, in sorted order.
    activities: BTreeSet<Label>,
    start: &'a BTreeMap<Label, u64>,
    end: &'a BTreeMap<Label, u64>,
    measure: Measure,
}

enum Measure {
    Frequency {
        counts: BTreeMap<Label, u64>,
    },
    Performance {
        serv_time: BTreeMap<Label, f64>,
        day_seconds: f64,
    },
}

/// The DOT text of a frequency DFG, as pm4py's `save_vis_dfg` draws it.
///
/// Nodes show each activity with its frequency, shaded from white (least
/// frequent) to blue. Edges show their frequency, with pen widths from 1.0
/// to 2.6. A circle leads to the start activities and the end activities
/// lead to a square.
pub fn dfg_dot(dfg: &Dfg, options: &DfgDotOptions) -> String {
    let mut activities: BTreeSet<Label> = dfg.edge_activities();
    activities.extend(dfg.start_activities.keys().cloned());
    activities.extend(dfg.end_activities.keys().cloned());
    let counts = match &options.activities_count {
        Some(counts) => counts.clone(),
        None => {
            let mut counts: BTreeMap<Label, u64> =
                activities.iter().map(|a| (a.clone(), 0)).collect();
            for ((_, b), n) in &dfg.graph {
                *counts.entry(b.clone()).or_default() += n;
            }
            for (a, n) in &dfg.start_activities {
                *counts.entry(a.clone()).or_default() += n;
            }
            counts
        }
    };
    let input = Input {
        edges: dfg
            .graph
            .iter()
            .map(|(k, &n)| (k.clone(), n as f64))
            .collect(),
        activities: counts.keys().cloned().collect(),
        start: &dfg.start_activities,
        end: &dfg.end_activities,
        measure: Measure::Frequency { counts },
    };
    draw(
        input,
        &options.bgcolor,
        &options.rankdir,
        options.graph_title.as_deref(),
        options.font_size,
        options.max_num_edges,
    )
}

/// The DOT text of a performance DFG, as pm4py's `save_vis_performance_dfg`
/// draws it.
///
/// Edges show the chosen duration aggregate, coloured from blue (fastest)
/// to red, as pm4py's `human_readable_stat` writes it: whole years (`Y`,
/// 360 days), months (`MO`, 30 days), days (`D`), hours, minutes,
/// seconds, milliseconds or nanoseconds. Pen widths scale over the
/// durations and the start and end frequencies together, as in pm4py.
pub fn performance_dfg_dot(dfg: &PerformanceDfg, options: &PerformanceDfgDotOptions) -> String {
    let edges: Vec<((Label, Label), f64)> = dfg
        .graph
        .iter()
        .map(|(k, s)| (k.clone(), s.aggregate(options.aggregation)))
        .collect();
    let edge_activities: BTreeSet<Label> = dfg
        .graph
        .keys()
        .flat_map(|(a, b)| [a.clone(), b.clone()])
        .collect();
    let mut activities = edge_activities.clone();
    activities.extend(dfg.start_activities.keys().cloned());
    let serv_time = options
        .serv_time
        .clone()
        .unwrap_or_else(|| edge_activities.iter().map(|a| (a.clone(), -1.0)).collect());
    let hours = options
        .business_hours
        .as_ref()
        .or(dfg.business_hours.as_ref());
    let input = Input {
        edges,
        activities,
        start: &dfg.start_activities,
        end: &dfg.end_activities,
        measure: Measure::Performance {
            serv_time,
            day_seconds: day_seconds(hours),
        },
    };
    draw(
        input,
        &options.bgcolor,
        &options.rankdir,
        options.graph_title.as_deref(),
        options.font_size,
        options.max_num_edges,
    )
}

/// pm4py's `dfg_gviz.graphviz_visualization`.
fn draw(
    input: Input<'_>,
    bgcolor: &str,
    rankdir: &str,
    title: Option<&str>,
    font_size: u32,
    max_num_edges: usize,
) -> String {
    let fs = font_size.to_string();
    let mut dot = Dot::new(
        true,
        false,
        "",
        vec![
            ("bgcolor", Some(bgcolor.to_owned())),
            ("rankdir", Some(rankdir.to_owned())),
        ],
    );
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, font_size))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }

    // Keep the heaviest edges; ties go to the larger activity names.
    let mut kept = input.edges;
    kept.sort_by(|(ka, va), (kb, vb)| vb.total_cmp(va).then_with(|| kb.cmp(ka)));
    kept.truncate(max_num_edges);
    let dfg: BTreeMap<(Label, Label), f64> = kept.into_iter().collect();

    let activity_color: BTreeMap<Label, String> = match &input.measure {
        Measure::Frequency { counts } => {
            let (lo, hi) = min_max(counts.values().map(|&n| n as f64));
            counts
                .iter()
                .map(|(a, &n)| (a.clone(), frequency_color(n as f64, lo, hi)))
                .collect()
        }
        Measure::Performance { serv_time, .. } => {
            let (lo, hi) = min_max(serv_time.values().copied());
            serv_time
                .iter()
                .map(|(a, &t)| (a.clone(), service_time_color(t, lo, hi)))
                .collect()
        }
    };

    dot.defaults("node", vec![("shape", Some("box".to_owned()))]);
    let mut activities: Vec<Label> = input.activities.iter().cloned().collect();
    let start: Vec<&Label> = input
        .start
        .keys()
        .filter(|a| input.activities.contains(*a))
        .collect();
    let end: Vec<&Label> = input
        .end
        .keys()
        .filter(|a| input.activities.contains(*a))
        .collect();

    let mut ext: Vec<f64> = dfg.values().copied().collect();
    ext.extend(start.iter().map(|a| input.start[*a] as f64));
    ext.extend(end.iter().map(|a| input.end[*a] as f64));
    let (pen_lo, pen_hi) = min_max(ext);
    let (dfg_lo, dfg_hi) = min_max(dfg.values().copied());
    let penwidth = |v: f64| py_float(arc_penwidth(v, pen_lo, pen_hi));

    let mut edges: Vec<(Label, Label)> = dfg.keys().cloned().collect();
    if !start.is_empty() && !end.is_empty() {
        let (acts, sorted) = sort_dfg_reachability(&edges, &start, &end);
        activities = acts;
        edges = sorted;
        // pm4py drops a start or end activity without edges here and then
        // fails on its arc; it is drawn instead.
        for a in start.iter().chain(&end) {
            if !activities.contains(a) {
                activities.push((*a).clone());
            }
        }
    }
    // pm4py leaves an edge activity missing from `activities_count`
    // undeclared, so Graphviz names it by its hash; it gets a plain node.
    for (a, b) in &edges {
        for x in [a, b] {
            if !activities.contains(x) {
                activities.push(x.clone());
            }
        }
    }

    let mut ids: BTreeMap<Label, String> = BTreeMap::new();
    for (i, act) in activities.iter().enumerate() {
        let id = format!("a{i}");
        let mut attrs = vec![("fontsize", Some(fs.clone()))];
        let label = match &input.measure {
            Measure::Frequency { counts } => counts.get(act).map(|n| {
                attrs.push(("style", Some("filled".to_owned())));
                attrs.push(("fillcolor", activity_color.get(act).cloned()));
                format!("{act} ({n})")
            }),
            Measure::Performance {
                serv_time,
                day_seconds,
            } => serv_time.get(act).filter(|&&t| t >= 0.0).map(|&t| {
                attrs.push(("style", Some("filled".to_owned())));
                attrs.push(("fillcolor", activity_color.get(act).cloned()));
                format!("{act} ({})", human_readable_stat(t, *day_seconds))
            }),
        };
        dot.node(&id, Some(label.as_deref().unwrap_or(act)), attrs);
        ids.insert(act.clone(), id);
    }

    let frequency = matches!(input.measure, Measure::Frequency { .. });
    for (a, b) in &edges {
        let v = dfg[&(a.clone(), b.clone())];
        let (label, color) = match &input.measure {
            Measure::Frequency { .. } => (format!("{}", v as u64), None),
            Measure::Performance { day_seconds, .. } => (
                human_readable_stat(v, *day_seconds),
                Some(value_to_color(v, dfg_lo, dfg_hi)),
            ),
        };
        dot.edge(
            &ids[a],
            &ids[b],
            Some(&label),
            vec![
                ("penwidth", Some(penwidth(v))),
                ("fontsize", Some(fs.clone())),
                ("color", color),
            ],
        );
    }

    for (side, acts) in [(Side::Start, &start), (Side::End, &end)] {
        if acts.is_empty() {
            continue;
        }
        let (node, symbol, shape, size, counts) = match side {
            Side::Start => ("@@startnode", START_SYMBOL, "circle", "34", input.start),
            Side::End => ("@@endnode", END_SYMBOL, "doublecircle", "32", input.end),
        };
        dot.node(
            node,
            Some(symbol),
            vec![
                ("shape", Some(shape.to_owned())),
                ("fontsize", Some(size.to_owned())),
            ],
        );
        for act in acts.iter() {
            let n = counts[*act];
            let label = if frequency {
                n.to_string()
            } else {
                String::new()
            };
            let id = &ids[*act];
            let (tail, head) = match side {
                Side::Start => (node, id.as_str()),
                Side::End => (id.as_str(), node),
            };
            dot.edge(
                tail,
                head,
                Some(&label),
                vec![
                    ("fontsize", Some(fs.clone())),
                    ("penwidth", Some(penwidth(n as f64))),
                ],
            );
        }
    }
    dot.set(vec![("overlap", Some("false".to_owned()))]);
    dot.set(vec![("fontsize", Some("11".to_owned()))]);
    dot.finish()
}

/// pm4py's `sort_dfg_reachability`: the activities on the edges and the
/// edges, in the order of their distance from the start activities.
///
/// Edges leaving a start activity come first and edges into an end
/// activity last.
fn sort_dfg_reachability(
    edges: &[(Label, Label)],
    start: &[&Label],
    end: &[&Label],
) -> (Vec<Label>, Vec<(Label, Label)>) {
    const INF: u64 = u64::MAX;
    let mut adjacency: BTreeMap<&Label, Vec<&Label>> = BTreeMap::new();
    for (u, v) in edges {
        adjacency.entry(u).or_default().push(v);
    }
    let mut distance: BTreeMap<&Label, u64> = BTreeMap::new();
    for &a in start {
        distance.insert(a, 0);
    }
    for (u, v) in edges {
        distance.entry(u).or_insert(INF);
        distance.entry(v).or_insert(INF);
    }
    let mut queue: VecDeque<&Label> = start.iter().copied().collect();
    while let Some(current) = queue.pop_front() {
        let d = distance[current];
        for &next in adjacency.get(current).into_iter().flatten() {
            if distance[next] > d + 1 {
                distance.insert(next, d + 1);
                queue.push_back(next);
            }
        }
    }
    let group = |(u, v): &(Label, Label)| {
        if start.contains(&u) {
            0
        } else if end.contains(&v) {
            2
        } else {
            1
        }
    };
    let mut sorted = edges.to_vec();
    sorted.sort_by(|x, y| {
        (group(x), distance[&x.0], distance[&x.1], &x.0, &x.1).cmp(&(
            group(y),
            distance[&y.0],
            distance[&y.1],
            &y.0,
            &y.1,
        ))
    });
    let mut activities: Vec<Label> = edges
        .iter()
        .flat_map(|(u, v)| [u.clone(), v.clone()])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    activities.sort_by(|a, b| (distance[a], a).cmp(&(distance[b], b)));
    (activities, sorted)
}
