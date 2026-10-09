//! BPMN reading and writing against pm4py (`fixtures/golden/io/bpmn-*`);
//! see `tools/golden/cases/io.py`.

use ichnos_golden::{cases, golden};
use ichnos_io::{
    Bounds, BpmnDocument, BpmnWriteOptions, read_bpmn, read_bpmn_from_reader, write_bpmn_to_writer,
};
use ichnos_model::Bpmn;
use ichnos_model::bpmn::{FlowKind, GatewayDirection, GatewayKind, NodeKind};
use serde_json::{Value, json};

fn flow_class(kind: FlowKind) -> &'static str {
    match kind {
        FlowKind::Sequence => "SequenceFlow",
        FlowKind::Message => "MessageFlow",
        FlowKind::Association => "Association",
    }
}

/// The golden form of a diagram: nodes sorted by id, flows by source,
/// target and id.
fn describe(document: &BpmnDocument, with_process_id: bool) -> Value {
    let b = &document.model;
    let mut nodes: Vec<Value> = b
        .nodes()
        .map(|(id, n)| {
            let bounds = document.bounds.get(&id).copied().unwrap_or_default();
            let (interrupting, parallel_multiple) = match n.kind {
                NodeKind::StartEvent {
                    is_interrupting,
                    parallel_multiple,
                    ..
                } => (Some(is_interrupting), Some(parallel_multiple)),
                _ => (None, None),
            };
            json!({
                "id": n.id,
                "name": n.name,
                "class": n.kind.class_name(),
                "direction": n.kind.gateway_direction().map(GatewayDirection::as_str),
                "process": n.process,
                "activity": match &n.kind {
                    NodeKind::BoundaryEvent { activity, .. } => activity.clone(),
                    _ => None,
                },
                "depth": match n.kind {
                    NodeKind::SubProcess { depth } => depth,
                    _ => None,
                },
                "text": match &n.kind {
                    NodeKind::TextAnnotation { text } => text.clone(),
                    _ => None,
                },
                "process_ref": match &n.kind {
                    NodeKind::Participant { process_ref } => process_ref.clone(),
                    _ => None,
                },
                "interrupting": interrupting,
                "parallel_multiple": parallel_multiple,
                "bounds": [bounds.x, bounds.y, bounds.width, bounds.height],
            })
        })
        .collect();
    nodes.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let mut flows: Vec<(String, String, String, Value)> = b
        .flows()
        .map(|(_, f)| {
            let (source, target) = (&b.node(f.source()).id, &b.node(f.target()).id);
            let v = json!({
                "id": f.id, "name": f.name, "class": flow_class(f.kind),
                "source": source, "target": target, "process": f.process,
            });
            (source.clone(), target.clone(), f.id.clone(), v)
        })
        .collect();
    flows.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
    let mut out = json!({
        "name": b.name,
        "nodes": nodes,
        "flows": flows.into_iter().map(|f| f.3).collect::<Vec<_>>(),
    });
    if with_process_id {
        out["process_id"] = json!(b.process_id);
    }
    // serde_json's default float parser can be off by one ulp, so put the
    // value through the same text round trip as the golden.
    serde_json::from_str(&out.to_string()).unwrap()
}

/// Compares node by node and flow by flow, so a failure names the entry.
fn assert_same(actual: &Value, expected: &Value, id: &str) {
    for key in ["nodes", "flows"] {
        let (a, e) = (
            actual[key].as_array().unwrap(),
            expected[key].as_array().unwrap(),
        );
        for (a, e) in a.iter().zip(e) {
            assert_eq!(a, e, "{id}: {key}");
        }
        assert_eq!(a.len(), e.len(), "{id}: number of {key}");
    }
    assert_eq!(actual, expected, "{id}");
}

fn ids(prefix: &str) -> Vec<String> {
    let ids: Vec<String> = cases("io")
        .into_iter()
        .filter(|c| c.starts_with(prefix))
        .collect();
    assert!(!ids.is_empty(), "no {prefix} cases");
    ids
}

