//! CSV logs, with pandas-style column inference and event-table formatting.

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

use ichnos_core::{
    EventKeys, EventLog,
    arrow::{
        array::{ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray},
        datatypes::{Field, Schema},
        record_batch::RecordBatch,
    },
};

use crate::{Error, Result, table};

/// Options for reading CSV event tables. The first record contains column names.
#[derive(Debug, Clone)]
pub struct CsvReadOptions {
    /// Source column names, normalized to standard event keys on import.
    pub keys: EventKeys,
    /// Chrono timestamp format; None accepts ISO 8601 and RFC 3339.
    pub timestamp_format: Option<String>,
    /// Field separator, comma by default.
    pub delimiter: u8,
    /// Treat pandas' default textual NA tokens as nulls.
    pub default_na: bool,
}

impl Default for CsvReadOptions {
    fn default() -> Self {
        Self {
            keys: EventKeys::default(),
            timestamp_format: None,
            delimiter: b',',
            default_na: true,
        }
    }
}

/// Options for writing CSV event tables. Nested attributes have no flat representation.
#[derive(Debug, Clone)]
pub struct CsvWriteOptions {
    /// Keys used to build the Arrow event table.
    pub keys: EventKeys,
    /// Field separator.
    pub delimiter: u8,
}

impl Default for CsvWriteOptions {
    fn default() -> Self {
        Self {
            keys: EventKeys::default(),
            delimiter: b',',
        }
    }
}

/// Whether pandas reads `value` as a missing value by default.
pub(crate) fn is_na(value: &str) -> bool {
    matches!(
        value,
        "" | "#N/A"
            | "#N/A N/A"
            | "#NA"
            | "-1.#IND"
            | "-1.#QNAN"
            | "-NaN"
            | "-nan"
            | "1.#IND"
            | "1.#QNAN"
            | "<NA>"
            | "N/A"
            | "NA"
            | "NULL"
            | "NaN"
            | "None"
            | "n/a"
            | "nan"
            | "null"
    )
}

fn infer(values: &[Option<String>]) -> ArrayRef {
    let present: Vec<&str> = values.iter().filter_map(|v| v.as_deref()).collect();
    if !present.is_empty()
        && present.iter().all(|v| v.trim().parse::<i64>().is_ok())
        && present.len() == values.len()
    {
        return Arc::new(Int64Array::from(
            values
                .iter()
                .map(|v| {
                    v.as_ref()
                        .map(|v| v.trim().parse::<i64>().expect("inferred integer"))
                })
                .collect::<Vec<_>>(),
        ));
    }
    if !present.is_empty()
        && present
            .iter()
            .all(|v| v.trim().parse::<f64>().is_ok_and(|v| !v.is_nan()))
    {
        return Arc::new(Float64Array::from(
            values
                .iter()
                .map(|v| {
                    v.as_ref()
                        .map(|v| v.trim().parse::<f64>().expect("inferred float"))
                })
                .collect::<Vec<_>>(),
        ));
    }
    if !present.is_empty()
        && present
            .iter()
            .all(|v| v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false"))
    {
        return Arc::new(BooleanArray::from(
            values
                .iter()
                .map(|v| v.as_ref().map(|v| v.eq_ignore_ascii_case("true")))
                .collect::<Vec<_>>(),
        ));
    }
    Arc::new(StringArray::from(
        values.iter().map(|v| v.as_deref()).collect::<Vec<_>>(),
    ))
}

/// Reads a CSV file, drops rows missing required values, and stable-sorts cases and events.
pub fn read_csv(path: impl AsRef<Path>, options: &CsvReadOptions) -> Result<EventLog> {
    read_csv_from_reader(File::open(path)?, options)
}

/// Reads CSV from a stream into an Arrow batch, then converts it to a formatted log.
pub fn read_csv_from_reader(input: impl Read, options: &CsvReadOptions) -> Result<EventLog> {
    if matches!(options.delimiter, b'\n' | b'\r' | b'"' | 0) {
        return Err(Error::InvalidOption("invalid CSV delimiter"));
    }
    let mut reader = ::csv::ReaderBuilder::new()
        .delimiter(options.delimiter)
        .from_reader(input);
    let headers = reader.headers()?.clone();
    if headers.is_empty() {
        return Err(Error::InvalidOption("CSV needs a header"));
    }
    let mut headers: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if name.is_empty() {
                format!("Unnamed: {i}")
            } else {
                name.to_owned()
            }
        })
        .collect();
    let original: std::collections::HashSet<String> = headers.iter().cloned().collect();
    let mut names = std::collections::HashSet::new();
    for name in &mut headers {
        if names.contains(name) {
            let mut suffix = 1;
            loop {
                let candidate = format!("{name}.{suffix}");
                if !names.contains(&candidate) && !original.contains(&candidate) {
                    *name = candidate;
                    break;
                }
                suffix += 1;
            }
        }
        names.insert(name.clone());
    }
    let mut values = vec![Vec::new(); headers.len()];
    for record in reader.records() {
        let record = record?;
        for (column, value) in values.iter_mut().zip(record.iter()) {
            column.push(if options.default_na && is_na(value) {
                None
            } else {
                Some(value.to_owned())
            });
        }
    }
    let columns: Vec<_> = values.iter().map(|values| infer(values)).collect();
    let schema = Arc::new(Schema::new(
        headers
            .iter()
            .zip(&columns)
            .map(|(name, column)| Field::new(name, column.data_type().clone(), true))
            .collect::<Vec<_>>(),
    ));
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    table::to_log(
        schema,
        &[batch],
        &options.keys,
        options.timestamp_format.as_deref(),
    )
}

/// Writes an event log as a CSV table with one row per event.
/// Dates use RFC 3339. List/container attributes return a core columnar error.
pub fn write_csv(log: &EventLog, path: impl AsRef<Path>, options: &CsvWriteOptions) -> Result<()> {
    write_csv_to_writer(log, File::create(path)?, options)
}

/// Writes a CSV event table to a stream. Log metadata and empty traces are not tabular.
pub fn write_csv_to_writer(
    log: &EventLog,
    output: impl Write,
    options: &CsvWriteOptions,
) -> Result<()> {
    if matches!(options.delimiter, b'\n' | b'\r' | b'"' | 0) {
        return Err(Error::InvalidOption("invalid CSV delimiter"));
    }
    let batch = log.to_arrow(&options.keys)?;
    let stream = ichnos_core::EventStream::from_arrow(&batch)?;
    let schema = batch.schema();
    let mut writer = ::csv::WriterBuilder::new()
        .delimiter(options.delimiter)
        .from_writer(output);
    writer.write_record(schema.fields().iter().map(|field| field.name()))?;
    for event in &stream.events {
        writer.write_record(schema.fields().iter().map(|field| {
            event.get(field.name()).map_or_else(String::new, |value| {
                value
                    .as_date()
                    .map_or_else(|| value.to_string(), |date| date.to_rfc3339())
            })
        }))?;
    }
    writer.flush()?;
    Ok(())
}
