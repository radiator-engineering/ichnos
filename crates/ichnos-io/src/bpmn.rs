//! BPMN 2.0 XML diagrams, ported from pm4py's `objects/bpmn/importer`
//! (`lxml` variant) and `objects/bpmn/exporter` (`etree` variant).
//!
//! A [`BpmnDocument`] holds the [`Bpmn`] diagram and the layout that the
//! diagram itself does not store: node bounds and flow waypoints.
//!
//! The reader follows pm4py's importer. It matches elements by the end of
//! their lower-case tag name, so `serviceTask` is a task and
//! `adHocSubProcess` a subprocess, and it ignores elements it does not know.
//! Event types come from the first `*EventDefinition` child. A flow needs a
//! source and a target, from its `sourceRef` and `targetRef` or from the
//! `incoming` and `outgoing` children of its nodes. The diagram's process
//! id and name come from the last `process` element.
//!
//! The writer follows pm4py's exporter, which prefixes `id` to the ids of
//! processes and flows (not nodes) and writes every event without its event
//! definition, except intermediate catch events, which get a message event
//! definition. Reading a written diagram back therefore changes those ids
//! and some event types, as it does in pm4py.

use crate::{
    Result,
    model_xml::{self as xml, Element, invalid},
};
use ichnos_model::Bpmn;
use ichnos_model::bpmn::{
    CatchTrigger, EndTrigger, FlowId, FlowKind, GatewayDirection, GatewayKind, NodeId, NodeKind,
    StartTrigger, TaskKind, ThrowTrigger,
};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

const FORMAT: &str = "BPMN";

/// The position and size of a node's shape (pm4py's `BPMNNodeLayout`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// pm4py's default layout of a node: at the origin, 100 by 100.
impl Default for Bounds {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }
}

/// A BPMN diagram with its layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BpmnDocument {
    /// The diagram.
    pub model: Bpmn,
    /// Node bounds. The writer uses [`Bounds::default`] for a node
    /// without bounds.
    pub bounds: BTreeMap<NodeId, Bounds>,
    /// Flow waypoints. The writer uses two points at the origin, pm4py's
    /// default, for a flow without an entry.
    pub waypoints: BTreeMap<FlowId, Vec<(f64, f64)>>,
}

impl From<Bpmn> for BpmnDocument {
    fn from(model: Bpmn) -> Self {
        Self {
            model,
            ..Default::default()
        }
    }
}

/// Limits for reading BPMN XML.
#[derive(Debug, Clone)]
pub struct BpmnReadOptions {
    /// Maximum XML nesting.
    pub max_depth: usize,
    /// Maximum XML elements.
    pub max_nodes: usize,
}

impl Default for BpmnReadOptions {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 1_000_000,
        }
    }
}

/// Options for writing BPMN XML.
#[derive(Debug, Clone)]
pub struct BpmnWriteOptions {
    /// Indent the XML.
    pub indent: bool,
    /// Write the diagram plane with shapes and edges (pm4py's
    /// `enble_bpmn_plane_exporting`).
    pub plane: bool,
    /// Write `incoming` and `outgoing` children for each node (pm4py's
    /// `enable_incoming_outgoing_exporting`).
    pub incoming_outgoing: bool,
}

impl Default for BpmnWriteOptions {
    fn default() -> Self {
        Self {
            indent: true,
            plane: true,
            incoming_outgoing: true,
        }
    }
}

/// Reads a BPMN 2.0 XML file (pm4py's `read_bpmn`).
pub fn read_bpmn(path: impl AsRef<Path>, options: &BpmnReadOptions) -> Result<BpmnDocument> {
    read_bpmn_from_reader(BufReader::new(File::open(path)?), options)
}

