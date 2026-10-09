//! OCEL graphs, ported from pm4py's `objects/conversion/ocel/variants`
//! (`ocel_to_nx` and `ocel_features_to_nx`).

use std::collections::{BTreeMap, HashMap};

use ichnos_core::AttributeValue;

use crate::Ocel;
use crate::constants::{
    CHANGED_FIELD, EVENT_ACTIVITY, EVENT_ID, EVENT_TIMESTAMP, OBJECT_ID, OBJECT_TYPE,
};
use crate::graphs::{ObjectGraphKind, discover_objects_graph};

/// A graph in place of pm4py's NetworkX graph.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcelGraph {
    /// Whether two nodes can have more than one edge (NetworkX's
    /// `MultiDiGraph`) or at most one (`DiGraph`).
    pub multigraph: bool,
    /// The nodes, in the order NetworkX adds them.
    pub nodes: Vec<OcelGraphNode>,
    /// The edges, in the order they are added.
    pub edges: Vec<OcelGraphEdge>,
}

/// A node of an [`OcelGraph`].
#[derive(Debug, Clone, PartialEq)]
pub struct OcelGraphNode {
    /// The node id.
    pub id: String,
    /// The node's `attr` dictionary; empty for a node that only an edge adds.
    pub attributes: BTreeMap<String, AttributeValue>,
}

/// An edge of an [`OcelGraph`].
#[derive(Debug, Clone, PartialEq)]
pub struct OcelGraphEdge {
    /// The source node id.
    pub source: String,
    /// The target node id.
    pub target: String,
    /// The edge's `attr` dictionary.
    pub attributes: BTreeMap<String, AttributeValue>,
}

/// Options for [`convert_ocel_to_networkx`], with pm4py's defaults.
#[derive(Debug, Clone)]
pub struct OcelToNxOptions {
    /// Add the directly-follows edges of each object's lifecycle. Default
    /// `true`.
    pub include_df: bool,
    /// Add a node for each object change. Default `true`.
    pub include_object_changes: bool,
}

impl Default for OcelToNxOptions {
    fn default() -> Self {
        OcelToNxOptions {
            include_df: true,
            include_object_changes: true,
        }
    }
}

/// Options for [`convert_ocel_features_to_networkx`]: which object graphs to
/// add. pm4py adds all five.
#[derive(Debug, Clone)]
pub struct OcelFeaturesToNxOptions {
    /// The object graphs to add, in order. A later graph's edge type
    /// replaces an earlier one's on the same pair.
    pub graphs: Vec<ObjectGraphKind>,
}

impl Default for OcelFeaturesToNxOptions {
    fn default() -> Self {
        OcelFeaturesToNxOptions {
            graphs: ObjectGraphKind::ALL.to_vec(),
        }
    }
}

#[derive(Default)]
struct Builder {
    graph: OcelGraph,
    nodes: HashMap<String, usize>,
    edges: HashMap<(String, String), usize>,
}

type Attrs = BTreeMap<String, AttributeValue>;

impl Builder {
    /// NetworkX's `add_node`: a node added again gets the new attributes.
    fn node(&mut self, id: &str, attributes: Option<Attrs>) {
        match self.nodes.get(id) {
            Some(&i) => {
                if let Some(a) = attributes {
                    self.graph.nodes[i].attributes = a;
                }
            }
            None => {
                self.nodes.insert(id.to_owned(), self.graph.nodes.len());
                self.graph.nodes.push(OcelGraphNode {
                    id: id.to_owned(),
                    attributes: attributes.unwrap_or_default(),
                });
            }
        }
    }

    /// NetworkX's `add_edge`. Without a multigraph, an edge added again
    /// gets the new attributes.
    fn edge(&mut self, source: &str, target: &str, attributes: Attrs) {
        self.node(source, None);
        self.node(target, None);
        let key = (source.to_owned(), target.to_owned());
        if !self.graph.multigraph {
            if let Some(&i) = self.edges.get(&key) {
                self.graph.edges[i].attributes = attributes;
                return;
            }
            self.edges.insert(key, self.graph.edges.len());
        }
        self.graph.edges.push(OcelGraphEdge {
            source: source.to_owned(),
            target: target.to_owned(),
            attributes,
        });
    }
}

fn text(v: &str) -> AttributeValue {
    AttributeValue::String(v.into())
}

