//! BPMN conversions against pm4py's (`fixtures/golden/bpmn`).
//!
//! pm4py names the BPMN nodes it creates with random UUIDs, so converted
//! diagrams are compared up to isomorphism. See `tools/golden/cases/bpmn.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_golden::{cases, golden};
use ichnos_model::bpmn::{
    CatchTrigger, EndTrigger, FlowKind, GatewayDirection, GatewayKind, NodeKind, StartTrigger,
    TaskKind, ThrowTrigger,
};
use ichnos_model::conversion::BpmnToPetriOptions;
use ichnos_model::{AcceptingPetriNet, Bpmn, ProcessTree};
use serde_json::{Value, json};

mod common;
use common::{build_accepting, str_field};

/// A directed multigraph with labelled nodes and edges.
#[derive(Debug)]
struct Graph {
    labels: Vec<String>,
    /// `(source, target) -> {edge label: count}`.
    edges: BTreeMap<(usize, usize), BTreeMap<String, usize>>,
}

impl Graph {
    fn edge_count(&self) -> usize {
        self.edges.values().flat_map(|m| m.values()).sum()
    }
}

fn node_label(class: &str, name: &str, direction: Option<&str>) -> String {
    format!("{class}|{name}|{}", direction.unwrap_or("-"))
}

fn canonical_graph(v: &Value) -> Graph {
    let mut index = BTreeMap::new();
    let mut labels = Vec::new();
    for n in v["nodes"].as_array().expect("nodes") {
        index.insert(str_field(n, "key").to_owned(), labels.len());
        labels.push(node_label(
            str_field(n, "class"),
            str_field(n, "name"),
            n["direction"].as_str(),
        ));
    }
    let mut edges: BTreeMap<(usize, usize), BTreeMap<String, usize>> = BTreeMap::new();
    for f in v["flows"].as_array().expect("flows") {
        let s = index[f[0].as_str().expect("source key")];
        let t = index[f[1].as_str().expect("target key")];
        let class = f[2].as_str().expect("flow class").to_owned();
        *edges.entry((s, t)).or_default().entry(class).or_default() += 1;
    }
    Graph { labels, edges }
}

fn flow_class(kind: FlowKind) -> &'static str {
    match kind {
        FlowKind::Sequence => "SequenceFlow",
        FlowKind::Message => "MessageFlow",
        FlowKind::Association => "Association",
    }
}

fn bpmn_graph(b: &Bpmn) -> Graph {
    let mut index = BTreeMap::new();
    let mut labels = Vec::new();
    for (id, n) in b.nodes() {
        index.insert(id, labels.len());
        labels.push(node_label(
            n.kind.class_name(),
            &n.name,
            n.kind.gateway_direction().map(GatewayDirection::as_str),
        ));
    }
    let mut edges: BTreeMap<(usize, usize), BTreeMap<String, usize>> = BTreeMap::new();
    for (_, f) in b.flows() {
        *edges
            .entry((index[&f.source()], index[&f.target()]))
            .or_default()
            .entry(flow_class(f.kind).to_owned())
            .or_default() += 1;
    }
    Graph { labels, edges }
}

/// Colour refinement on both graphs together, so colours are comparable.
fn refine(a: &Graph, b: &Graph) -> (Vec<usize>, Vec<usize>) {
    let mut colors: [Vec<String>; 2] = [a.labels.clone(), b.labels.clone()];
    let mut classes = 0;
    loop {
        let sigs: Vec<Vec<String>> = [a, b]
            .iter()
            .zip(&colors)
            .map(|(g, c)| {
                (0..g.labels.len())
                    .map(|v| {
                        let mut out: Vec<(&String, &String, usize)> = Vec::new();
                        let mut inc: Vec<(&String, &String, usize)> = Vec::new();
                        for ((s, t), m) in &g.edges {
                            for (cls, n) in m {
                                if *s == v {
                                    out.push((cls, &c[*t], *n));
                                }
                                if *t == v {
                                    inc.push((cls, &c[*s], *n));
                                }
                            }
                        }
                        out.sort();
                        inc.sort();
                        format!("{}{out:?}{inc:?}", c[v])
                    })
                    .collect()
            })
            .collect();
        let ranks: BTreeMap<&String, usize> = sigs
            .iter()
            .flatten()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, s)| (s, i))
            .collect();
        let new: Vec<Vec<usize>> = sigs
            .iter()
            .map(|s| s.iter().map(|x| ranks[x]).collect())
            .collect();
        if ranks.len() == classes {
            return (new[0].clone(), new[1].clone());
        }
        classes = ranks.len();
        colors = [
            new[0].iter().map(usize::to_string).collect(),
            new[1].iter().map(usize::to_string).collect(),
        ];
    }
}

