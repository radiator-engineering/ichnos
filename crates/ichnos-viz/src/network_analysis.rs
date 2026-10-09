//! Network analysis graphs, ported from pm4py's
//! `visualization/network_analysis/variants/frequency.py` and
//! `performance.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_discovery::dfg::{Aggregation, BusinessHours};

use crate::dot::{Dot, title_label};
use crate::ocdfg::aggregate;
use crate::style::{day_seconds, human_readable_stat, py_float};

/// One edge of a network analysis: the values that link its two nodes,
/// each with its measure (pm4py's inner dictionary, in its order).
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkAnalysisEdge<T> {
    /// The source node.
    pub source: String,
    /// The target node.
    pub target: String,
    /// The measure of each value: a count for the frequency view, the
    /// durations in seconds for the performance view.
    pub values: Vec<(String, T)>,
}

/// Options for [`network_analysis_dot`] and
/// [`network_analysis_performance_dot`], with the defaults of pm4py's
/// `save_vis_network_analysis`.
#[derive(Debug, Clone)]
pub struct NetworkAnalysisDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// Draw only nodes whose larger degree is at least this. Default 1.
    pub activity_threshold: u64,
    /// Draw only values that occur at least this often, and count only
    /// those in the degrees. Default 1.
    pub edge_threshold: u64,
    /// How the performance view aggregates durations. Default the mean.
    /// pm4py's `save_vis_network_analysis` always uses the mean.
    pub aggregation: Aggregation,
    /// A weekly schedule. Durations then count working days of its mean
    /// length, not calendar days.
    pub business_hours: Option<BusinessHours>,
}

impl Default for NetworkAnalysisDotOptions {
    fn default() -> Self {
        NetworkAnalysisDotOptions {
            bgcolor: "white".to_owned(),
            graph_title: None,
            activity_threshold: 1,
            edge_threshold: 1,
            aggregation: Aggregation::Mean,
            business_hours: None,
        }
    }
}

/// The DOT text of a network analysis with counts, as pm4py's
/// `save_vis_network_analysis` draws it with the `frequency` variant.
///
/// Each node shows its in- and out-degree: the sums of the counts that
/// reach the edge threshold. Each value of an edge is one arc, labelled
/// with the value and its count, its pen width scaled by the count.
///
/// Two pm4py quirks are kept. Every node gets the same fill colour, since
/// pm4py shades nodes against a range it has not yet computed. And the
/// pen-width range comes from a running minimum and maximum that never
/// updates both on one value, so it depends on the order of `edges`.
pub fn network_analysis_dot(
    edges: &[NetworkAnalysisEdge<u64>],
    options: &NetworkAnalysisDotOptions,
) -> String {
    draw(edges, options, |c, _| c.to_string())
}

/// The DOT text of a network analysis with durations, as pm4py's
/// `save_vis_network_analysis` draws it with the `performance` variant.
///
/// The graph is that of [`network_analysis_dot`] with each value's number
/// of durations as its count, but arcs show the aggregated duration
/// instead of the count.
pub fn network_analysis_performance_dot(
    edges: &[NetworkAnalysisEdge<Vec<f64>>],
    options: &NetworkAnalysisDotOptions,
) -> String {
    let counts: Vec<NetworkAnalysisEdge<u64>> = edges
        .iter()
        .map(|e| NetworkAnalysisEdge {
            source: e.source.clone(),
            target: e.target.clone(),
            values: e
                .values
                .iter()
                .map(|(k, v)| (k.clone(), v.len() as u64))
                .collect(),
        })
        .collect();
    let day = day_seconds(options.business_hours.as_ref());
    let labels: Vec<String> = edges
        .iter()
        .flat_map(|e| &e.values)
        .map(|(_, v)| human_readable_stat(aggregate(v, options.aggregation), day))
        .collect();
    draw(&counts, options, |_, i| labels[i].clone())
}

/// Python's `sys.maxsize`.
const MAXSIZE: i128 = i64::MAX as i128;

