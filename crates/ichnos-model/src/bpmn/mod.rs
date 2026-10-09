//! BPMN process diagrams, ported from pm4py's `objects/bpmn`.
//!
//! A [`Bpmn`] stores its nodes and flows in arenas, addressed by the typed
//! indices [`NodeId`] and [`FlowId`]. An id is only meaningful for the
//! diagram that created it. Each node and flow also has a string id, the
//! BPMN XML `id`, which pm4py uses for lookups and in conversions.
//!
//! pm4py models the node types as a class hierarchy. Here a node has one
//! [`NodeKind`]; [`NodeKind::class_name`] gives the pm4py class. Layout
//! (positions, sizes, waypoints) is not stored.

mod reduction;
mod semantics;

pub use semantics::{BpmnMarking, NodeNotEnabled};

use std::collections::{BTreeMap, BTreeSet};

use crate::Label;

/// Index of a node in a [`Bpmn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

/// Index of a flow in a [`Bpmn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlowId(u32);

impl NodeId {
    /// Returns the position of this node in its arena.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl FlowId {
    /// Returns the position of this flow in its arena.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("a BPMN diagram holds at most u32::MAX nodes and flows")
}

/// The type of a gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GatewayKind {
    /// Exclusive (XOR) gateway.
    Exclusive,
    /// Parallel (AND) gateway.
    Parallel,
    /// Inclusive (OR) gateway.
    Inclusive,
    /// Event-based gateway.
    EventBased,
}

/// The `gatewayDirection` of a gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum GatewayDirection {
    /// No direction given.
    #[default]
    Unspecified,
    /// A split.
    Diverging,
    /// A join.
    Converging,
}

impl GatewayDirection {
    /// The BPMN attribute value: `Unspecified`, `Diverging` or
    /// `Converging`.
    pub fn as_str(self) -> &'static str {
        match self {
            GatewayDirection::Unspecified => "Unspecified",
            GatewayDirection::Diverging => "Diverging",
            GatewayDirection::Converging => "Converging",
        }
    }
}

/// The trigger of a start event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartTrigger {
    /// pm4py's base `StartEvent`.
    Plain,
    /// `NormalStartEvent`.
    Normal,
    /// `MessageStartEvent`.
    Message,
}

/// The trigger of an intermediate catch event or a boundary event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatchTrigger {
    /// pm4py's base class (`IntermediateCatchEvent`, `BoundaryEvent`).
    Plain,
    /// A message event.
    Message,
    /// An error event.
    Error,
    /// A cancel event.
    Cancel,
}

/// The trigger of an intermediate throw event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThrowTrigger {
    /// pm4py's base `IntermediateThrowEvent`.
    Plain,
    /// `NormalIntermediateThrowEvent`.
    Normal,
    /// `MessageIntermediateThrowEvent`.
    Message,
}

/// The trigger of an end event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EndTrigger {
    /// pm4py's base `EndEvent`.
    Plain,
    /// `NormalEndEvent`.
    Normal,
    /// `MessageEndEvent`.
    Message,
    /// `TerminateEndEvent`.
    Terminate,
    /// `ErrorEndEvent`.
    Error,
    /// `CancelEndEvent`.
    Cancel,
}

/// The type of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskKind {
    /// A plain `Task`.
    Plain,
    /// A `UserTask`.
    User,
    /// A `SendTask`.
    Send,
}