/// Reads BPMN 2.0 XML from a stream.
///
/// Fails if two nodes share an id, if a flow names a node that is not in
/// the diagram, if a flow reference is empty, or if a bounds or waypoint
/// coordinate is missing or not a number. pm4py fails in each of these
/// cases too, except for repeated node ids, where it keeps the first node
/// in the diagram but attaches flows and bounds to the last one.
pub fn read_bpmn_from_reader(
    input: impl BufRead,
    options: &BpmnReadOptions,
) -> Result<BpmnDocument> {
    let root = xml::read(input, FORMAT, options.max_depth, options.max_nodes)?;
    let mut parser = Parser::default();
    parser.visit(&root, &Context::default(), 0)?;
    parser.finish()
}

/// What a descendant inherits from its ancestors in pm4py's recursive
/// `parse_element`.
#[derive(Debug, Clone, Default)]
struct Context {
    /// The enclosing process or subprocess.
    process: Option<String>,
    /// The enclosing node, for `incoming` and `outgoing` children.
    node: Option<String>,
    /// The node whose shape encloses this element, for its bounds.
    shape: Option<String>,
    /// The flow whose edge encloses this element, for its waypoints.
    edge: Option<String>,
    /// The innermost enclosing collaboration, for message flows.
    collaboration: Option<String>,
}

/// One end of a flow, from the flow element or from a node's `incoming`
/// or `outgoing` child.
#[derive(Debug, Clone)]
struct End {
    node: String,
    process: Option<String>,
    tag: String,
    name: String,
    collaboration: Option<String>,
}

#[derive(Default)]
struct Parser {
    process_id: Option<String>,
    name: String,
    nodes: Vec<(String, String, Option<String>, NodeKind)>,
    node_index: HashMap<String, usize>,
    incoming: HashMap<String, End>,
    outgoing: HashMap<String, End>,
    flow_order: Vec<String>,
    bounds: HashMap<String, Bounds>,
    waypoints: HashMap<String, Vec<(f64, f64)>>,
}

/// pm4py's name handling: line breaks are dropped, or for events replaced
/// by spaces.
fn name(el: &Element, event: bool) -> String {
    let with = if event { " " } else { "" };
    el.attr("name")
        .map(|n| n.replace(['\r', '\n'], with))
        .unwrap_or_default()
}

/// The first event definition child, lower case and without
/// `eventdefinition`, as pm4py's importer compares it.
fn event_type(el: &Element) -> String {
    el.children
        .iter()
        .map(|c| c.name.to_lowercase())
        .find(|t| t.ends_with("eventdefinition"))
        .map(|t| t.replace("eventdefinition", ""))
        .unwrap_or_default()
}

fn direction(el: &Element) -> GatewayDirection {
    match el
        .attr("gatewayDirection")
        .map(str::to_uppercase)
        .as_deref()
    {
        Some("DIVERGING") => GatewayDirection::Diverging,
        Some("CONVERGING") => GatewayDirection::Converging,
        _ => GatewayDirection::Unspecified,
    }
}

fn coordinate(el: &Element, key: &str) -> Result<f64> {
    let value = el.required(key, FORMAT)?;
    value.trim().parse().map_err(|_| {
        invalid(
            FORMAT,
            format!("<{}> {key}={value:?} is not a number", el.name),
        )
    })
}

impl Parser {
    fn add_node(
        &mut self,
        id: &str,
        name: String,
        process: Option<String>,
        kind: NodeKind,
    ) -> Result<()> {
        if self.node_index.contains_key(id) {
            return Err(invalid(FORMAT, format!("two nodes have the id {id:?}")));
        }
        self.node_index.insert(id.to_owned(), self.nodes.len());
        self.nodes.push((id.to_owned(), name, process, kind));
        Ok(())
    }

    fn note_flow(&mut self, id: &str) {
        if !self.incoming.contains_key(id) && !self.outgoing.contains_key(id) {
            self.flow_order.push(id.to_owned());
        }
    }