fn write(document: &BpmnDocument) -> ichnos_io::Result<Vec<u8>> {
    write_with(document, &Default::default())
}

fn write_with(document: &BpmnDocument, options: &BpmnWriteOptions) -> ichnos_io::Result<Vec<u8>> {
    let mut xml = Vec::new();
    write_bpmn_to_writer(document, &mut xml, options)?;
    Ok(xml)
}

#[test]
fn bpmn_reading_matches_pm4py() {
    let mut checked = 0;
    for id in ids("bpmn-read-").into_iter().chain(ids("bpmn-writer-")) {
        let g = golden("io", &id);
        let expected: Value = g.expected_as();
        let read = read_bpmn(g.fixture("model"), &Default::default());
        if expected.get("error").is_some() {
            assert!(
                read.is_err(),
                "{id}: pm4py fails with {expected}, ichnos reads it"
            );
        } else {
            let document = read.unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_same(&describe(&document, true), &expected, &id);
        }
        checked += 1;
    }
    assert_eq!(checked, 11);
}

#[test]
fn bpmn_writing_matches_pm4py() {
    for id in ids("bpmn-write-") {
        let g = golden("io", &id);
        let expected: Value = g.expected_as();
        let read = read_bpmn(g.fixture("model"), &Default::default());
        match expected.get("stage").and_then(Value::as_str) {
            Some("read") => assert!(read.is_err(), "{id}: pm4py fails to read"),
            Some(_) => {
                let document = read.unwrap_or_else(|e| panic!("{id}: {e}"));
                assert!(write(&document).is_err(), "{id}: pm4py fails to write");
            }
            None => {
                let document = read.unwrap_or_else(|e| panic!("{id}: {e}"));
                let xml = write(&document).unwrap_or_else(|e| panic!("{id}: {e}"));
                let back = read_bpmn_from_reader(xml.as_slice(), &Default::default())
                    .unwrap_or_else(|e| panic!("{id}: reading the written XML: {e}"));
                assert_same(&describe(&back, false), &expected, &id);
            }
        }
    }
}

/// pm4py's two export switches, `enble_bpmn_plane_exporting` and
/// `enable_incoming_outgoing_exporting`, map to `plane` and
/// `incoming_outgoing`.
#[test]
fn bpmn_write_options_match_pm4py() {
    for id in ids("bpmn-options-") {
        let g = golden("io", &id);
        let expected: Value = g.expected_as();
        let params = &g.meta().params;
        let options = BpmnWriteOptions {
            plane: params["plane"].as_bool().unwrap(),
            incoming_outgoing: params["incoming_outgoing"].as_bool().unwrap(),
            ..Default::default()
        };
        let document = read_bpmn(g.fixture("model"), &Default::default())
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let xml = write_with(&document, &options).unwrap_or_else(|e| panic!("{id}: {e}"));
        let text = String::from_utf8(xml.clone()).unwrap();
        assert_eq!(text.contains("BPMNPlane"), options.plane, "{id}: plane");
        assert_eq!(
            text.contains("<incoming>") || text.contains(":incoming>"),
            options.incoming_outgoing,
            "{id}: incoming"
        );
        let back = read_bpmn_from_reader(xml.as_slice(), &Default::default())
            .unwrap_or_else(|e| panic!("{id}: reading the written XML: {e}"));
        assert_same(&describe(&back, false), &expected, &id);
    }
}

