//! Transition systems, ported from pm4py's
//! `visualization/transition_system/util/visualize_graphviz.py` (the
//! `view_based` variant).

use ichnos_model::TransitionSystem;

use crate::dot::{Dot, title_label};

/// Options for [`transition_system_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionSystemDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// The font size. Default 11.
    pub font_size: u32,
    /// Label edges with their names. Default `true`.
    pub show_labels: bool,
    /// Label states with their names. Default `true`.
    pub show_names: bool,
}

impl Default for TransitionSystemDotOptions {
    fn default() -> Self {
        TransitionSystemDotOptions {
            bgcolor: "white".to_owned(),
            graph_title: None,
            font_size: 11,
            show_labels: true,
            show_names: true,
        }
    }
}

/// The DOT text of a transition system, as pm4py's
/// `save_vis_transition_system` draws it.
///
/// States are ellipses labelled with their names, and edges carry theirs.
pub fn transition_system_dot(
    ts: &TransitionSystem,
    options: &TransitionSystemDotOptions,
) -> String {
    let font_size = options.font_size.to_string();
    let mut dot = Dot::new(
        true,
        false,
        &ts.name,
        vec![("bgcolor", Some(options.bgcolor.clone()))],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, options.font_size))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }
    let id = |index: usize| format!("s{index}");
    for (s, state) in ts.states() {
        let label = if options.show_names {
            state.name.as_str()
        } else {
            ""
        };
        dot.node(
            &id(s.index()),
            Some(label),
            vec![("fontsize", Some(font_size.clone()))],
        );
    }
    for (_, edge) in ts.edges() {
        let (tail, head) = (id(edge.from().index()), id(edge.to().index()));
        if options.show_labels {
            dot.edge(
                &tail,
                &head,
                Some(&edge.name),
                vec![("fontsize", Some(font_size.clone()))],
            );
        } else {
            dot.edge(&tail, &head, None, vec![]);
        }
    }
    dot.set(vec![("overlap", Some("false".to_owned()))]);
    dot.finish()
}