/// What a node is, with the attributes of its type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A start event.
    StartEvent {
        /// The event trigger.
        trigger: StartTrigger,
        /// BPMN `isInterrupting`.
        is_interrupting: bool,
        /// BPMN `parallelMultiple`.
        parallel_multiple: bool,
    },
    /// An intermediate catch event.
    IntermediateCatchEvent(CatchTrigger),
    /// An intermediate throw event.
    IntermediateThrowEvent(ThrowTrigger),
    /// A boundary event attached to an activity.
    BoundaryEvent {
        /// The event trigger.
        trigger: CatchTrigger,
        /// The string id of the activity the event is attached to.
        activity: Option<String>,
    },
    /// An end event.
    EndEvent(EndTrigger),
    /// A task.
    Task(TaskKind),
    /// A subprocess. Its children are the nodes whose `process` is the
    /// subprocess's string id.
    SubProcess {
        /// The nesting depth, as pm4py's importer sets it.
        depth: Option<u32>,
    },
    /// A gateway.
    Gateway {
        /// The gateway type.
        kind: GatewayKind,
        /// The gateway direction.
        direction: GatewayDirection,
    },
    /// A text annotation.
    TextAnnotation {
        /// The annotation text.
        text: Option<String>,
    },
    /// A participant (pool) of a collaboration.
    Participant {
        /// The string id of the participant's process.
        process_ref: Option<String>,
    },
    /// A collaboration.
    Collaboration,
}

impl NodeKind {
    /// A plain start event, as pm4py's converters create it.
    pub fn start_event() -> Self {
        NodeKind::StartEvent {
            trigger: StartTrigger::Plain,
            is_interrupting: true,
            parallel_multiple: false,
        }
    }

    /// A normal end event, as pm4py's converters create it.
    pub fn end_event() -> Self {
        NodeKind::EndEvent(EndTrigger::Normal)
    }

    /// A plain task.
    pub fn task() -> Self {
        NodeKind::Task(TaskKind::Plain)
    }

    /// A gateway.
    pub fn gateway(kind: GatewayKind, direction: GatewayDirection) -> Self {
        NodeKind::Gateway { kind, direction }
    }

