use ichnos_io::*;
use ichnos_model::{
    Dfg, Operator, ProcessTree,
    petri::{ArcKind, Marking},
};

fn writer_fixture(name: &str, output: &[u8]) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/logs/writer-output")
        .join(name);
    assert_eq!(output, std::fs::read(path).unwrap());
}

#[test]
fn pnml_retains_alternative_markings_special_arcs_stochastic_and_data() {
    let xml=br#"<pnml xmlns="http://www.pnml.org/version-2009/grammar/pnml"><net id="net&amp;name"><page id="page">
      <arc id="a" source="p" target="t"><inscription><text>2</text></inscription><arctype><text>inhibitor</text></arctype></arc>
      <page id="nested"><place id="p"><name><text>in&amp;put</text></name><initialMarking><text>2</text></initialMarking></place></page>
      <place id="q"/><place id="r"/>
      <transition id="t" guard="x &gt; 0"><name><text><![CDATA[work & more]]></text></name>
        <toolspecific tool="StochasticPetriNet"><property key="distributionType">EXPONENTIAL</property><property key="distributionParameters">3.5</property><property key="priority">2</property><property key="weight">0.25</property><property key="invisible">true</property><property key="vendor">x&amp;y</property></toolspecific>
        <readVariable>x</readVariable><writeVariable>y</writeVariable>
      </transition>
      <arc id="b" source="q" target="t"><type value="reset"/></arc><arc id="c" source="t" target="r"/>
    </page><finalmarkings><marking><place idref="r"><text>3</text></place></marking><marking><place idref="p"><text>1</text></place></marking></finalmarkings>
    <variables><variable type="java.lang.Double"><name>x</name></variable></variables></net></pnml>"#;
    let document = read_pnml_from_reader(xml.as_slice(), &Default::default()).unwrap();
    assert_eq!(document.additional_final_markings.len(), 1);
    assert_eq!(
        document
            .model
            .initial_marking
            .iter()
            .map(|(_, n)| n)
            .sum::<u32>(),
        2
    );
    assert_eq!(
        document
            .model
            .final_marking
            .iter()
            .map(|(_, n)| n)
            .sum::<u32>(),
        3
    );
    assert_eq!(
        document
            .model
            .net
            .arcs()
            .filter(|(_, a)| a.kind == ArcKind::Reset)
            .count(),
        1
    );
    let (t, transition) = document.model.net.transitions().next().unwrap();
    assert!(transition.is_silent());
    let info = &document.stochastic[&t];
    assert_eq!(info.distribution_parameters.as_deref(), Some("3.5"));
    assert_eq!(info.properties["vendor"], "x&y");
    assert_eq!(document.transition_data[&t].guard.as_deref(), Some("x > 0"));
    assert_eq!(document.transition_data[&t].read_variables, ["x"]);
    assert_eq!(document.transition_data[&t].write_variables, ["y"]);
    let mut output = Vec::new();
    write_pnml_to_writer(&document, &mut output, &Default::default()).unwrap();
    writer_fixture("special.pnml", &output);
    assert_eq!(
        document,
        read_pnml_from_reader(output.as_slice(), &Default::default()).unwrap()
    );
    output.clear();
    write_pnml_to_writer(
        &document,
        &mut output,
        &PnmlWriteOptions {
            include_alternative_final_markings: false,
            ..Default::default()
        },
    )
    .unwrap();
    let primary_only = read_pnml_from_reader(output.as_slice(), &Default::default()).unwrap();
    assert!(primary_only.additional_final_markings.is_empty());
    assert_eq!(
        primary_only.model.final_marking,
        document.model.final_marking
    );
    let mut enabled = Marking::new();
    assert!(
        document
            .model
            .net
            .enabled_transitions(&enabled)
            .contains(&t)
    );
    enabled = document.model.net.fire(t, &enabled).unwrap();
    assert_eq!(enabled.iter().map(|(_, n)| n).sum::<u32>(), 1);
}

