//! Prefix trees, ported from pm4py's `visualization/trie/variants/classic.py`.

use ichnos_discovery::prefix_tree::PrefixTree;

use crate::dot::{Dot, title_label};

/// Options for [`prefix_tree_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixTreeDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
}

impl Default for PrefixTreeDotOptions {
    fn default() -> Self {
        PrefixTreeDotOptions {
            bgcolor: "white".to_owned(),
            graph_title: None,
        }
    }
}

/// The DOT text of a prefix tree, as pm4py's `save_vis_prefix_tree` draws
/// it.
///
/// The graph is undirected. Each activity is a box linked to the activity
/// before it; the root, which has no activity, is not drawn.
pub fn prefix_tree_dot(tree: &PrefixTree, options: &PrefixTreeDotOptions) -> String {
    let mut dot = Dot::new(
        false,
        false,
        "pt",
        vec![("bgcolor", Some(options.bgcolor.clone()))],
    );
    dot.defaults(
        "node",
        vec![
            ("shape", Some("ellipse".to_owned())),
            ("fixedsize", Some("false".to_owned())),
        ],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        // pm4py draws this title at 20 points, whatever the font size.
        dot.set(vec![
            ("label", Some(title_label(title, 10))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }
    if !tree.nodes.is_empty() {
        draw(&mut dot, tree, 0, None);
    }
    dot.set(vec![("overlap", Some("false".to_owned()))]);
    dot.set(vec![("splines", Some("false".to_owned()))]);
    dot.set(vec![("rankdir", Some("LR".to_owned()))]);
    dot.finish()
}

/// pm4py's `draw_recursive`: the node, the edge from its parent, then its
/// children in label order.
fn draw(dot: &mut Dot, tree: &PrefixTree, index: usize, parent: Option<&str>) {
    let node = &tree.nodes[index];
    let id = format!("n{index}");
    if let Some(label) = &node.label {
        dot.node(&id, Some(label), vec![("shape", Some("box".to_owned()))]);
    }
    if let Some(parent) = parent {
        dot.edge(parent, &id, None, vec![]);
    }
    let here = node.label.is_some().then_some(id.as_str());
    for &child in node.children.values() {
        draw(dot, tree, child, here);
    }
}