/// Returns `true` if the graphs are isomorphic, labels and edge
/// multiplicities included.
fn isomorphic(a: &Graph, b: &Graph) -> bool {
    if a.labels.len() != b.labels.len() || a.edge_count() != b.edge_count() {
        return false;
    }
    let (ca, cb) = refine(a, b);
    let histogram = |c: &[usize]| {
        let mut h = BTreeMap::new();
        for &x in c {
            *h.entry(x).or_insert(0) += 1;
        }
        h
    };
    if histogram(&ca) != histogram(&cb) {
        return false;
    }
    let mut order: Vec<usize> = (0..a.labels.len()).collect();
    let size = histogram(&ca);
    order.sort_by_key(|&v| (size[&ca[v]], ca[v]));
    let mut m = Matcher {
        a,
        b,
        ca: &ca,
        cb: &cb,
        order: &order,
        map: vec![usize::MAX; a.labels.len()],
        used: vec![false; b.labels.len()],
    };
    m.extend(0)
}

/// Backtracking search for a colour-preserving bijection from `a` to `b`.
struct Matcher<'a> {
    a: &'a Graph,
    b: &'a Graph,
    ca: &'a [usize],
    cb: &'a [usize],
    order: &'a [usize],
    map: Vec<usize>,
    used: Vec<bool>,
}

impl Matcher<'_> {
    fn edge(g: &Graph, s: usize, t: usize) -> Option<&BTreeMap<String, usize>> {
        g.edges.get(&(s, t))
    }

    /// Maps `order[i..]`, given the mapping of `order[..i]`.
    fn extend(&mut self, i: usize) -> bool {
        let Some(&v) = self.order.get(i) else {
            return true;
        };
        let (a, b) = (self.a, self.b);
        for w in 0..b.labels.len() {
            if self.used[w] || self.cb[w] != self.ca[v] {
                continue;
            }
            let fits = Self::edge(a, v, v) == Self::edge(b, w, w)
                && self.order[..i].iter().all(|&u| {
                    let fu = self.map[u];
                    Self::edge(a, v, u) == Self::edge(b, w, fu)
                        && Self::edge(a, u, v) == Self::edge(b, fu, w)
                });
            if !fits {
                continue;
            }
            self.map[v] = w;
            self.used[w] = true;
            if self.extend(i + 1) {
                return true;
            }
            self.used[w] = false;
        }
        false
    }
}

#[test]
fn isomorphism_check_tells_graphs_apart() {
    let t = |s: &str| ProcessTree::parse(s).unwrap().to_bpmn().unwrap();
    let a = bpmn_graph(&t("->( 'a', X( 'b', 'c' ) )"));
    assert!(isomorphic(&a, &bpmn_graph(&t("->( 'a', X( 'c', 'b' ) )"))));
    assert!(!isomorphic(&a, &bpmn_graph(&t("->( 'a', +( 'b', 'c' ) )"))));
    assert!(!isomorphic(&a, &bpmn_graph(&t("->( X( 'b', 'c' ), 'a' )"))));
}

#[test]
fn trees_to_bpmn_match_pm4py() {
    let ids: Vec<String> = cases("bpmn")
        .into_iter()
        .filter(|id| id.starts_with("tree-to-bpmn-"))
        .collect();
    assert_eq!(ids.len(), 7, "expected 7 tree goldens, found {ids:?}");
    for id in ids {
        let g = golden("bpmn", &id);
        let tree = ProcessTree::parse(str_field(g.expected_at("/model"), "tree")).expect("tree");
        let actual = bpmn_graph(&tree.to_bpmn().expect("no interleaving"));
        let expected = canonical_graph(g.expected_at("/bpmn"));
        assert!(
            isomorphic(&actual, &expected),
            "{id}: diagrams differ\nichnos: {actual:?}\npm4py: {expected:?}"
        );
    }
}