fn attrs<'a>(pairs: impl IntoIterator<Item = (&'a str, AttributeValue)>) -> Attrs {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// pm4py's missing values: a float NaN is left out like pandas' `NaN`.
fn present(v: &AttributeValue) -> bool {
    !matches!(v.plain(), AttributeValue::Float(x) if x.is_nan())
}

/// The graph of events and objects of `ocel`, as pm4py's
/// `convert_ocel_to_networkx` builds it with the `ocel_to_nx` variant.
///
/// The nodes are the events, then the objects, then the object changes
/// (`@@change##<i>`). Each node's attributes are its row of pm4py's table
/// plus `type` (`EVENT`, `OBJECT` or `CHANGE`). The edges are typed
/// `E2O` (event to object, with its qualifier), `O2O` (with its
/// qualifier), `DF` (consecutive events of an object's lifecycle, objects in
/// id order, with the `object`) and `CHANGE` (change to object). A missing
/// qualifier is the empty string.
///
/// A change node holds only the value of its own field; pm4py's row also
/// holds the other fields' columns, as `NaN`.
pub fn convert_ocel_to_networkx(ocel: &Ocel, options: &OcelToNxOptions) -> OcelGraph {
    let mut b = Builder {
        graph: OcelGraph {
            multigraph: true,
            ..OcelGraph::default()
        },
        ..Builder::default()
    };
    for e in &ocel.events {
        let mut a = attrs([
            (EVENT_ID, text(&e.id)),
            (EVENT_ACTIVITY, text(&e.activity)),
            (EVENT_TIMESTAMP, AttributeValue::Date(e.timestamp)),
        ]);
        for (k, v) in &e.attributes {
            if present(v) {
                a.insert(k.to_string(), v.clone());
            }
        }
        a.insert("type".to_owned(), text("EVENT"));
        b.node(&e.id, Some(a));
    }
    for o in &ocel.objects {
        let mut a = attrs([
            (OBJECT_ID, text(&o.id)),
            (OBJECT_TYPE, text(&o.object_type)),
        ]);
        for (k, v) in &o.attributes {
            if present(v) {
                a.insert(k.to_string(), v.clone());
            }
        }
        a.insert("type".to_owned(), text("OBJECT"));
        b.node(&o.id, Some(a));
    }
    let qualified = |kind: &str, q: &Option<std::sync::Arc<str>>| {
        attrs([
            ("type", text(kind)),
            ("qualifier", text(q.as_deref().unwrap_or(""))),
        ])
    };
    for r in &ocel.relations {
        b.edge(&r.event, &r.object, qualified("E2O", &r.qualifier));
    }
    for r in &ocel.o2o {
        b.edge(&r.source, &r.target, qualified("O2O", &r.qualifier));
    }
    if options.include_df {
        let mut lifecycles: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for r in &ocel.relations {
            lifecycles.entry(&r.object).or_default().push(&r.event);
        }
        for (object, events) in lifecycles {
            for w in events.windows(2) {
                b.edge(
                    w[0],
                    w[1],
                    attrs([("type", text("DF")), ("object", text(object))]),
                );
            }
        }
    }
    if options.include_object_changes {
        for (i, c) in ocel.object_changes.iter().enumerate() {
            let id = format!("@@change##{i}");
            let mut a = attrs([
                (OBJECT_ID, text(&c.object)),
                (OBJECT_TYPE, text(&c.object_type)),
                (EVENT_TIMESTAMP, AttributeValue::Date(c.timestamp)),
                (CHANGED_FIELD, text(&c.field)),
            ]);
            if let Some(v) = c.value.as_ref().filter(|v| present(v)) {
                a.insert(c.field.to_string(), v.clone());
            }
            a.insert("type".to_owned(), text("CHANGE"));
            b.node(&id, Some(a));
            b.edge(&id, &c.object, attrs([("type", text("CHANGE"))]));
        }
    }
    b.graph
}

/// The graph of object relations of `ocel`, as pm4py's
/// `convert_ocel_to_networkx` builds it with the `ocel_features_to_nx`
/// variant.
///
/// The edges are the pairs of the object graphs (see
/// [`discover_objects_graph`]), each typed `INTERACTION`, `DESCENDANTS`,
/// `INHERITANCE`, `COBIRTH` or `CODEATH`; a pair in more than one graph keeps
/// the type of the last. The nodes are the objects the edges join, without
/// attributes. pm4py adds each graph's pairs in the iteration order of a
/// Python set; here they come in id order.
pub fn convert_ocel_features_to_networkx(
    ocel: &Ocel,
    options: &OcelFeaturesToNxOptions,
) -> OcelGraph {
    let mut b = Builder::default();
    for &kind in &options.graphs {
        let label = kind.name().trim_start_matches("object_").to_uppercase();
        for (x, y) in discover_objects_graph(ocel, kind) {
            b.edge(&x, &y, attrs([("type", text(&label))]));
        }
    }
    b.graph
}
