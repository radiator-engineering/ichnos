//! Object graphs, ported from pm4py's
//! `visualization/ocel/object_graph/variants/graphviz.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_ocel::Ocel;

use crate::dot::{Dot, title_label};
use crate::ocdfg::color_of;

/// Options for [`object_graph_dot`], with the defaults of pm4py's
/// `save_vis_object_graph`.
#[derive(Debug, Clone)]
pub struct ObjectGraphDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// Draw arrows. Default `true`.
    pub directed: bool,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// Colours by object type. Other types get
    /// [`object_type_color`](crate::object_type_color).
    pub object_type_colors: BTreeMap<String, String>,
}

impl Default for ObjectGraphDotOptions {
    fn default() -> Self {
        ObjectGraphDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            directed: true,
            graph_title: None,
            object_type_colors: BTreeMap::new(),
        }
    }
}

/// The DOT text of a graph between objects, such as an object interaction
/// graph, as pm4py's `save_vis_object_graph` draws it.
///
/// Each object in a pair is a node labelled with its id, outlined and
/// written in the colour of its type: the type of the first object of
/// `ocel` with that id. pm4py fails on an id that `ocel` does not hold; this
/// draws it without a colour.
pub fn object_graph_dot(
    ocel: &Ocel,
    graph: &BTreeSet<(String, String)>,
    options: &ObjectGraphDotOptions,
) -> String {
    let s = |v: &str| Some(v.to_owned());
    let mut dot = Dot::new(
        options.directed,
        false,
        "ograph",
        vec![("bgcolor", s(&options.bgcolor))],
    );
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
    let mut types: BTreeMap<&str, &str> = BTreeMap::new();
    for o in &ocel.objects {
        types.entry(&o.id).or_insert(&o.object_type);
    }
    let nodes: BTreeSet<&str> = graph
        .iter()
        .flat_map(|(a, b)| [a.as_str(), b.as_str()])
        .collect();
    let mut ids: BTreeMap<&str, String> = BTreeMap::new();
    for n in nodes {
        let id = format!("n{}", ids.len());
        let color = types
            .get(n)
            .map(|ot| color_of(&options.object_type_colors, ot));
        dot.node(
            &id,
            Some(n),
            vec![("fontcolor", color.clone()), ("color", color)],
        );
        ids.insert(n, id);
    }
    for (a, b) in graph {
        dot.edge(&ids[a.as_str()], &ids[b.as_str()], None, vec![]);
    }
    dot.set(vec![("rankdir", s(&options.rankdir))]);
    dot.finish()
}
