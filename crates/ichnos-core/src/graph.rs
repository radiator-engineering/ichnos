//! The log as a directed graph of cases, events and attribute values. Port of
//! pm4py's `convert_log_to_networkx` (`objects/conversion/log/variants/to_nx.py`),
//! with a petgraph graph in place of a NetworkX one.

use std::sync::Arc;

use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;

use crate::attribute::{AttributeValue, Attributes};
use crate::error::{Error, Result};
use crate::log::{CaseKey, EventLog};

/// A log graph. Build it with [`EventLog::to_graph`].
pub type LogGraph = DiGraph<LogNode, LogEdge>;

/// A node of a [`LogGraph`].
#[derive(Debug, Clone, PartialEq)]
pub enum LogNode {
    /// A trace, with its attributes.
    Case {
        /// The case ID.
        case_id: AttributeValue,
        /// The trace attributes.
        attributes: Attributes,
    },
    /// An event, with its attributes.
    Event {
        /// The case ID of its trace.
        case_id: AttributeValue,
        /// Its index in the trace.
        index: usize,
        /// The event attributes.
        attributes: Attributes,
    },
    /// An attribute value shared by the cases and events that carry it.
    Attribute(AttributeValue),
}

impl LogNode {
    /// pm4py's node ID: `CASE=<case id>`, `EVENT=<case id>_<index>`, or the
    /// attribute value, formatted as Python's `str()` would.
    pub fn id(&self) -> String {
        match self {
            Self::Case { case_id, .. } => format!("CASE={case_id}"),
            Self::Event { case_id, index, .. } => format!("EVENT={case_id}_{index}"),
            Self::Attribute(value) => value.to_string(),
        }
    }
}

/// An edge of a [`LogGraph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEdge {
    /// From an event to its case (pm4py type `BELONGS_TO`).
    BelongsTo,
    /// From an event to the next event of its trace (pm4py type `DF`).
    DirectlyFollows,
    /// From a case or event to the value of one of its attributes (pm4py
    /// type `ATTRIBUTE_EDGE`), with the attribute key.
    Attribute(Arc<str>),
}

#[derive(PartialEq, Eq, Hash)]
enum NodeKey {
    Case(String),
    Event(String, usize),
    Attribute(CaseKey),
}

/// Adds nodes once per key. A repeated key keeps its node and takes the new
/// payload, as NetworkX's `add_node` does.
struct Builder {
    graph: LogGraph,
    nodes: FxHashMap<NodeKey, NodeIndex>,
}

impl Builder {
    fn node(&mut self, key: NodeKey, node: LogNode) -> NodeIndex {
        match self.nodes.get(&key) {
            Some(&i) => {
                self.graph[i] = node;
                i
            }
            None => {
                let i = self.graph.add_node(node);
                self.nodes.insert(key, i);
                i
            }
        }
    }

    /// An attribute node, added only if new, as in pm4py.
    fn value(&mut self, key: &str, value: &AttributeValue) -> Result<NodeIndex> {
        let hashable = CaseKey::new(value).ok_or_else(|| Error::NestedAttribute {
            key: key.to_owned(),
            kind: value.type_name(),
        })?;
        let key = NodeKey::Attribute(hashable);
        if let Some(&i) = self.nodes.get(&key) {
            return Ok(i);
        }
        let i = self
            .graph
            .add_node(LogNode::Attribute(value.plain().clone()));
        self.nodes.insert(key, i);
        Ok(i)
    }
}