#[test]
fn unindented_output_reads_back_the_same() {
    let g = golden("io", "bpmn-read-all_kinds");
    let document = read_bpmn(g.fixture("model"), &Default::default()).unwrap();
    let indented = write(&document).unwrap();
    let flat = write_with(
        &document,
        &BpmnWriteOptions {
            indent: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(indented, flat);
    let read = |xml: &[u8]| read_bpmn_from_reader(xml, &Default::default()).unwrap();
    assert_eq!(
        describe(&read(&flat), true),
        describe(&read(&indented), true)
    );
}

/// The diagram in `fixtures/logs/writer-output/special.bpmn`, which the
/// `bpmn-writer-special` golden reads with pm4py.
fn special() -> BpmnDocument {
    let mut b = Bpmn::new("main");
    b.name = "Special & <odd>".into();
    let start = b.add_node(NodeKind::start_event(), "go");
    let task = b.add_node(NodeKind::task(), "a & \"b\" <c>");
    let split = b.add_node(
        NodeKind::gateway(GatewayKind::Exclusive, GatewayDirection::Diverging),
        "",
    );
    let left = b.add_node(NodeKind::Task(ichnos_model::bpmn::TaskKind::User), "left");
    let right = b.add_node(NodeKind::Task(ichnos_model::bpmn::TaskKind::Send), "right");
    let join = b.add_node(
        NodeKind::gateway(GatewayKind::Exclusive, GatewayDirection::Converging),
        "",
    );
    let end = b.add_node(NodeKind::end_event(), "done");
    let mut flows = Vec::new();
    for (s, t) in [
        (start, task),
        (task, split),
        (split, left),
        (split, right),
        (left, join),
        (right, join),
        (join, end),
    ] {
        flows.push(b.add_flow(s, t).unwrap());
    }
    b.flow_mut(flows[2]).name = "yes".into();
    let mut document = BpmnDocument::from(b);
    document.bounds.insert(
        task,
        Bounds {
            x: 120.5,
            y: 40.0,
            width: 100.0,
            height: 80.0,
        },
    );
    document
        .waypoints
        .insert(flows[0], vec![(36.0, 80.0), (120.5, 80.0)]);
    document.waypoints.insert(flows[1], Vec::new());
    document
}

#[test]
fn writer_output_is_stable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/logs/writer-output/special.bpmn");
    assert_eq!(write(&special()).unwrap(), std::fs::read(path).unwrap());
}

#[test]
fn layout_survives_a_round_trip() {
    let document = special();
    let back =
        read_bpmn_from_reader(write(&document).unwrap().as_slice(), &Default::default()).unwrap();
    let task = back.model.node_by_id("id_1").unwrap();
    assert_eq!(
        back.bounds[&task],
        document.bounds.values().next().copied().unwrap()
    );
    let flow = |id: &str| {
        back.model
            .flows()
            .find(|(_, f)| f.id == id)
            .map(|(f, _)| f)
            .unwrap()
    };
    assert_eq!(
        back.waypoints[&flow("idflow_0")],
        vec![(36.0, 80.0), (120.5, 80.0)]
    );
    assert!(back.waypoints[&flow("idflow_1")].is_empty());
    assert_eq!(back.waypoints[&flow("idflow_2")], vec![(0.0, 0.0); 2]);
    // The writer prefixes `id` to process and flow ids, as pm4py does.
    assert_eq!(back.model.process_id, "idmain");
    assert_eq!(back.model.name, "");
}

#[test]
fn several_processes_need_a_collaboration() {
    let mut b = Bpmn::new("main");
    let a = b.add_node(NodeKind::task(), "a");
    b.node_mut(a).process = "other".into();
    b.add_node(NodeKind::task(), "b");
    assert!(write(&BpmnDocument::from(b)).is_err());
}

#[test]
fn malformed_diagrams_are_errors() {
    let read = |xml: &str| read_bpmn_from_reader(xml.as_bytes(), &Default::default());
    let unknown = r#"<definitions><process id="p"><task id="a"/>
        <sequenceFlow id="f" sourceRef="a" targetRef="missing"/></process></definitions>"#;
    assert!(read(unknown).is_err());
    let twice =
        r#"<definitions><process id="p"><task id="a"/><task id="a"/></process></definitions>"#;
    assert!(read(twice).is_err());
    let bounds = r#"<definitions><process id="p"><task id="a"/></process>
        <BPMNShape id="s" bpmnElement="a"><Bounds x="1" y="2" width="w" height="4"/></BPMNShape>
        </definitions>"#;
    assert!(read(bounds).is_err());
}