#[test]
fn petri_nets_to_bpmn_match_pm4py() {
    let ids: Vec<String> = cases("bpmn")
        .into_iter()
        .filter(|id| id.starts_with("petri-to-bpmn-"))
        .collect();
    assert_eq!(ids.len(), 9, "expected 9 net goldens, found {ids:?}");
    for id in ids {
        let g = golden("bpmn", &id);
        let apn: AcceptingPetriNet = build_accepting(g.expected_at("/model"));
        let actual = bpmn_graph(&apn.to_bpmn());
        let expected = canonical_graph(g.expected_at("/bpmn"));
        assert!(
            isomorphic(&actual, &expected),
            "{id}: diagrams differ ({} vs {} nodes, {} vs {} flows)",
            actual.labels.len(),
            expected.labels.len(),
            actual.edge_count(),
            expected.edge_count()
        );
    }
}

fn node_kind(n: &Value) -> NodeKind {
    let direction = match n["direction"].as_str() {
        None | Some("Unspecified") => GatewayDirection::Unspecified,
        Some("Diverging") => GatewayDirection::Diverging,
        Some("Converging") => GatewayDirection::Converging,
        Some(other) => panic!("unknown direction {other}"),
    };
    let start = |trigger| NodeKind::StartEvent {
        trigger,
        is_interrupting: false,
        parallel_multiple: false,
    };
    let boundary = |trigger| NodeKind::BoundaryEvent {
        trigger,
        activity: n["activity"].as_str().map(str::to_owned),
    };
    match str_field(n, "class") {
        "StartEvent" => start(StartTrigger::Plain),
        "NormalStartEvent" => start(StartTrigger::Normal),
        "MessageStartEvent" => start(StartTrigger::Message),
        "IntermediateCatchEvent" => NodeKind::IntermediateCatchEvent(CatchTrigger::Plain),
        "MessageIntermediateCatchEvent" => NodeKind::IntermediateCatchEvent(CatchTrigger::Message),
        "ErrorIntermediateCatchEvent" => NodeKind::IntermediateCatchEvent(CatchTrigger::Error),
        "CancelIntermediateCatchEvent" => NodeKind::IntermediateCatchEvent(CatchTrigger::Cancel),
        "IntermediateThrowEvent" => NodeKind::IntermediateThrowEvent(ThrowTrigger::Plain),
        "NormalIntermediateThrowEvent" => NodeKind::IntermediateThrowEvent(ThrowTrigger::Normal),
        "MessageIntermediateThrowEvent" => NodeKind::IntermediateThrowEvent(ThrowTrigger::Message),
        "BoundaryEvent" => boundary(CatchTrigger::Plain),
        "MessageBoundaryEvent" => boundary(CatchTrigger::Message),
        "ErrorBoundaryEvent" => boundary(CatchTrigger::Error),
        "CancelBoundaryEvent" => boundary(CatchTrigger::Cancel),
        "EndEvent" => NodeKind::EndEvent(EndTrigger::Plain),
        "NormalEndEvent" => NodeKind::EndEvent(EndTrigger::Normal),
        "MessageEndEvent" => NodeKind::EndEvent(EndTrigger::Message),
        "TerminateEndEvent" => NodeKind::EndEvent(EndTrigger::Terminate),
        "ErrorEndEvent" => NodeKind::EndEvent(EndTrigger::Error),
        "CancelEndEvent" => NodeKind::EndEvent(EndTrigger::Cancel),
        "Task" => NodeKind::Task(TaskKind::Plain),
        "UserTask" => NodeKind::Task(TaskKind::User),
        "SendTask" => NodeKind::Task(TaskKind::Send),
        "SubProcess" => NodeKind::SubProcess {
            depth: n["depth"]
                .as_u64()
                .map(|d| u32::try_from(d).expect("depth")),
        },
        "ExclusiveGateway" => NodeKind::gateway(GatewayKind::Exclusive, direction),
        "ParallelGateway" => NodeKind::gateway(GatewayKind::Parallel, direction),
        "InclusiveGateway" => NodeKind::gateway(GatewayKind::Inclusive, direction),
        "EventBasedGateway" => NodeKind::gateway(GatewayKind::EventBased, direction),
        "TextAnnotation" => NodeKind::TextAnnotation { text: None },
        "Participant" => NodeKind::Participant { process_ref: None },
        "Collaboration" => NodeKind::Collaboration,
        other => panic!("unknown BPMN class {other}"),
    }
}