/// Draws the graph. `label` gives the text under a value's name from its
/// count and its position among all values of `edges`.
fn draw(
    edges: &[NetworkAnalysisEdge<u64>],
    options: &NetworkAnalysisDotOptions,
    label: impl Fn(u64, usize) -> String,
) -> String {
    let s = |v: &str| Some(v.to_owned());
    let mut dot = Dot::new(true, false, "pt", vec![("bgcolor", s(&options.bgcolor))]);
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
    let nodes: BTreeSet<&str> = edges
        .iter()
        .flat_map(|e| [e.source.as_str(), e.target.as_str()])
        .collect();
    let mut in_degree: BTreeMap<&str, u64> = nodes.iter().map(|&n| (n, 0)).collect();
    let mut out_degree = in_degree.clone();
    for e in edges {
        for &(_, c) in &e.values {
            if c >= options.edge_threshold {
                *in_degree.get_mut(e.target.as_str()).expect("a node") += c;
                *out_degree.get_mut(e.source.as_str()).expect("a node") += c;
            }
        }
    }
    let mut ids: BTreeMap<&str, String> = BTreeMap::new();
    for &n in &nodes {
        let (i, o) = (in_degree[n], out_degree[n]);
        let degree = i.max(o);
        if degree < options.activity_threshold {
            continue;
        }
        let id = format!("n{}", ids.len());
        dot.node(
            &id,
            Some(&format!("{n}\n(in={i}; out={o})")),
            vec![
                ("style", s("filled")),
                ("fillcolor", Some(node_color(degree))),
            ],
        );
        ids.insert(n, id);
    }
    let drawn = |e: &NetworkAnalysisEdge<u64>| {
        ids.contains_key(e.source.as_str()) && ids.contains_key(e.target.as_str())
    };
    let (mut lo, mut hi) = (MAXSIZE, -MAXSIZE);
    for e in edges.iter().filter(|e| drawn(e)) {
        for &(_, c) in &e.values {
            let c = i128::from(c);
            if c > hi {
                hi = c;
            } else if c < lo {
                lo = c;
            }
        }
    }
    let mut position = 0;
    for e in edges {
        let keep = drawn(e);
        for (name, c) in &e.values {
            let i = position;
            position += 1;
            if !keep || *c < options.edge_threshold {
                continue;
            }
            let text = label(*c, i);
            dot.edge(
                &ids[e.source.as_str()],
                &ids[e.target.as_str()],
                Some(&format!("{name}\n{text}")),
                vec![("penwidth", Some(py_float(penwidth(i128::from(*c), lo, hi))))],
            );
        }
    }
    dot.finish()
}

/// pm4py's `get_arc_penwidth` with Python integers, whose range here can
/// reach `sys.maxsize`.
fn penwidth(value: i128, min: i128, max: i128) -> f64 {
    1.0 + 1.6 * (value - min) as f64 / ((max - min) as f64 + 0.00001)
}

/// pm4py's `get_trans_freq_color(count, sys.maxsize, sys.maxsize)`, which
/// gives one very large shade written in hex.
fn node_color(count: u64) -> String {
    let scaled = (100 * (i128::from(count) - MAXSIZE)) as f64 / 0.00001;
    let shade = (255.0 - scaled).trunc() as u128;
    format!("#{shade:X}{shade:X}FF")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(s: &str, t: &str, values: &[(&str, u64)]) -> NetworkAnalysisEdge<u64> {
        NetworkAnalysisEdge {
            source: s.to_owned(),
            target: t.to_owned(),
            values: values.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
        }
    }

    #[test]
    fn node_colours_follow_python_integers() {
        // int(255 - 100 * (3 - sys.maxsize) / 0.00001) in Python.
        assert_eq!(
            node_color(3),
            "#4C4B4000000000000000004C4B400000000000000000FF"
        );
        assert_eq!(node_color(10), node_color(3));
    }

    #[test]
    fn pen_widths_use_the_order_dependent_range() {
        // Rising counts set only the maximum: the minimum stays sys.maxsize
        // and both arcs get the widest pen.
        let rising = network_analysis_dot(
            &[edge("a", "b", &[("x", 1)]), edge("b", "c", &[("y", 2)])],
            &NetworkAnalysisDotOptions::default(),
        );
        assert_eq!(rising.matches("penwidth=2.6]").count(), 2, "{rising}");
        let falling = network_analysis_dot(
            &[edge("a", "b", &[("x", 2)]), edge("b", "c", &[("y", 1)])],
            &NetworkAnalysisDotOptions::default(),
        );
        assert!(
            falling.contains("penwidth=2.5999840001599983]"),
            "{falling}"
        );
        assert!(falling.contains("penwidth=1.0]"), "{falling}");
    }
}