    fn visit(&mut self, el: &Element, parent: &Context, depth: u32) -> Result<()> {
        let mut ctx = parent.clone();
        let tag = el.name.to_lowercase();
        if depth > 0 && tag.ends_with("collaboration") {
            ctx.collaboration = el.attr("id").map(str::to_owned);
        }
        if let Some(id) = el.attr("id") {
            let process = ctx.process.clone();
            if tag.ends_with("collaboration") {
                self.add_node(
                    id,
                    String::new(),
                    Some(id.to_owned()),
                    NodeKind::Collaboration,
                )?;
            } else if tag.ends_with("participant") {
                let process_ref = el.attr("processRef").map(str::to_owned);
                self.add_node(
                    id,
                    name(el, false),
                    None,
                    NodeKind::Participant { process_ref },
                )?;
            } else if tag.ends_with("textannotation") {
                let mut text = Some(String::new());
                for child in &el.children {
                    text = Some(child.text.clone()).filter(|t| !t.is_empty());
                }
                self.add_node(
                    id,
                    String::new(),
                    process,
                    NodeKind::TextAnnotation { text },
                )?;
            } else if tag.ends_with("subprocess") {
                let kind = NodeKind::SubProcess { depth: Some(depth) };
                self.add_node(id, name(el, false), process, kind)?;
                ctx.node = Some(id.to_owned());
                ctx.process = Some(id.to_owned());
            } else if tag.ends_with("process") {
                ctx.process = Some(id.to_owned());
                self.process_id = Some(id.to_owned());
                self.name = name(el, false);
            } else if tag.ends_with("shape") {
                ctx.shape = el.attr("bpmnElement").map(str::to_owned);
            } else if tag.ends_with("task") {
                let kind = if tag.ends_with("usertask") {
                    TaskKind::User
                } else if tag.ends_with("sendtask") {
                    TaskKind::Send
                } else {
                    TaskKind::Plain
                };
                self.add_node(id, name(el, false), process, NodeKind::Task(kind))?;
                ctx.node = Some(id.to_owned());
            } else if let Some(kind) = event_kind(el, &tag) {
                self.add_node(id, name(el, true), process, kind)?;
                ctx.node = Some(id.to_owned());
            } else if tag.ends_with("edge") {
                ctx.edge = el.attr("bpmnElement").map(str::to_owned);
            } else if let Some(kind) = gateway_kind(&tag) {
                let kind = NodeKind::gateway(kind, direction(el));
                self.add_node(id, name(el, false), process, kind)?;
                ctx.node = Some(id.to_owned());
            } else if (tag.ends_with("sequenceflow")
                || tag.ends_with("messageflow")
                || tag.ends_with("association"))
                && let (Some(source), Some(target)) = (el.attr("sourceRef"), el.attr("targetRef"))
            {
                self.note_flow(id);
                let end = |node: &str| End {
                    node: node.to_owned(),
                    process: ctx.process.clone(),
                    tag: tag.clone(),
                    name: name(el, false),
                    collaboration: ctx.collaboration.clone(),
                };
                self.incoming.insert(id.to_owned(), end(target));
                self.outgoing.insert(id.to_owned(), end(source));
            }
        } else if tag.ends_with("incoming") || tag.ends_with("outgoing") {
            if let Some(node) = &ctx.node {
                let flow = el.text.trim();
                if flow.is_empty() {
                    return Err(invalid(FORMAT, format!("empty <{}>", el.name)));
                }
                let end = End {
                    node: node.clone(),
                    process: ctx.process.clone(),
                    tag: tag.clone(),
                    name: name(el, false),
                    collaboration: ctx.collaboration.clone(),
                };
                self.note_flow(flow);
                let ends = if tag.ends_with("incoming") {
                    &mut self.incoming
                } else {
                    &mut self.outgoing
                };
                ends.entry(flow.to_owned()).or_insert(end);
            }
        } else if tag.ends_with("waypoint") {
            if let Some(flow) = &ctx.edge {
                let point = (coordinate(el, "x")?, coordinate(el, "y")?);
                self.waypoints.entry(flow.clone()).or_default().push(point);
            }
        } else if tag.ends_with("label") {
            ctx.shape = None;
        } else if tag.ends_with("bounds")
            && let Some(node) = &ctx.shape
        {
            let bounds = Bounds {
                x: coordinate(el, "x")?,
                y: coordinate(el, "y")?,
                width: coordinate(el, "width")?,
                height: coordinate(el, "height")?,
            };
            self.bounds.insert(node.clone(), bounds);
        }
        for child in &el.children {
            self.visit(child, &ctx, depth + 1)?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<BpmnDocument> {
        let mut model = Bpmn::new(self.process_id.take().unwrap_or_default());
        model.name = std::mem::take(&mut self.name);
        let mut ids = HashMap::new();
        let mut bounds = BTreeMap::new();
        for (id, name, process, kind) in std::mem::take(&mut self.nodes) {
            let n = model.add_node_with_id(&id, kind, name);
            model.node_mut(n).process = process.unwrap_or_default();
            if let Some(b) = self.bounds.get(&id) {
                bounds.insert(n, *b);
            }
            ids.insert(id, n);
        }
        let node = |end: &End| {
            ids.get(&end.node).copied().ok_or_else(|| {
                invalid(
                    FORMAT,
                    format!(
                        "a flow names the node {:?}, which is not in the diagram",
                        end.node
                    ),
                )
            })
        };
        let mut waypoints = BTreeMap::new();
        for id in &self.flow_order {
            let (Some(out), Some(inc)) = (self.outgoing.get(id), self.incoming.get(id)) else {
                continue;
            };
            let (kind, process) = if out.tag.ends_with("messageflow") {
                (FlowKind::Message, &out.collaboration)
            } else if out.tag.ends_with("association") {
                (FlowKind::Association, &out.process)
            } else {
                (FlowKind::Sequence, &out.process)
            };
            let f = model
                .add_flow_with(kind, id, &out.name, node(out)?, node(inc)?)
                .expect("both ends are in the diagram");
            model.flow_mut(f).process = process.clone().unwrap_or_default();
            waypoints.insert(f, self.waypoints.remove(id).unwrap_or_default());
        }
        Ok(BpmnDocument {
            model,
            bounds,
            waypoints,
        })
    }
}

fn event_kind(el: &Element, tag: &str) -> Option<NodeKind> {
    let t = event_type(el);
    let catch = || {
        if t.ends_with("message") {
            CatchTrigger::Message
        } else if t.ends_with("error") {
            CatchTrigger::Error
        } else if t.ends_with("cancel") {
            CatchTrigger::Cancel
        } else {
            CatchTrigger::Plain
        }
    };
    Some(if tag.ends_with("startevent") {
        NodeKind::StartEvent {
            trigger: if t.ends_with("message") {
                StartTrigger::Message
            } else {
                StartTrigger::Normal
            },
            is_interrupting: false,
            parallel_multiple: false,
        }
    } else if tag.ends_with("endevent") {
        NodeKind::EndEvent(if t.ends_with("message") {
            EndTrigger::Message
        } else if t.ends_with("terminate") {
            EndTrigger::Terminate
        } else if t.ends_with("error") {
            EndTrigger::Error
        } else if t.ends_with("cancel") {
            EndTrigger::Cancel
        } else {
            EndTrigger::Normal
        })
    } else if tag.ends_with("intermediatecatchevent") {
        NodeKind::IntermediateCatchEvent(catch())
    } else if tag.ends_with("intermediatethrowevent") {
        NodeKind::IntermediateThrowEvent(if t.ends_with("message") {
            ThrowTrigger::Message
        } else {
            ThrowTrigger::Normal
        })
    } else if tag.ends_with("boundaryevent") {
        NodeKind::BoundaryEvent {
            trigger: catch(),
            activity: el.attr("attachedToRef").map(str::to_owned),
        }
    } else {
        return None;
    })
}

fn gateway_kind(tag: &str) -> Option<GatewayKind> {
    Some(if tag.ends_with("exclusivegateway") {
        GatewayKind::Exclusive
    } else if tag.ends_with("parallelgateway") {
        GatewayKind::Parallel
    } else if tag.ends_with("inclusivegateway") {
        GatewayKind::Inclusive
    } else if tag.ends_with("eventbasedgateway") {
        GatewayKind::EventBased
    } else {
        return None;
    })
}

/// The processes of a document as pm4py's exporter lays them out.
struct Layout {
    /// The `bpmnElement` of the plane: the collaboration's id, or `id` and
    /// the only process.
    plane: String,
    /// The collaboration node, when the diagram has several processes.
    collaboration: Option<NodeId>,
    /// The processes, each written as a `process` element unless it is
    /// the plane, in first-use order.
    processes: Vec<String>,
}

fn layout(b: &Bpmn) -> Result<Layout> {
    fn add(processes: &mut Vec<String>, p: &str) {
        if !processes.iter().any(|q| q == p) {
            processes.push(p.to_owned());
        }
    }
    let mut processes: Vec<String> = Vec::new();
    for (_, n) in b.nodes() {
        add(&mut processes, &n.process);
    }
    for (_, f) in b.flows() {
        add(&mut processes, &f.process);
    }
    if processes.len() <= 1 {
        let only = processes
            .first()
            .cloned()
            .unwrap_or_else(|| b.process_id.clone());
        return Ok(Layout {
            plane: format!("id{only}"),
            collaboration: None,
            processes: vec![only],
        });
    }
    let collaboration = b
        .nodes()
        .find(|(_, n)| n.kind == NodeKind::Collaboration)
        .map(|(id, _)| id)
        .ok_or_else(|| {
            invalid(
                FORMAT,
                "a diagram with several processes needs a collaboration node",
            )
        })?;
    for (_, n) in b.nodes() {
        if let NodeKind::Participant { process_ref } = &n.kind {
            let process_ref = process_ref
                .as_deref()
                .ok_or_else(|| invalid(FORMAT, format!("participant {:?} has no process", n.id)))?;
            add(&mut processes, process_ref);
        }
    }
    Ok(Layout {
        plane: b.node(collaboration).id.clone(),
        collaboration: Some(collaboration),
        processes,
    })
}

/// Writes a BPMN 2.0 XML file (pm4py's `write_bpmn` with
/// `auto_layout=False`).
pub fn write_bpmn(
    document: &BpmnDocument,
    path: impl AsRef<Path>,
    options: &BpmnWriteOptions,
) -> Result<()> {
    layout(&document.model)?;
    write_bpmn_to_writer(document, BufWriter::new(File::create(path)?), options)
}

/// Writes BPMN 2.0 XML to a stream.
///
/// Nodes and flows are written in id order, and processes in the order
/// their nodes and flows first use them; pm4py's order depends on hashing.
/// A node without a process gets no `process` element. The diagram and
/// plane ids are fixed, where pm4py makes random ones.
///
/// Fails if the nodes and flows use several processes and the diagram has
/// no collaboration node, or if a participant has no process reference;
/// pm4py fails in both cases too.
pub fn write_bpmn_to_writer(
    document: &BpmnDocument,
    output: impl Write,
    options: &BpmnWriteOptions,
) -> Result<()> {
    let b = &document.model;
    let layout = layout(b)?;
    let mut w = xml::writer(output, options.indent)?;
    xml::start(
        &mut w,
        "bpmn:definitions",
        &[
            ("xmlns:bpmn", "http://www.omg.org/spec/BPMN/20100524/MODEL"),
            ("xmlns:bpmndi", "http://www.omg.org/spec/BPMN/20100524/DI"),
            ("xmlns:omgdc", "http://www.omg.org/spec/DD/20100524/DC"),
            ("xmlns:omgdi", "http://www.omg.org/spec/DD/20100524/DI"),
            ("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"),
            ("targetNamespace", "http://www.signavio.com/bpmn20"),
            ("typeLanguage", "http://www.w3.org/2001/XMLSchema"),
            ("expressionLanguage", "http://www.w3.org/1999/XPath"),
            ("xmlns:xsd", "http://www.w3.org/2001/XMLSchema"),
        ],
        false,
    )?;
    if let Some(collaboration) = layout.collaboration {
        xml::start(
            &mut w,
            "bpmn:collaboration",
            &[("id", &b.node(collaboration).id)],
            false,
        )?;
        for (_, n) in b.nodes() {
            if let NodeKind::Participant {
                process_ref: Some(process_ref),
            } = &n.kind
            {
                let process_ref = format!("id{process_ref}");
                xml::start(
                    &mut w,
                    "bpmn:participant",
                    &[
                        ("id", &n.id),
                        ("name", &n.name),
                        ("processRef", &process_ref),
                    ],
                    true,
                )?;
            }
        }
        write_process_content(&mut w, document, &layout.plane, options)?;
        xml::end(&mut w, "bpmn:collaboration")?;
    }
    for process in &layout.processes {
        if *process == layout.plane || (process.is_empty() && !holds_elements(b, process)) {
            continue;
        }
        let id = format!("id{process}");
        xml::start(
            &mut w,
            "bpmn:process",
            &[
                ("id", &id),
                ("isClosed", "false"),
                ("isExecutable", "false"),
                ("processType", "None"),
            ],
            false,
        )?;
        write_process_content(&mut w, document, process, options)?;
        xml::end(&mut w, "bpmn:process")?;
    }
    xml::start(
        &mut w,
        "bpmndi:BPMNDiagram",
        &[("id", "id_diagram"), ("name", "diagram")],
        !options.plane,
    )?;
    if options.plane {
        write_plane(&mut w, document, &layout.plane)?;
        xml::end(&mut w, "bpmndi:BPMNDiagram")?;
    }
    xml::end(&mut w, "bpmn:definitions")?;
    w.get_mut().flush()?;
    Ok(())
}

/// Whether a process holds a node or flow that the writer writes as an
/// element.
fn holds_elements(b: &Bpmn, process: &str) -> bool {
    b.nodes()
        .any(|(_, n)| n.process == process && node_tag(&n.kind).is_some())
        || b.flows().any(|(_, f)| f.process == process)
}

/// The element pm4py's exporter writes for a node, or `None` for
/// participants and collaborations, which it does not write there.
fn node_tag(kind: &NodeKind) -> Option<&'static str> {
    Some(match kind {
        NodeKind::TextAnnotation { .. } => "bpmn:textAnnotation",
        NodeKind::StartEvent { .. } => "bpmn:startEvent",
        NodeKind::EndEvent(_) => "bpmn:endEvent",
        NodeKind::IntermediateCatchEvent(_) => "bpmn:intermediateCatchEvent",
        NodeKind::IntermediateThrowEvent(_) => "bpmn:intermediateThrowEvent",
        NodeKind::BoundaryEvent { .. } => "bpmn:boundaryEvent",
        NodeKind::Task(TaskKind::User) => "bpmn:userTask",
        NodeKind::Task(TaskKind::Send) => "bpmn:sendTask",
        NodeKind::Task(TaskKind::Plain) => "bpmn:task",
        NodeKind::SubProcess { .. } => "bpmn:subProcess",
        NodeKind::Gateway { kind, .. } => match kind {
            GatewayKind::Exclusive => "bpmn:exclusiveGateway",
            GatewayKind::Parallel => "bpmn:parallelGateway",
            GatewayKind::Inclusive => "bpmn:inclusiveGateway",
            GatewayKind::EventBased => "bpmn:eventBasedGateway",
        },
        NodeKind::Participant { .. } | NodeKind::Collaboration => return None,
    })
}

fn write_process_content(
    w: &mut quick_xml::Writer<impl Write>,
    document: &BpmnDocument,
    process: &str,
    options: &BpmnWriteOptions,
) -> Result<()> {
    let b = &document.model;
    for (_, n) in b.nodes().filter(|(_, n)| n.process == process) {
        let Some(tag) = node_tag(&n.kind) else {
            continue;
        };
        let flag = |v: bool| if v { "true" } else { "false" };
        let attrs: Vec<(&str, &str)> = match &n.kind {
            NodeKind::TextAnnotation { .. } => vec![("id", &n.id)],
            NodeKind::StartEvent {
                is_interrupting,
                parallel_multiple,
                ..
            } => vec![
                ("id", &n.id),
                ("isInterrupting", flag(*is_interrupting)),
                ("name", &n.name),
                ("parallelMultiple", flag(*parallel_multiple)),
            ],
            NodeKind::Gateway { direction, .. } => vec![
                ("id", &n.id),
                ("gatewayDirection", direction.as_str()),
                ("name", &n.name),
            ],
            _ => vec![("id", &n.id), ("name", &n.name)],
        };
        xml::start(w, tag, &attrs, false)?;
        match &n.kind {
            NodeKind::TextAnnotation { text } => {
                xml::text(w, "bpmn:text", text.as_deref().unwrap_or(""))?;
            }
            NodeKind::IntermediateCatchEvent(_) => {
                xml::start(w, "bpmn:messageEventDefinition", &[], true)?;
            }
            _ => {}
        }
        // pm4py's annotation branch never sets the element it appends these
        // to, so they land on the node written before; ichnos skips them.
        if options.incoming_outgoing && !matches!(n.kind, NodeKind::TextAnnotation { .. }) {
            for f in n.in_flows() {
                xml::text(w, "bpmn:incoming", &format!("id{}", b.flow(*f).id))?;
            }
            for f in n.out_flows() {
                xml::text(w, "bpmn:outgoing", &format!("id{}", b.flow(*f).id))?;
            }
        }
        xml::end(w, tag)?;
    }
    for (_, f) in b.flows().filter(|(_, f)| f.process == process) {
        let id = format!("id{}", f.id);
        let (source, target) = (&b.node(f.source()).id, &b.node(f.target()).id);
        let (tag, attrs): (&str, Vec<(&str, &str)>) = match f.kind {
            FlowKind::Sequence => (
                "bpmn:sequenceFlow",
                vec![
                    ("id", &id),
                    ("name", &f.name),
                    ("sourceRef", source),
                    ("targetRef", target),
                ],
            ),
            FlowKind::Message => (
                "bpmn:messageFlow",
                vec![
                    ("id", &id),
                    ("name", &f.name),
                    ("sourceRef", source),
                    ("targetRef", target),
                ],
            ),
            FlowKind::Association => (
                "bpmn:association",
                vec![("id", &id), ("sourceRef", source), ("targetRef", target)],
            ),
        };
        xml::start(w, tag, &attrs, true)?;
    }
    Ok(())
}

fn write_plane(
    w: &mut quick_xml::Writer<impl Write>,
    document: &BpmnDocument,
    plane: &str,
) -> Result<()> {
    let b = &document.model;
    xml::start(
        w,
        "bpmndi:BPMNPlane",
        &[("bpmnElement", plane), ("id", "id_plane")],
        false,
    )?;
    for (id, n) in b.nodes().filter(|(_, n)| n.process != plane) {
        let shape = format!("{}_gui", n.id);
        xml::start(
            w,
            "bpmndi:BPMNShape",
            &[("bpmnElement", &n.id), ("id", &shape)],
            false,
        )?;
        let bounds = document.bounds.get(&id).copied().unwrap_or_default();
        let [h, wd, x, y] =
            [bounds.height, bounds.width, bounds.x, bounds.y].map(|v| v.to_string());
        xml::start(
            w,
            "omgdc:Bounds",
            &[("height", &h), ("width", &wd), ("x", &x), ("y", &y)],
            true,
        )?;
        xml::end(w, "bpmndi:BPMNShape")?;
    }
    for (id, f) in b.flows() {
        let element = format!("id{}", f.id);
        let edge = format!("id{}_gui", f.id);
        xml::start(
            w,
            "bpmndi:BPMNEdge",
            &[("bpmnElement", &element), ("id", &edge)],
            false,
        )?;
        let default = [(0.0, 0.0); 2];
        let points = document
            .waypoints
            .get(&id)
            .map_or(&default[..], Vec::as_slice);
        for (x, y) in points {
            xml::start(
                w,
                "omgdi:waypoint",
                &[("x", &x.to_string()), ("y", &y.to_string())],
                true,
            )?;
        }
        xml::end(w, "bpmndi:BPMNEdge")?;
    }
    xml::end(w, "bpmndi:BPMNPlane")
}
