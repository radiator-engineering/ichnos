use ichnos_core::AttributeValue;
use ichnos_io::{XesReadOptions, XesWriteOptions, read_xes_from_reader, write_xes_to_writer};

#[test]
fn nested_values_metadata_and_escaping_round_trip() {
    let xml = br#"<?xml version="1.0"?><log xmlns="http://www.xes-standard.org/">
      <extension name="Concept" prefix="concept" uri="http://example.org/?a=1&amp;b=2"/>
      <classifier name="Activity" keys="concept:name 'key with spaces'"/>
      <global scope="event"><string key="concept:name" value="unknown"/></global>
      <global scope="trace"><int key="n" value="0"/></global>
      <string key="creator" value="a &amp; &quot;b&quot;"/>
      <trace><string key="concept:name" value="1"/><event>
        <string key="concept:name" value="A"><string key="source" value="nested"><int key="rank" value="3"/></string></string>
        <list key="items"><string key="unit" value="EUR"/><values><int key="x" value="1"/><int key="x" value="2"/></values></list>
        <container key="bag"><boolean key="yes" value="true"/><id key="id" value="abc"/></container>
        <date key="time:timestamp" value="2024-01-01T10:00:00.123456789+02:00"/>
        <float key="cost" value="1.5"/>
      </event></trace><trace/>
    </log>"#;
    let log = read_xes_from_reader(xml.as_slice(), &XesReadOptions::default()).unwrap();
    let event = &log.traces[0].events[0];
    assert_eq!(event.get("items").unwrap().as_list().unwrap().len(), 2);
    assert!(matches!(
        event.get("bag"),
        Some(AttributeValue::Container(_))
    ));
    assert_eq!(event.get("concept:name").unwrap().as_str(), Some("A"));
    assert_eq!(log.classifiers[0].keys, ["concept:name", "key with spaces"]);
    let mut output = Vec::new();
    write_xes_to_writer(&log, &mut output, &XesWriteOptions::default()).unwrap();
    assert_eq!(
        log,
        read_xes_from_reader(output.as_slice(), &XesReadOptions::default()).unwrap()
    );
}

#[test]
fn malformed_documents_are_errors() {
    for xml in [
        "",
        "<trace/>",
        "<log><trace></log>",
        "<log><event/></log>",
        "<log><int key='x' value='bad'/></log>",
        "<log><trace>",
        "<log/><log/>",
        "<!DOCTYPE log><log/>",
    ] {
        assert!(
            read_xes_from_reader(xml.as_bytes(), &XesReadOptions::default()).is_err(),
            "{xml}"
        );
    }
    let options = XesReadOptions {
        max_depth: 2,
        ..Default::default()
    };
    assert!(
        read_xes_from_reader("<log><trace><event/></trace></log>".as_bytes(), &options).is_err()
    );
}

#[test]
fn xes_20_direct_list_children_are_values() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/logs/xes_20.xes");
    let log = ichnos_io::read_xes(path, &Default::default()).unwrap();
    let event = &log.traces[0].events[0];
    assert_eq!(event.get("lista").unwrap().as_list().unwrap().len(), 2);
    assert_eq!(
        event
            .get("contenitore")
            .unwrap()
            .as_container()
            .unwrap()
            .len(),
        2
    );
    let mut output = Vec::new();
    write_xes_to_writer(&log, &mut output, &Default::default()).unwrap();
    assert_eq!(
        log,
        read_xes_from_reader(output.as_slice(), &Default::default()).unwrap()
    );
}

#[test]
fn gzip_file_round_trip() {
    let log = read_xes_from_reader(
        "<log><trace><event><string key='concept:name' value='A'/></event></trace></log>"
            .as_bytes(),
        &Default::default(),
    )
    .unwrap();
    let path = std::env::temp_dir().join(format!("ichnos-io-{}.xes.gz", std::process::id()));
    ichnos_io::write_xes(&log, &path, &Default::default()).unwrap();
    let back = ichnos_io::read_xes(&path, &Default::default()).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(log, back);
}

#[test]
fn vendor_elements_character_content_and_missing_values_are_ignored() {
    let xml = br#"<log><trace><event>text &amp; more<![CDATA[vendor text]]>
      <foo key="vendor" value="x"><int key="hidden" value="invalid"/></foo>
      <string key="absent"/><int key="absent_number"/>
      <string key="concept:name" value="A"/>
    </event></trace></log>"#;
    let log = read_xes_from_reader(xml.as_slice(), &Default::default()).unwrap();
    assert_eq!(log.traces[0].events[0].attributes.len(), 1);
    assert_eq!(
        log.traces[0].events[0]
            .get("concept:name")
            .unwrap()
            .as_str(),
        Some("A")
    );
}