    /// The name of the pm4py class for this kind, such as
    /// `NormalStartEvent` or `ExclusiveGateway`.
    pub fn class_name(&self) -> &'static str {
        match self {
            NodeKind::StartEvent { trigger, .. } => match trigger {
                StartTrigger::Plain => "StartEvent",
                StartTrigger::Normal => "NormalStartEvent",
                StartTrigger::Message => "MessageStartEvent",
            },
            NodeKind::IntermediateCatchEvent(t) => match t {
                CatchTrigger::Plain => "IntermediateCatchEvent",
                CatchTrigger::Message => "MessageIntermediateCatchEvent",
                CatchTrigger::Error => "ErrorIntermediateCatchEvent",
                CatchTrigger::Cancel => "CancelIntermediateCatchEvent",
            },
            NodeKind::IntermediateThrowEvent(t) => match t {
                ThrowTrigger::Plain => "IntermediateThrowEvent",
                ThrowTrigger::Normal => "NormalIntermediateThrowEvent",
                ThrowTrigger::Message => "MessageIntermediateThrowEvent",
            },
            NodeKind::BoundaryEvent { trigger, .. } => match trigger {
                CatchTrigger::Plain => "BoundaryEvent",
                CatchTrigger::Message => "MessageBoundaryEvent",
                CatchTrigger::Error => "ErrorBoundaryEvent",
                CatchTrigger::Cancel => "CancelBoundaryEvent",
            },
            NodeKind::EndEvent(t) => match t {
                EndTrigger::Plain => "EndEvent",
                EndTrigger::Normal => "NormalEndEvent",
                EndTrigger::Message => "MessageEndEvent",
                EndTrigger::Terminate => "TerminateEndEvent",
                EndTrigger::Error => "ErrorEndEvent",
                EndTrigger::Cancel => "CancelEndEvent",
            },
            NodeKind::Task(k) => match k {
                TaskKind::Plain => "Task",
                TaskKind::User => "UserTask",
                TaskKind::Send => "SendTask",
            },
            NodeKind::SubProcess { .. } => "SubProcess",
            NodeKind::Gateway { kind, .. } => match kind {
                GatewayKind::Exclusive => "ExclusiveGateway",
                GatewayKind::Parallel => "ParallelGateway",
                GatewayKind::Inclusive => "InclusiveGateway",
                GatewayKind::EventBased => "EventBasedGateway",
            },
            NodeKind::TextAnnotation { .. } => "TextAnnotation",
            NodeKind::Participant { .. } => "Participant",
            NodeKind::Collaboration => "Collaboration",
        }
    }

    /// Returns `true` for events (pm4py's `BPMN.Event` subclasses).
    pub fn is_event(&self) -> bool {
        matches!(
            self,
            NodeKind::StartEvent { .. }
                | NodeKind::IntermediateCatchEvent(_)
                | NodeKind::IntermediateThrowEvent(_)
                | NodeKind::BoundaryEvent { .. }
                | NodeKind::EndEvent(_)
        )
    }

    /// Returns `true` for tasks and subprocesses (pm4py's `BPMN.Activity`).
    pub fn is_activity(&self) -> bool {
        matches!(self, NodeKind::Task(_) | NodeKind::SubProcess { .. })
    }

    /// Returns `true` for gateways.
    pub fn is_gateway(&self) -> bool {
        matches!(self, NodeKind::Gateway { .. })
    }

    /// Returns `true` for tasks of any [`TaskKind`].
    pub fn is_task(&self) -> bool {
        matches!(self, NodeKind::Task(_))
    }

    /// Returns `true` for start events.
    pub fn is_start_event(&self) -> bool {
        matches!(self, NodeKind::StartEvent { .. })
    }

    /// Returns `true` for end events.
    pub fn is_end_event(&self) -> bool {
        matches!(self, NodeKind::EndEvent(_))
    }

    /// The gateway type, or `None` if this is not a gateway.
    pub fn gateway_kind(&self) -> Option<GatewayKind> {
        match self {
            NodeKind::Gateway { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// The gateway direction, or `None` if this is not a gateway.
    pub fn gateway_direction(&self) -> Option<GatewayDirection> {
        match self {
            NodeKind::Gateway { direction, .. } => Some(*direction),
            _ => None,
        }
    }
}

/// A node of a BPMN diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The string id (BPMN XML `id`).
    pub id: String,
    /// The node name. For a task, this is its activity.
    pub name: String,
    /// The string id of the process the node belongs to: the diagram's
    /// [`Bpmn::process_id`], or the id of the enclosing subprocess.
    pub process: String,
    /// The node type.
    pub kind: NodeKind,
    in_flows: Vec<FlowId>,
    out_flows: Vec<FlowId>,
}

impl Node {
    /// Flows that end in this node, in insertion order.
    pub fn in_flows(&self) -> &[FlowId] {
        &self.in_flows
    }

    /// Flows that start in this node, in insertion order.
    pub fn out_flows(&self) -> &[FlowId] {
        &self.out_flows
    }
}

/// The type of a flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlowKind {
    /// A sequence flow.
    Sequence,
    /// A message flow between participants.
    Message,
    /// An association, for example to a text annotation.
    Association,
}

/// A flow (edge) of a BPMN diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    /// The string id (BPMN XML `id`).
    pub id: String,
    /// The flow name.
    pub name: String,
    /// The string id of the process the flow belongs to.
    pub process: String,
    /// The flow type.
    pub kind: FlowKind,
    source: NodeId,
    target: NodeId,
}

impl Flow {
    /// The node the flow starts in.
    pub fn source(&self) -> NodeId {
        self.source
    }

    /// The node the flow ends in.
    pub fn target(&self) -> NodeId {
        self.target
    }
}

/// Errors from editing a [`Bpmn`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BpmnError {
    /// A flow named a node that is not in the diagram.
    #[error("node {0:?} is not in the diagram")]
    NoSuchNode(NodeId),
}

