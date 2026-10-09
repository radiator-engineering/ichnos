//! Boundary, delivery-error and lazy-reader behavior beyond real-log goldens.

use ichnos_core::{Event, EventKeys};
use ichnos_stream::{
    Collector, CsvEventReader, Error, LiveEventStream, MissingEventPolicy, StreamSink, StreamState,
    StreamingDfgDiscovery, StreamingDfgOptions, TraceIterator, XesEventReader, XesTraceReader,
    feed,
};
use std::{cell::RefCell, io::Cursor, rc::Rc};

#[test]
fn stops_drain_inactive_items_and_errors_do_not_skip_other_observers() {
    let borrowed = Rc::new(RefCell::new(Collector::<Event>::new()));
    let other = Rc::new(RefCell::new(Collector::<Event>::new()));
    let mut stream = LiveEventStream::new();
    stream.register(borrowed.clone());
    stream.register(other.clone());
    stream.append(Event::new()).unwrap();
    let guard = borrowed.borrow();
    assert!(matches!(stream.stop(), Err(Error::ObserverBorrowed)));
    assert_eq!(stream.state(), StreamState::Finished);
    assert_eq!(other.borrow().get().len(), 1);
    assert!(guard.get().is_empty());
    drop(guard);
    stream.stop().unwrap();
    assert!(!stream.append(Event::new()).unwrap());
    assert!(matches!(
        stream.start(),
        Err(Error::StreamState(StreamState::Finished))
    ));
}

#[test]
fn missing_events_are_atomic_and_case_ends_move() {
    let options = StreamingDfgOptions {
        missing: MissingEventPolicy::Reject,
        ..Default::default()
    };
    let mut algorithm = StreamingDfgDiscovery::new(options);
    let before = algorithm.get().clone();
    assert!(matches!(
        algorithm.push(&Event::new()),
        Err(Error::MissingField { event: 0, .. })
    ));
    assert_eq!(algorithm.get(), &before);
    for (case, activity) in [("c1", "a"), ("c2", "a"), ("c1", "b")] {
        let event: Event = [("case:concept:name", case), ("concept:name", activity)]
            .into_iter()
            .collect();
        algorithm.push(&event).unwrap();
    }
    assert_eq!(algorithm.get().dfg.end_activities["a"], 1);
    assert_eq!(algorithm.get().dfg.end_activities["b"], 1);
}

#[test]
fn event_reader_is_lazy_fused_and_preserves_empty_items() {
    let text = b"<log><trace><event><string key=\"concept:name\" value=\"a\"/></event>".to_vec();
    let mut reader = XesEventReader::from_reader(Cursor::new(text), Default::default());
    assert_eq!(
        reader
            .next()
            .unwrap()
            .unwrap()
            .get("concept:name")
            .unwrap()
            .as_str(),
        Some("a")
    );
    assert!(reader.next().unwrap().is_err());
    assert!(reader.next().is_none());
    let text = b"<log><global scope=\"event\"><string key=\"concept:name\" value=\"unused\"/></global><trace><event/></trace><trace/></log>".to_vec();
    let events = XesEventReader::from_reader(Cursor::new(text.clone()), Default::default())
        .collect::<ichnos_stream::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(events, [Event::new()]);
    let traces = XesTraceReader::from_reader(Cursor::new(text), Default::default())
        .collect::<ichnos_stream::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(traces.len(), 2);
    assert!(traces[1].events.is_empty());
}

#[test]
fn iterator_adapters_transform_before_filtering_and_report_csv_width_errors() {
    let mut reader = CsvEventReader::from_reader(
        Cursor::new(b"concept:name,case:concept:name\na,c\nb,c\n".to_vec()),
        Default::default(),
    );
    let mut collector = Collector::new();
    let transformed = reader
        .by_ref()
        .map(|event| {
            event.map(|mut event| {
                event.insert(
                    "concept:name",
                    event
                        .get("concept:name")
                        .unwrap()
                        .to_string()
                        .to_uppercase(),
                );
                event
            })
        })
        .filter(|event| {
            event
                .as_ref()
                .ok()
                .is_none_or(|e| e.get("concept:name").unwrap().as_str() == Some("B"))
        });
    assert_eq!(feed(transformed, &mut collector).unwrap(), 1);
    assert_eq!(
        collector.get()[0].get("concept:name").unwrap().as_str(),
        Some("B")
    );
    assert!(matches!(reader.reset(), Err(Error::NotRewindable)));
    let mut bad =
        CsvEventReader::from_reader(Cursor::new(b"a,b\none\n".to_vec()), Default::default());
    assert!(bad.next().unwrap().is_err());
    assert!(bad.next().is_none());
}

#[test]
fn typed_xes_failures_depth_limit_and_missing_projection_fields() {
    for xml in [
        "<trace/>",
        "<log/>garbage",
        "<log><event/></log>",
        "<log><trace><event><int key=\"x\" value=\"wrong\"/></event></trace></log>",
    ] {
        assert!(
            XesEventReader::from_reader(Cursor::new(xml.as_bytes().to_vec()), Default::default())
                .collect::<ichnos_stream::Result<Vec<_>>>()
                .is_err()
        );
    }
    let options = ichnos_stream::XesStreamOptions {
        max_depth: 2,
        ..Default::default()
    };
    assert!(
        XesEventReader::from_reader(
            Cursor::new(b"<log><trace><event/></trace></log>".to_vec()),
            options
        )
        .next()
        .unwrap()
        .is_err()
    );
    let stream = ichnos_core::EventStream {
        events: vec![Event::new()],
        ..Default::default()
    };
    assert!(matches!(
        TraceIterator::from_event_stream(&stream, &EventKeys::default()),
        Err(Error::MissingField { event: 0, .. })
    ));
}

#[test]
fn feed_stops_on_input_error() {
    let mut collector = Collector::<Event>::new();
    let items = [
        Ok(Event::new()),
        Err(Error::Xes("test failure")),
        Ok(Event::new()),
    ];
    assert!(feed(items, &mut collector).is_err());
    assert_eq!(collector.get().len(), 1);
    collector.push(&Event::new()).unwrap();
}

#[test]
fn gzip_streams_reopen_and_decode_the_same_items() {
    use std::io::Write;
    let path =
        std::env::temp_dir().join(format!("ichnos-stream-gzip-{}.xes.gz", std::process::id()));
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = flate2::write::GzEncoder::new(file, Default::default());
    encoder
        .write_all(b"<log><trace><event/></trace></log>")
        .unwrap();
    encoder.finish().unwrap();
    let mut reader = XesEventReader::open(&path, Default::default()).unwrap();
    assert_eq!(reader.next().unwrap().unwrap(), Event::new());
    assert!(reader.next().is_none());
    reader.reset().unwrap();
    assert_eq!(reader.next().unwrap().unwrap(), Event::new());
    drop(reader);
    std::fs::remove_file(path).unwrap();
}
