use arrow::csv::{ReaderBuilder, reader::Format};
use ichnos_core::{EventKeys, EventLog, format_batch};
use std::fs::File;
pub fn load(name: &str) -> EventLog {
    let file = File::open(ichnos_golden::fixture_path(format!("{name}.csv"))).unwrap();
    let format = Format::default().with_header(true);
    let (schema, _) = format.infer_schema(file, None).unwrap();
    let schema = arrow::datatypes::Schema::new(
        schema
            .fields()
            .iter()
            .enumerate()
            .map(|(i, f)| {
                if f.name().is_empty() {
                    f.as_ref().clone().with_name(format!("Unnamed: {i}"))
                } else {
                    f.as_ref().clone()
                }
            })
            .collect::<Vec<_>>(),
    );
    let file = File::open(ichnos_golden::fixture_path(format!("{name}.csv"))).unwrap();
    let reader = ReaderBuilder::new(std::sync::Arc::new(schema))
        .with_format(format)
        .with_batch_size(100_000)
        .build(file)
        .unwrap();
    let keys = EventKeys::default();
    let batches = reader
        .map(|b| format_batch(&b.unwrap(), &keys, None).unwrap())
        .collect::<Vec<_>>();
    EventLog::from_arrow_batches(&batches, &keys).unwrap()
}