/// A BPMN process diagram (pm4py's `BPMN`).
///
/// Nodes and flows live in arenas. Removing one leaves a tombstone, so the
/// ids of the others stay valid. Iteration follows id order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bpmn {
    /// The string id of the top-level process.
    pub process_id: String,
    /// The diagram name.
    pub name: String,
    nodes: Vec<Option<Node>>,
    flows: Vec<Option<Flow>>,
}

impl Default for Bpmn {
    fn default() -> Self {
        Bpmn::new("process")
    }
}

impl Bpmn {
    /// Creates an empty diagram whose top-level process has the given id.
    pub fn new(process_id: impl Into<String>) -> Self {
        Bpmn {
            process_id: process_id.into(),
            name: String::new(),
            nodes: Vec::new(),
            flows: Vec::new(),
        }
    }

    /// Adds a node in the top-level process, with the string id `id_<n>`,
    /// where `n` is its index.
    pub fn add_node(&mut self, kind: NodeKind, name: impl Into<String>) -> NodeId {
        let id = format!("id_{}", self.nodes.len());
        self.add_node_with_id(id, kind, name)
    }

    /// Adds a node in the top-level process with the given string id.
    pub fn add_node_with_id(
        &mut self,
        id: impl Into<String>,
        kind: NodeKind,
        name: impl Into<String>,
    ) -> NodeId {
        let n = NodeId(to_u32(self.nodes.len()));
        self.nodes.push(Some(Node {
            id: id.into(),
            name: name.into(),
            process: self.process_id.clone(),
            kind,
            in_flows: Vec::new(),
            out_flows: Vec::new(),
        }));
        n
    }

    /// Adds a sequence flow in the top-level process, with the string id
    /// `flow_<n>`, where `n` is its index.
    pub fn add_flow(&mut self, source: NodeId, target: NodeId) -> Result<FlowId, BpmnError> {
        let id = format!("flow_{}", self.flows.len());
        self.add_flow_with(FlowKind::Sequence, id, "", source, target)
    }

    /// Adds a flow of the given kind, string id and name in the top-level
    /// process.
    pub fn add_flow_with(
        &mut self,
        kind: FlowKind,
        id: impl Into<String>,
        name: impl Into<String>,
        source: NodeId,
        target: NodeId,
    ) -> Result<FlowId, BpmnError> {
        for n in [source, target] {
            if !self.contains_node(n) {
                return Err(BpmnError::NoSuchNode(n));
            }
        }
        let f = FlowId(to_u32(self.flows.len()));
        self.flows.push(Some(Flow {
            id: id.into(),
            name: name.into(),
            process: self.process_id.clone(),
            kind,
            source,
            target,
        }));
        self.node_slot(source).out_flows.push(f);
        self.node_slot(target).in_flows.push(f);
        Ok(f)
    }

    fn node_slot(&mut self, id: NodeId) -> &mut Node {
        self.nodes[id.index()]
            .as_mut()
            .expect("the node is in the diagram")
    }

    /// Removes a flow. Does nothing if it is already gone.
    pub fn remove_flow(&mut self, id: FlowId) {
        let Some(flow) = self.flows.get_mut(id.index()).and_then(Option::take) else {
            return;
        };
        if let Some(Some(s)) = self.nodes.get_mut(flow.source.index()) {
            s.out_flows.retain(|&f| f != id);
        }
        if let Some(Some(t)) = self.nodes.get_mut(flow.target.index()) {
            t.in_flows.retain(|&f| f != id);
        }
    }

    /// Removes a node and every flow that touches it. Does nothing if the
    /// node is already gone.
    pub fn remove_node(&mut self, id: NodeId) {
        let Some(node) = self.nodes.get(id.index()).and_then(Option::as_ref) else {
            return;
        };
        let flows: Vec<FlowId> = node
            .in_flows
            .iter()
            .chain(&node.out_flows)
            .copied()
            .collect();
        for f in flows {
            self.remove_flow(f);
        }
        self.nodes[id.index()] = None;
    }

