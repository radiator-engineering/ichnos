use ichnos_core::{EventKeys, EventLog};
use ichnos_io::{
    CsvReadOptions, ParquetReadOptions, read_csv_from_reader, read_parquet, write_csv_to_writer,
    write_parquet_to_writer,
};

#[test]
fn csv_formats_custom_keys_dates_and_stable_sort() {
    let input = "case,activity,when,other_date,number\n2,B,2024-01-02T00:00:00Z,2020-01-01,3\n10,A,2024-01-01T00:00:00Z,2020-01-02,4\n2,C,2024-01-02T00:00:00Z,2020-01-03,5\n,A,2024-01-01T00:00:00Z,2020-01-04,6\n2,,2024-01-01T00:00:00Z,2020-01-05,7\n2,A,,2020-01-06,8\n";
    let options = CsvReadOptions {
        keys: EventKeys::default()
            .with_case_id("case")
            .with_activity("activity")
            .with_timestamp("when"),
        ..Default::default()
    };
    let log = read_csv_from_reader(input.as_bytes(), &options).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log.traces[0].case_id().unwrap().as_str(), Some("10.0"));
    assert_eq!(log.traces[1].case_id().unwrap().as_str(), Some("2.0"));
    let names: Vec<_> = log.traces[1]
        .events
        .iter()
        .map(|e| e.get("concept:name").unwrap().as_str().unwrap())
        .collect();
    assert_eq!(names, ["B", "C"]);
    assert!(
        log.traces[1].events[0]
            .get("other_date")
            .unwrap()
            .as_date()
            .is_some()
    );
    assert_eq!(
        log.traces[1].events[0].get("number").unwrap().as_i64(),
        Some(3)
    );
}

#[test]
fn explicit_dates_nulls_quoted_records_and_inference() {
    let input = "case:concept:name;concept:name;time:timestamp;bool;nullable;label\r\n2;\"A;B\";31/12/2024 10:00;True;1;\"two\nlines\"\r\n2;C;31/12/2024 10:00;False;NA;not a date\r\n";
    let options = CsvReadOptions {
        delimiter: b';',
        timestamp_format: Some("%d/%m/%Y %H:%M".into()),
        ..Default::default()
    };
    let log = read_csv_from_reader(input.as_bytes(), &options).unwrap();
    let event = &log.traces[0].events[0];
    assert_eq!(event.get("concept:name").unwrap().as_str(), Some("A;B"));
    assert_eq!(event.get("bool").unwrap().as_bool(), Some(true));
    assert_eq!(event.get("nullable").unwrap().as_f64(), Some(1.0));
    assert_eq!(event.get("label").unwrap().as_str(), Some("two\nlines"));
    assert!(log.traces[0].events[1].get("nullable").is_none());
}

