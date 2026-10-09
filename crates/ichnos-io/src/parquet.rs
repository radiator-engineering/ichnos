//! Arrow-backed Parquet event tables.

use std::{fs::File, io::Write, path::Path};

use ::parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::Compression,
    file::{properties::WriterProperties, reader::ChunkReader},
};
use ichnos_core::{EventKeys, EventLog};

use crate::{Error, Result, table};

/// Options for reading Parquet event tables.
#[derive(Debug, Clone)]
pub struct ParquetReadOptions {
    /// Source columns, normalized to standard event keys on import.
    pub keys: EventKeys,
    /// Chrono format for text timestamps; None accepts ISO 8601 / RFC 3339.
    pub timestamp_format: Option<String>,
    /// Arrow batch size. Formatting and sorting still cover the entire table.
    pub batch_size: usize,
}

impl Default for ParquetReadOptions {
    fn default() -> Self {
        Self {
            keys: EventKeys::default(),
            timestamp_format: None,
            batch_size: 8192,
        }
    }
}

/// Options for writing a Parquet event table.
#[derive(Debug, Clone)]
pub struct ParquetWriteOptions {
    /// Keys used for the Arrow view.
    pub keys: EventKeys,
    /// Column compression, Snappy by default.
    pub compression: Compression,
}

impl Default for ParquetWriteOptions {
    fn default() -> Self {
        Self {
            keys: EventKeys::default(),
            compression: Compression::SNAPPY,
        }
    }
}

/// Reads and formats a Parquet file as an event log.
pub fn read_parquet(path: impl AsRef<Path>, options: &ParquetReadOptions) -> Result<EventLog> {
    read_parquet_from_reader(File::open(path)?, options)
}

/// Reads a random-access Parquet source (a File or bytes::Bytes).
pub fn read_parquet_from_reader<R: ChunkReader + 'static>(
    input: R,
    options: &ParquetReadOptions,
) -> Result<EventLog> {
    if options.batch_size == 0 {
        return Err(Error::InvalidOption("Parquet batch size must be positive"));
    }
    let builder = ParquetRecordBatchReaderBuilder::try_new(input)?;
    let schema = builder.schema().clone();
    let reader = builder.with_batch_size(options.batch_size).build()?;
    let batches = reader.collect::<std::result::Result<Vec<_>, _>>()?;
    table::to_log(
        schema,
        &batches,
        &options.keys,
        options.timestamp_format.as_deref(),
    )
}

/// Writes the log's Arrow view to Parquet. Log metadata and empty traces are lost.
pub fn write_parquet(
    log: &EventLog,
    path: impl AsRef<Path>,
    options: &ParquetWriteOptions,
) -> Result<()> {
    write_parquet_to_writer(log, File::create(path)?, options)
}

/// Writes an Arrow-backed Parquet table to a stream, including XES ID field metadata.
pub fn write_parquet_to_writer<W: Write + Send>(
    log: &EventLog,
    output: W,
    options: &ParquetWriteOptions,
) -> Result<()> {
    let batch = log.to_arrow(&options.keys)?;
    let props = WriterProperties::builder()
        .set_compression(options.compression)
        .build();
    let mut writer = ArrowWriter::try_new(output, batch.schema(), Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}