    /// Returns `true` if the node is in the diagram.
    pub fn contains_node(&self, id: NodeId) -> bool {
        matches!(self.nodes.get(id.index()), Some(Some(_)))
    }

    /// Returns `true` if the flow is in the diagram.
    pub fn contains_flow(&self, id: FlowId) -> bool {
        matches!(self.flows.get(id.index()), Some(Some(_)))
    }

    /// Returns a node.
    ///
    /// # Panics
    ///
    /// If the node is not in the diagram.
    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes[id.index()]
            .as_ref()
            .unwrap_or_else(|| panic!("node {id:?} is not in the diagram"))
    }

    /// Returns a node for editing its id, name, process or kind.
    ///
    /// # Panics
    ///
    /// If the node is not in the diagram.
    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        self.nodes[id.index()]
            .as_mut()
            .unwrap_or_else(|| panic!("node {id:?} is not in the diagram"))
    }

    /// Returns a flow.
    ///
    /// # Panics
    ///
    /// If the flow is not in the diagram.
    pub fn flow(&self, id: FlowId) -> &Flow {
        self.flows[id.index()]
            .as_ref()
            .unwrap_or_else(|| panic!("flow {id:?} is not in the diagram"))
    }

    /// Returns a flow for editing its id, name, process or kind.
    ///
    /// # Panics
    ///
    /// If the flow is not in the diagram.
    pub fn flow_mut(&mut self, id: FlowId) -> &mut Flow {
        self.flows[id.index()]
            .as_mut()
            .unwrap_or_else(|| panic!("flow {id:?} is not in the diagram"))
    }

    /// The live nodes, in id order.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.as_ref().map(|n| (NodeId(to_u32(i)), n)))
    }

    /// The live flows, in id order.
    pub fn flows(&self) -> impl Iterator<Item = (FlowId, &Flow)> {
        self.flows
            .iter()
            .enumerate()
            .filter_map(|(i, f)| f.as_ref().map(|f| (FlowId(to_u32(i)), f)))
    }

    /// The ids of the live nodes, in id order.
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes().map(|(id, _)| id)
    }

    /// The number of live nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.iter().flatten().count()
    }

    /// The number of live flows.
    pub fn flow_count(&self) -> usize {
        self.flows.iter().flatten().count()
    }

    /// Sets the process of every node and flow to the top-level process, as
    /// pm4py's converters do before they return.
    pub fn set_all_processes_to_main(&mut self) {
        let p = self.process_id.clone();
        for n in self.nodes.iter_mut().flatten() {
            n.process.clone_from(&p);
        }
        for f in self.flows.iter_mut().flatten() {
            f.process.clone_from(&p);
        }
    }

    /// The first node with the given string id (pm4py's `get_node_by_id`).
    pub fn node_by_id(&self, id: &str) -> Option<NodeId> {
        self.nodes().find(|(_, n)| n.id == id).map(|(i, _)| i)
    }

    fn nodes_where(&self, mut f: impl FnMut(&Node) -> bool) -> Vec<NodeId> {
        self.nodes().filter(|(_, n)| f(n)).map(|(i, _)| i).collect()
    }

    /// Start events of the top-level process (pm4py's
    /// `get_global_start_events`).
    pub fn global_start_events(&self) -> Vec<NodeId> {
        self.nodes_where(|n| n.kind.is_start_event() && n.process == self.process_id)
    }

    /// Boundary events attached to the activity with the given string id
    /// (pm4py's `get_boundary_events_of_activity`).
    pub fn boundary_events_of(&self, activity_id: &str) -> Vec<NodeId> {
        self.nodes_where(|n| {
            matches!(&n.kind, NodeKind::BoundaryEvent { activity: Some(a), .. } if a == activity_id)
        })
    }

    /// Message boundary events attached to the activity with the given
    /// string id (pm4py's `get_external_boundary_events_of_activity`).
    pub fn external_boundary_events_of(&self, activity_id: &str) -> Vec<NodeId> {
        self.nodes_where(|n| {
            matches!(
                &n.kind,
                NodeKind::BoundaryEvent { trigger: CatchTrigger::Message, activity: Some(a) }
                    if a == activity_id
            )
        })
    }

    /// The chain of processes that contain a node, innermost first, ending
    /// with the top-level process (pm4py's `get_processes_deep`). The chain
    /// stops early if a process id names no node.
    pub fn processes_deep(&self, node: NodeId) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = node;
        loop {
            let p = &self.node(current).process;
            if *p == self.process_id {
                out.push(self.process_id.clone());
                return out;
            }
            out.push(p.clone());
            match self.node_by_id(p) {
                Some(parent) if seen.insert(parent) => current = parent,
                _ => return out,
            }
        }
    }

    /// Nodes inside the process with the given id (pm4py's
    /// `get_all_nodes_inside_process`). With `deep`, nodes of nested
    /// subprocesses count too.
    pub fn nodes_inside_process(&self, process_id: &str, deep: bool) -> Vec<NodeId> {
        self.nodes()
            .filter(|(i, n)| {
                if deep {
                    self.processes_deep(*i).iter().any(|p| p == process_id)
                } else {
                    n.process == process_id
                }
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Subprocesses directly inside the given process (pm4py's
    /// `get_all_direct_child_subprocesses`). Unless `include_normal`, only
    /// subprocesses with boundary events or termination events count.
    pub fn direct_child_subprocesses(
        &self,
        process_id: &str,
        include_normal: bool,
    ) -> BTreeSet<NodeId> {
        self.nodes()
            .filter(|(_, n)| {
                matches!(n.kind, NodeKind::SubProcess { .. })
                    && n.process == process_id
                    && (include_normal
                        || !self.boundary_events_of(&n.id).is_empty()
                        || !self.termination_events_for_petri_net(&n.id).is_empty())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Subprocesses inside the given process at any depth (pm4py's
    /// `get_all_child_subprocesses`).
    pub fn child_subprocesses(&self, process_id: &str, include_normal: bool) -> BTreeSet<NodeId> {
        let mut out = BTreeSet::new();
        let mut stack: Vec<NodeId> = self
            .direct_child_subprocesses(process_id, include_normal)
            .into_iter()
            .collect();
        while let Some(s) = stack.pop() {
            if out.insert(s) {
                stack.extend(self.direct_child_subprocesses(&self.node(s).id, include_normal));
            }
        }
        out
    }

    /// Subprocesses, deepest first (pm4py's
    /// `get_subprocesses_sorted_by_depth`). Subprocesses without a depth
    /// come last.
    pub fn subprocesses_by_depth(&self) -> Vec<NodeId> {
        let mut subs: Vec<(Option<u32>, NodeId)> = self
            .nodes()
            .filter_map(|(i, n)| match n.kind {
                NodeKind::SubProcess { depth } => Some((depth, i)),
                _ => None,
            })
            .collect();
        subs.sort_by_key(|s| std::cmp::Reverse(s.0));
        subs.into_iter().map(|(_, i)| i).collect()
    }

    /// Terminate end events directly inside a subprocess (pm4py's
    /// `get_termination_events_of_subprocess`).
    pub fn termination_events_of_subprocess(&self, activity_id: &str) -> Vec<NodeId> {
        self.nodes_where(|n| {
            n.process == activity_id && n.kind == NodeKind::EndEvent(EndTrigger::Terminate)
        })
    }

    /// Intermediate events directly inside a subprocess that no boundary
    /// event of the subprocess shares a name with (pm4py's
    /// `get_termination_events_of_subprocess_for_pnet`).
    pub fn termination_events_for_petri_net(&self, activity_id: &str) -> Vec<NodeId> {
        let boundary: Vec<&str> = self
            .boundary_events_of(activity_id)
            .into_iter()
            .map(|b| self.node(b).name.as_str())
            .collect();
        self.nodes_where(|n| {
            n.process == activity_id
                && matches!(
                    n.kind,
                    NodeKind::IntermediateCatchEvent(_) | NodeKind::IntermediateThrowEvent(_)
                )
                && !boundary.contains(&n.name.as_str())
        })
    }

    /// Start events directly inside a subprocess (pm4py's
    /// `get_start_events_of_subprocess`).
    pub fn start_events_of_subprocess(&self, activity_id: &str) -> Vec<NodeId> {
        self.nodes_where(|n| n.process == activity_id && n.kind.is_start_event())
    }

    /// End events directly inside a subprocess (pm4py's
    /// `get_end_events_of_subprocess`).
    pub fn end_events_of_subprocess(&self, activity_id: &str) -> Vec<NodeId> {
        self.nodes_where(|n| n.process == activity_id && n.kind.is_end_event())
    }

    /// End events inside subprocesses that are not normal end events
    /// (pm4py's `bpmn_graph_end_events_as_throw_events`).
    pub fn end_events_as_throw_events(&self) -> Vec<NodeId> {
        self.nodes_where(|n| {
            n.process != self.process_id
                && n.kind.is_end_event()
                && n.kind != NodeKind::EndEvent(EndTrigger::Normal)
        })
    }

    /// Renames tasks whose name is a key of `labels` (pm4py's
    /// `bpmn.util.label_replacing.apply`, used by
    /// `pm4py.replace_activity_labels`).
    pub fn replace_task_labels(&mut self, labels: &BTreeMap<String, String>) {
        for n in self.nodes.iter_mut().flatten() {
            if let Some(new) = labels.get(&n.name).filter(|_| n.kind.is_task()) {
                n.name.clone_from(new);
            }
        }
    }

    /// Breadth-first levels from the start events (pm4py's `bfs_bpmn`):
    /// start events get level 0, and each step reaches the not yet visited
    /// successors that are not end events. Nodes never reached, end events
    /// among them, share the level after the last one.
    pub fn bfs_levels(&self) -> BTreeMap<NodeId, usize> {
        let mut level = 0;
        let mut bfs: BTreeMap<NodeId, usize> = self
            .nodes()
            .filter(|(_, n)| n.kind.is_start_event())
            .map(|(i, _)| (i, level))
            .collect();
        loop {
            level += 1;
            let to_visit: BTreeSet<NodeId> = self
                .flows()
                .filter(|(_, f)| {
                    bfs.contains_key(&f.source)
                        && !bfs.contains_key(&f.target)
                        && !self.node(f.target).kind.is_end_event()
                })
                .map(|(_, f)| f.target)
                .collect();
            if to_visit.is_empty() {
                break;
            }
            for n in to_visit {
                bfs.insert(n, level);
            }
        }
        level += 1;
        for n in self.node_ids() {
            bfs.entry(n).or_insert(level);
        }
        bfs
    }

    /// Nodes and flows sorted by their [`Bpmn::bfs_levels`] (pm4py's
    /// `get_sorted_nodes_edges`). Ties keep id order.
    pub fn sorted_nodes_and_flows(&self) -> (Vec<NodeId>, Vec<FlowId>) {
        let bfs = self.bfs_levels();
        let mut nodes: Vec<NodeId> = self.node_ids().collect();
        nodes.sort_by_key(|n| bfs[n]);
        let mut flows: Vec<FlowId> = self.flows().map(|(i, _)| i).collect();
        flows.sort_by_key(|f| {
            let f = self.flow(*f);
            (bfs[&f.source], bfs[&f.target])
        });
        (nodes, flows)
    }

    /// The activities of the tasks, as labels.
    pub fn task_labels(&self) -> BTreeSet<Label> {
        self.nodes()
            .filter(|(_, n)| n.kind.is_task() && !n.name.is_empty())
            .map(|(_, n)| Label::from(n.name.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests;