#[test]
fn errors_and_empty_tables() {
    for input in [
        "a,b\n1,2\n",
        "case:concept:name,concept:name,time:timestamp\n1,A,bad\n",
        "a,b\n1\n",
    ] {
        assert!(
            read_csv_from_reader(input.as_bytes(), &Default::default()).is_err(),
            "{input}"
        );
    }
    let log = read_csv_from_reader(
        "case:concept:name,concept:name,time:timestamp\n".as_bytes(),
        &Default::default(),
    )
    .unwrap();
    assert!(log.is_empty());
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/logs/running-example.parquet");
    assert!(
        read_parquet(
            path,
            &ParquetReadOptions {
                batch_size: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn csv_and_parquet_writes_are_readable() {
    let input = "case:concept:name,concept:name,time:timestamp,cost\n1,A,2024-01-01T00:00:00Z,3\n1,B,2024-01-02T00:00:00Z,4\n";
    let log = read_csv_from_reader(input.as_bytes(), &CsvReadOptions::default()).unwrap();
    let mut csv = Vec::new();
    write_csv_to_writer(&log, &mut csv, &Default::default()).unwrap();
    let back = read_csv_from_reader(csv.as_slice(), &Default::default()).unwrap();
    assert_eq!(log.traces, back.traces);
    let path = std::env::temp_dir().join(format!("ichnos-io-{}.parquet", std::process::id()));
    write_parquet_to_writer(
        &log,
        std::fs::File::create(&path).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let back = read_parquet(
        &path,
        &ParquetReadOptions {
            batch_size: 1,
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(log.traces, back.traces);
}

#[test]
fn parquet_empty_log_round_trip() {
    let path = std::env::temp_dir().join(format!("ichnos-io-empty-{}.parquet", std::process::id()));
    ichnos_io::write_parquet(&EventLog::default(), &path, &Default::default()).unwrap();
    assert!(read_parquet(&path, &Default::default()).unwrap().is_empty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn datetime_inference_sees_rows_before_null_filtering() {
    let input = "case:concept:name,concept:name,time:timestamp,other\n1,A,2024-01-01T00:00:00Z,2020-01-01\n,B,2024-01-01T00:00:00Z,not-a-date\n";
    let log = read_csv_from_reader(input.as_bytes(), &Default::default()).unwrap();
    assert_eq!(log.num_events(), 1);
    assert_eq!(
        log.traces[0].events[0].get("other").unwrap().as_str(),
        Some("2020-01-01")
    );
}

#[test]
fn csv_unnamed_and_duplicate_headers_match_pandas() {
    let input = "case:concept:name,concept:name,time:timestamp,,x,x,x.1\n1,A,2024-01-01T00:00:00Z,4,5,6,7\n";
    let log = read_csv_from_reader(input.as_bytes(), &Default::default()).unwrap();
    let event = &log.traces[0].events[0];
    assert_eq!(event.get("Unnamed: 3").unwrap().as_i64(), Some(4));
    assert_eq!(event.get("x").unwrap().as_i64(), Some(5));
    assert_eq!(event.get("x.2").unwrap().as_i64(), Some(6));
    assert_eq!(event.get("x.1").unwrap().as_i64(), Some(7));
}

#[test]
fn disabling_na_inference_preserves_literal_tokens() {
    let input =
        "case:concept:name,concept:name,time:timestamp,nullable\n1,A,2024-01-01T00:00:00Z,NaN\n";
    let log = read_csv_from_reader(
        input.as_bytes(),
        &CsvReadOptions {
            default_na: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        log.traces[0].events[0].get("nullable").unwrap().as_str(),
        Some("NaN")
    );
}

#[test]
fn parquet_numeric_nan_values_follow_pandas_missing_semantics() {
    use ichnos_core::{AttributeValue, Event, Trace, chrono::DateTime};
    let date = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
    let event: Event = [
        ("concept:name", AttributeValue::from("A")),
        ("time:timestamp", AttributeValue::from(date)),
        ("cost", AttributeValue::Float(f64::NAN)),
    ]
    .into_iter()
    .collect();
    let mut missing = Trace::with_case_id(AttributeValue::Float(f64::NAN));
    missing.events.push(event.clone());
    let mut valid = Trace::with_case_id(AttributeValue::Float(1.0));
    valid.events.push(event);
    let log = EventLog::from_traces(vec![missing, valid]);
    let path = std::env::temp_dir().join(format!("ichnos-io-nan-{}.parquet", std::process::id()));
    ichnos_io::write_parquet(&log, &path, &Default::default()).unwrap();
    let back = read_parquet(&path, &Default::default()).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(back.num_events(), 1);
    assert!(back.traces[0].events[0].get("cost").is_none());
}

#[test]
fn table_writers_reject_nested_attributes() {
    use ichnos_core::{AttributeValue, Attributes};
    for value in [
        AttributeValue::List(vec![("x".into(), 1.into())]),
        AttributeValue::Container(Attributes::new()),
    ] {
        let mut log = ichnos_io::read_xes_from_reader(
            br#"<log><trace><string key="concept:name" value="case"/><event><string key="concept:name" value="A"/><date key="time:timestamp" value="2024-01-01T00:00:00Z"/></event></trace></log>"#.as_slice(),
            &Default::default(),
        ).unwrap();
        log.traces[0].events[0].attributes.insert("nested", value);
        assert!(ichnos_io::write_csv_to_writer(&log, Vec::new(), &Default::default()).is_err());
        assert!(ichnos_io::write_parquet_to_writer(&log, Vec::new(), &Default::default()).is_err());
    }
}
