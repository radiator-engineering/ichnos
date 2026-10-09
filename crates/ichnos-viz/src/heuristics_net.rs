//! Heuristics nets, ported from pm4py's
//! `visualization/heuristics_net/variants/pydotplus_vis.py`.

use std::collections::BTreeMap;

use ichnos_model::{HeuristicsNet, Label};

use crate::dot::Dot;
use crate::style::py_float;

/// Options for [`heuristics_net_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeuristicsNetDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// Start and end arcs need at least this frequency (pm4py's
    /// `min_dfg_occurrences`, which discovery stores on the net). Default 1,
    /// as in discovery.
    pub min_dfg_occurrences: u64,
}

impl Default for HeuristicsNetDotOptions {
    fn default() -> Self {
        HeuristicsNetDotOptions {
            bgcolor: "white".to_owned(),
            graph_title: None,
            min_dfg_occurrences: 1,
        }
    }
}

/// pm4py's edge colour, `default_edges_color`.
const EDGE_COLOR: &str = "#000000";

/// The DOT text of a heuristics net, as pm4py's `save_vis_heuristics_net`
/// draws it.
///
/// Only activities with a connection are drawn: grey boxes, darker for
/// more frequent activities, labelled with the activity and its frequency.
/// Edges show their frequency, with pen widths growing with its logarithm.
/// A green `@@S` node leads to the start activities and the end activities
/// lead to an orange `@@E` node, one pair per merged net. The graph is
/// strict, so Graphviz draws parallel edges once.
pub fn heuristics_net_dot(net: &HeuristicsNet, options: &HeuristicsNetDotOptions) -> String {
    let mut dot = Dot::new(true, true, "", vec![]);
    dot.set(vec![("bgcolor", Some(options.bgcolor.clone()))]);
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title.to_owned())),
            ("labelloc", Some("top".to_owned())),
            ("labeljust", Some("center".to_owned())),
            ("fontsize", Some("20".to_owned())),
        ]);
    }
    let s = |v: &str| Some(v.to_owned());

    // pm4py also keeps start and end activities without connections, but
    // looks them up before it has drawn anything, so none are kept.
    let mut ids: BTreeMap<&Label, String> = BTreeMap::new();
    for (name, node) in &net.nodes {
        if node.inputs.is_empty() && node.outputs.is_empty() {
            continue;
        }
        let gray = gray_color((255.0 - (node.occurrences as f64).ln() * 9.0).max(0.0));
        let id = format!("n{}", ids.len());
        dot.node(
            &id,
            Some(&format!("{name} ({})", node.occurrences)),
            vec![
                ("shape", s("box")),
                ("style", s("filled")),
                ("fillcolor", Some(gray)),
                ("fontcolor", s("#000000")),
            ],
        );
        ids.insert(name, id);
    }
    let penwidth = |n: u64| py_float(1.0 + (1.0 + n as f64).ln() / 11.0);

    for (name, node) in &net.nodes {
        let Some(tail) = ids.get(name) else { continue };
        for (other, edges) in &node.outputs {
            let Some(head) = ids.get(other) else { continue };
            for edge in edges {
                dot.edge(
                    tail,
                    head,
                    Some(&edge.frequency.to_string()),
                    vec![
                        ("color", s(EDGE_COLOR)),
                        ("fontcolor", s(EDGE_COLOR)),
                        ("penwidth", Some(penwidth(edge.frequency))),
                    ],
                );
            }
        }
    }

    for (index, starts) in net.start_activities.iter().enumerate() {
        let drawn: Vec<(&String, u64)> = starts
            .iter()
            .filter_map(|(a, &n)| ids.get(a).map(|id| (id, n)))
            .collect();
        if drawn.is_empty() {
            continue;
        }
        let node = format!("start_{index}");
        dot.node(
            &node,
            Some("@@S"),
            vec![
                ("color", s(EDGE_COLOR)),
                ("fontsize", s("8")),
                ("fontcolor", s("#32CD32")),
                ("fillcolor", s("#32CD32")),
                ("style", s("filled")),
            ],
        );
        for (id, n) in drawn {
            if n >= options.min_dfg_occurrences {
                dot.edge(
                    &node,
                    id,
                    Some(&n.to_string()),
                    vec![
                        ("color", s(EDGE_COLOR)),
                        ("fontcolor", s(EDGE_COLOR)),
                        ("penwidth", Some(penwidth(n))),
                    ],
                );
            }
        }
    }
    for (index, ends) in net.end_activities.iter().enumerate() {
        let drawn: Vec<(&String, u64)> = ends
            .iter()
            .filter_map(|(a, &n)| ids.get(a).map(|id| (id, n)))
            .collect();
        if drawn.is_empty() {
            continue;
        }
        let node = format!("end_{index}");
        dot.node(
            &node,
            Some("@@E"),
            vec![
                ("color", s("#FFA500")),
                ("fillcolor", s("#FFA500")),
                ("fontcolor", s("#FFA500")),
                ("fontsize", s("8")),
                ("style", s("filled")),
            ],
        );
        for (id, n) in drawn {
            if n >= options.min_dfg_occurrences {
                dot.edge(
                    id,
                    &node,
                    Some(&n.to_string()),
                    vec![
                        ("color", s(EDGE_COLOR)),
                        ("fontcolor", s(EDGE_COLOR)),
                        ("penwidth", Some(penwidth(n))),
                    ],
                );
            }
        }
    }
    dot.finish()
}

/// pm4py's `get_corr_hex`: one hex digit of a float, truncated.
fn hex_digit(x: f64) -> char {
    char::from_digit(x.trunc().clamp(0.0, 15.0) as u32, 16)
        .expect("a digit below 16")
        .to_ascii_uppercase()
}

/// pm4py's `transform_to_hex_2`: a grey from a float in 0 to 255.
fn gray_color(value: f64) -> String {
    let dark = 255.0 - value;
    let light = 255.0 - dark;
    let pair = |x: f64| format!("{}{}", hex_digit(x / 16.0), hex_digit(x % 16.0));
    format!("#{}{}{}", pair(dark), pair(light), pair(light))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gray_matches_pm4py() {
        // pm4py: transform_to_hex_2(max(255 - log(6) * 9, 0)).
        let v = 255.0 - 6.0_f64.ln() * 9.0;
        assert_eq!(gray_color(v), "#10EEEE");
        assert_eq!(gray_color(255.0), "#00FFFF");
    }
}