#[test]
fn writer_loop_and_boundary_only_dfg_have_stable_bytes() {
    let tree = ProcessTree::node(
        Operator::Loop,
        vec![ProcessTree::activity("A"), ProcessTree::activity("B")],
    );
    let mut output = Vec::new();
    write_ptml_to_writer(&tree, &mut output, &Default::default()).unwrap();
    writer_fixture("loop.ptml", &output);
    let mut graph = Dfg::new();
    graph.add_start("only", 2);
    graph.add_end("only", 2);
    output.clear();
    write_dfg_to_writer(&graph, &mut output, &Default::default()).unwrap();
    writer_fixture("boundary.dfg", &output);
}

#[test]
fn pnml_empty_names_fall_back_to_ids() {
    let document = read_pnml_from_reader(br#"<pnml><net id="n"><place id="p"><name><text/></name></place><transition id="t"><name><text/></name></transition></net></pnml>"#.as_slice(), &Default::default()).unwrap();
    assert!(document.place_names.is_empty());
    let (id, transition) = document.model.net.transitions().next().unwrap();
    assert_eq!(transition.label.as_ref().unwrap().as_str(), "t");
    assert_eq!(document.transition_names[&id], "t");
}

#[test]
fn pnml_final_guess_is_opt_in_and_explicit_empty_stays_empty() {
    let xml = br#"<pnml><net id="n"><place id="sink"/></net></pnml>"#;
    assert!(
        read_pnml_from_reader(xml.as_slice(), &Default::default())
            .unwrap()
            .model
            .final_marking
            .is_empty()
    );
    let document = read_pnml_from_reader(
        xml.as_slice(),
        &PnmlReadOptions {
            guess_final_marking: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        document
            .model
            .final_marking
            .iter()
            .map(|(_, n)| n)
            .sum::<u32>(),
        1
    );
    let empty=br#"<pnml><net id="n"><place id="sink"/><finalmarkings><marking/></finalmarkings></net></pnml>"#;
    let document = read_pnml_from_reader(
        empty.as_slice(),
        &PnmlReadOptions {
            guess_final_marking: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(document.model.final_marking.is_empty());
    let mut output = Vec::new();
    write_pnml_to_writer(&document, &mut output, &Default::default()).unwrap();
    assert_eq!(
        document,
        read_pnml_from_reader(output.as_slice(), &Default::default()).unwrap()
    );
}

#[test]
fn malformed_pnml_and_xml_limits_are_errors() {
    for xml in [
        "",
        "<!DOCTYPE pnml><pnml/>",
        "<pnml><net/><net/></pnml>",
        "<pnml><net><place id='p'/><transition id='p'/></net></pnml>",
        "<pnml><net><arc id='a' source='missing' target='missing'/></net></pnml>",
        "<pnml><net><place id='p'><initialMarking><text>-1</text></initialMarking></place></net></pnml>",
        "<pnml><net><place id='p'/><finalmarkings><marking><place idref='q'><text>1</text></place></marking></finalmarkings></net></pnml>",
    ] {
        assert!(
            read_pnml_from_reader(xml.as_bytes(), &Default::default()).is_err(),
            "{xml}"
        );
    }
    assert!(
        read_pnml_from_reader(
            b"<pnml><net><place id='p'/></net></pnml>".as_slice(),
            &PnmlReadOptions {
                max_nodes: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn ptml_preserves_nested_non_silent_loop_exit_position() {
    let xml=br#"<ptml><processTree root="s"><sequence id="s"/><xorLoop id="l"/><manualTask id="a" name="A"/><manualTask id="b" name="B"/><manualTask id="exit" name="exit"/><manualTask id="after" name="after"/>
    <parentsNode sourceId="s" targetId="l"/><parentsNode sourceId="s" targetId="after"/><parentsNode sourceId="l" targetId="a"/><parentsNode sourceId="l" targetId="b"/><parentsNode sourceId="l" targetId="exit"/></processTree></ptml>"#;
    let tree = read_ptml_from_reader(xml.as_slice(), &Default::default()).unwrap();
    let expected = ProcessTree::sequence([
        ProcessTree::sequence([
            ProcessTree::node(
                Operator::Loop,
                vec![ProcessTree::activity("A"), ProcessTree::activity("B")],
            ),
            ProcessTree::activity("exit"),
        ]),
        ProcessTree::activity("after"),
    ]);
    assert_eq!(tree, expected);
    let mut output = Vec::new();
    write_ptml_to_writer(&tree, &mut output, &Default::default()).unwrap();
    assert_eq!(
        tree,
        read_ptml_from_reader(output.as_slice(), &Default::default()).unwrap()
    );
}

#[test]
fn ptml_cycles_missing_nodes_and_expansion_limits_error() {
    for xml in [
        "<ptml><processTree root='s'><sequence id='s'/><parentsNode sourceId='s' targetId='s'/></processTree></ptml>",
        "<ptml><processTree root='x'><automaticTask id='s'/></processTree></ptml>",
        "<ptml><processTree root='s'><manualTask id='s' name='A'/><automaticTask id='unused'/></processTree></ptml>",
    ] {
        assert!(
            read_ptml_from_reader(xml.as_bytes(), &Default::default()).is_err(),
            "{xml}"
        );
    }
    let tree = ProcessTree::sequence([ProcessTree::activity("A"), ProcessTree::Tau]);
    assert!(
        write_ptml_to_writer(
            &tree,
            Vec::new(),
            &PtmlWriteOptions {
                max_nodes: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        write_ptml_to_writer(
            &ProcessTree::node(Operator::Interleaving, vec![ProcessTree::activity("A")]),
            Vec::new(),
            &Default::default()
        )
        .is_err()
    );
}

#[test]
fn dfg_duplicate_records_boundaries_unicode_and_empty_graphs() {
    let text = "2\nα\nβ\n2\n0x1\n0x3\n1\n1x5\n0>1x2\n0>1x7\n";
    let graph = read_dfg_from_reader(text.as_bytes(), &Default::default()).unwrap();
    assert_eq!(graph.graph.values().copied().collect::<Vec<_>>(), [7]);
    assert_eq!(
        graph.start_activities.values().copied().collect::<Vec<_>>(),
        [3]
    );
    for graph in [graph, Dfg::new(), {
        let mut g = Dfg::new();
        g.add_start("only", 2);
        g.add_end("only", 2);
        g
    }] {
        let mut output = Vec::new();
        write_dfg_to_writer(&graph, &mut output, &Default::default()).unwrap();
        assert_eq!(
            graph,
            read_dfg_from_reader(output.as_slice(), &Default::default()).unwrap()
        );
    }
}

#[test]
fn dfg_invalid_indexes_counts_and_unrepresentable_names_error() {
    for text in [
        "",
        "-1\n",
        "1\nA\n1\n1x2\n0\n",
        "1\nA\n0\n0\n0>0x-1\n",
        "2\nA\nA\n0\n0\n",
    ] {
        assert!(
            read_dfg_from_reader(text.as_bytes(), &Default::default()).is_err(),
            "{text}"
        );
    }
    let mut dfg = Dfg::new();
    dfg.add_start(" trimmed ", 1);
    assert!(write_dfg_to_writer(&dfg, Vec::new(), &Default::default()).is_err());
    assert!(
        read_dfg_from_reader(
            b"1\nA\n0\n0\n".as_slice(),
            &DfgReadOptions {
                max_activities: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn unbounded_sample_net_imports_and_round_trips_without_footprints() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/logs/more_models/SampleNet.pnml");
    let document = read_pnml(path, &Default::default()).unwrap();
    assert_eq!(document.model.net.places().count(), 4);
    assert_eq!(document.model.net.transitions().count(), 4);
    let mut output = Vec::new();
    write_pnml_to_writer(&document, &mut output, &Default::default()).unwrap();
    assert_eq!(
        document,
        read_pnml_from_reader(output.as_slice(), &Default::default()).unwrap()
    );
}
