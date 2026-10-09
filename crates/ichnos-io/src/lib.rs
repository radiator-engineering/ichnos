//! Readers and writers for XES event logs, CSV and Parquet tables.

pub mod csv;
mod error;
pub mod parquet;
mod table;
pub mod xes;

pub use csv::{
    CsvReadOptions, CsvWriteOptions, read_csv, read_csv_from_reader, write_csv, write_csv_to_writer,
};
pub use error::{Error, Result};
pub use parquet::{
    ParquetReadOptions, ParquetWriteOptions, read_parquet, read_parquet_from_reader, write_parquet,
    write_parquet_to_writer,
};
pub use xes::{
    XesReadOptions, XesWriteOptions, read_xes, read_xes_from_reader, write_xes, write_xes_to_writer,
};