impl EventLog {
    /// The log as a directed graph. Port of pm4py's
    /// `convert_log_to_networkx`.
    ///
    /// Each trace is a [`LogNode::Case`] and each event a [`LogNode::Event`]
    /// with a [`LogEdge::BelongsTo`] edge to its case. With
    /// `directly_follows`, consecutive events are joined by
    /// [`LogEdge::DirectlyFollows`]. Each value of a trace attribute in
    /// `case_attributes` and of an event attribute in `event_attributes`
    /// becomes one [`LogNode::Attribute`] node, linked from every case or
    /// event that carries it.
    ///
    /// The case ID is the trace attribute `concept:name`. Traces with the same
    /// case ID share their case and event nodes, as in pm4py, and adding an
    /// edge twice keeps one edge with the later label. Fails if a trace has no
    /// case ID or a listed attribute holds a list or container.
    pub fn to_graph<S: AsRef<str>>(
        &self,
        directly_follows: bool,
        case_attributes: &[S],
        event_attributes: &[S],
    ) -> Result<LogGraph> {
        let mut b = Builder {
            graph: LogGraph::new(),
            nodes: FxHashMap::default(),
        };
        for (t, trace) in self.traces.iter().enumerate() {
            let case_id = trace.case_id().ok_or(Error::MissingCaseId(t))?;
            let id = case_id.to_string();
            let case = b.node(
                NodeKey::Case(id.clone()),
                LogNode::Case {
                    case_id: case_id.plain().clone(),
                    attributes: trace.attributes.clone(),
                },
            );
            let mut previous = None;
            for (index, event) in trace.events.iter().enumerate() {
                let node = b.node(
                    NodeKey::Event(id.clone(), index),
                    LogNode::Event {
                        case_id: case_id.plain().clone(),
                        index,
                        attributes: event.attributes.clone(),
                    },
                );
                b.graph.update_edge(node, case, LogEdge::BelongsTo);
                for key in event_attributes {
                    let key = key.as_ref();
                    if let Some(value) = event.get(key) {
                        let target = b.value(key, value)?;
                        b.graph
                            .update_edge(node, target, LogEdge::Attribute(key.into()));
                    }
                }
                if directly_follows && let Some(previous) = previous {
                    b.graph
                        .update_edge(previous, node, LogEdge::DirectlyFollows);
                }
                previous = Some(node);
            }
            for key in case_attributes {
                let key = key.as_ref();
                if let Some(value) = trace.attributes.get(key) {
                    let target = b.value(key, value)?;
                    b.graph
                        .update_edge(case, target, LogEdge::Attribute(key.into()));
                }
            }
        }
        Ok(b.graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::EventKeys;

    #[test]
    fn builds_cases_events_and_shared_attribute_nodes() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A,B", "A"], ",", &keys);
        log.traces[0].attributes.insert("creator", "x");
        let graph = log.to_graph(true, &["creator"], &["concept:name"]).unwrap();

        let mut ids: Vec<String> = graph.node_weights().map(LogNode::id).collect();
        ids.sort();
        assert_eq!(
            ids,
            [
                "A",
                "B",
                "CASE=0",
                "CASE=1",
                "EVENT=0_0",
                "EVENT=0_1",
                "EVENT=1_0",
                "x"
            ]
        );

        let mut edges: Vec<(String, String, LogEdge)> = graph
            .edge_indices()
            .map(|e| {
                let (s, t) = graph.edge_endpoints(e).unwrap();
                (graph[s].id(), graph[t].id(), graph[e].clone())
            })
            .collect();
        edges.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        let name: Arc<str> = "concept:name".into();
        assert_eq!(
            edges,
            [
                (
                    "CASE=0".into(),
                    "x".into(),
                    LogEdge::Attribute("creator".into())
                ),
                (
                    "EVENT=0_0".into(),
                    "A".into(),
                    LogEdge::Attribute(name.clone())
                ),
                ("EVENT=0_0".into(), "CASE=0".into(), LogEdge::BelongsTo),
                (
                    "EVENT=0_0".into(),
                    "EVENT=0_1".into(),
                    LogEdge::DirectlyFollows
                ),
                (
                    "EVENT=0_1".into(),
                    "B".into(),
                    LogEdge::Attribute(name.clone())
                ),
                ("EVENT=0_1".into(), "CASE=0".into(), LogEdge::BelongsTo),
                ("EVENT=1_0".into(), "A".into(), LogEdge::Attribute(name)),
                ("EVENT=1_0".into(), "CASE=1".into(), LogEdge::BelongsTo),
            ]
        );
    }

    #[test]
    fn without_directly_follows_and_without_case_id() {
        let keys = EventKeys::default();
        let log = EventLog::from_trace_strings(["A,B"], ",", &keys);
        let graph = log.to_graph::<&str>(false, &[], &[]).unwrap();
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        let mut log = log;
        log.traces[0].attributes.clear();
        assert!(matches!(
            log.to_graph::<&str>(false, &[], &[]),
            Err(Error::MissingCaseId(0))
        ));
    }
}