fn build_bpmn(v: &Value) -> Bpmn {
    let mut b = Bpmn::new(str_field(v, "process_id"));
    let mut ids = BTreeMap::new();
    for n in v["nodes"].as_array().expect("nodes") {
        let id = b.add_node_with_id(str_field(n, "id"), node_kind(n), str_field(n, "name"));
        b.node_mut(id).process = str_field(n, "process").to_owned();
        ids.insert(str_field(n, "id").to_owned(), id);
    }
    for f in v["flows"].as_array().expect("flows") {
        let kind = match str_field(f, "class") {
            "SequenceFlow" => FlowKind::Sequence,
            "MessageFlow" => FlowKind::Message,
            "Association" => FlowKind::Association,
            other => panic!("unknown flow class {other}"),
        };
        let id = b
            .add_flow_with(
                kind,
                str_field(f, "id"),
                str_field(f, "name"),
                ids[str_field(f, "source")],
                ids[str_field(f, "target")],
            )
            .expect("both ends exist");
        b.flow_mut(id).process = str_field(f, "process").to_owned();
    }
    b
}

/// `[id]8-4-4-4-12` lowercase hex, as pm4py's `uuid4` names.
fn is_uuid(name: &str) -> bool {
    let s = name.strip_prefix("id").unwrap_or(name);
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')))
}

/// A transition of a converted net, as the golden file describes it.
struct Described {
    name: Option<String>,
    label: Option<String>,
    preset: Vec<String>,
    postset: Vec<String>,
}

impl Described {
    fn key(&self) -> (&str, &str, &[String], &[String]) {
        (
            self.name.as_deref().unwrap_or(""),
            self.label.as_deref().unwrap_or(""),
            &self.preset,
            &self.postset,
        )
    }
}

/// The golden form of a converted net. Transitions named like a UUID, and
/// ichnos's helper transitions, which pm4py names with UUIDs, get a `null`
/// name.
fn describe_net(apn: &AcceptingPetriNet, node_ids: &BTreeSet<String>) -> Value {
    let net = &apn.net;
    let mut places: Vec<&str> = net.places().map(|(_, p)| p.name.as_str()).collect();
    places.sort_unstable();
    let mut transitions: Vec<Described> = net
        .transitions()
        .map(|(t, tr)| {
            let named = !is_uuid(&tr.name)
                && (node_ids.contains(&tr.name)
                    || tr.name.starts_with("sfl_")
                    || tr.name.starts_with("tfl_"));
            let mut preset: Vec<String> =
                net.preset(t).map(|p| net.place(p).name.clone()).collect();
            let mut postset: Vec<String> =
                net.postset(t).map(|p| net.place(p).name.clone()).collect();
            preset.sort_unstable();
            postset.sort_unstable();
            Described {
                name: named.then(|| tr.name.clone()),
                label: tr.label.as_ref().map(|l| l.as_str().to_owned()),
                preset,
                postset,
            }
        })
        .collect();
    // pm4py's sort key: (name or "", label or "", preset, postset).
    transitions.sort_by(|x, y| x.key().cmp(&y.key()));
    let transitions: Vec<Value> = transitions
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "label": t.label,
                "preset": t.preset,
                "postset": t.postset,
            })
        })
        .collect();
    let marking = |m: &ichnos_model::Marking| -> BTreeMap<String, u32> {
        m.iter()
            .map(|(p, n)| (net.place(p).name.clone(), n))
            .collect()
    };
    json!({
        "places": places,
        "transitions": transitions,
        "initial_marking": marking(&apn.initial_marking),
        "final_marking": marking(&apn.final_marking),
    })
}

#[test]
fn bpmn_to_petri_nets_match_pm4py() {
    let ids: Vec<String> = cases("bpmn")
        .into_iter()
        .filter(|id| id.starts_with("bpmn-to-petri-"))
        .collect();
    assert_eq!(ids.len(), 6, "expected 6 BPMN goldens, found {ids:?}");
    for id in ids {
        let g = golden("bpmn", &id);
        let b = build_bpmn(g.expected_at("/model"));
        let node_ids: BTreeSet<String> = b.nodes().map(|(_, n)| n.id.clone()).collect();
        for (reduce, key) in [(false, "/petri_net_unreduced"), (true, "/petri_net")] {
            let options = BpmnToPetriOptions {
                use_id: false,
                reduce,
            };
            let actual = describe_net(&b.to_petri_net(options).net, &node_ids);
            assert_eq!(&actual, g.expected_at(key), "{id} {key}");
        }
    }
}
