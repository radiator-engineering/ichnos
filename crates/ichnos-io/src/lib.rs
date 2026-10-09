//! Readers and writers for event logs, object-centric event logs, tables, Petri nets, process
//! trees and DFGs.

pub mod csv;
pub mod dfg;
mod error;
mod model_xml;
pub mod ocel;
pub mod parquet;
pub mod pnml;
pub mod ptml;
mod table;
pub mod xes;

pub use csv::{
    CsvReadOptions, CsvWriteOptions, read_csv, read_csv_from_reader, write_csv, write_csv_to_writer,
};
pub use dfg::{
    DfgReadOptions, DfgWriteOptions, read_dfg, read_dfg_from_reader, write_dfg, write_dfg_to_writer,
};
pub use error::{Error, Result};
pub use ocel::{
    OcelReadOptions, read_ocel, read_ocel_json, read_ocel_json_from_reader, read_ocel_xml,
    read_ocel_xml_from_reader, read_ocel2, read_ocel2_json, read_ocel2_json_from_reader,
    read_ocel2_xml, read_ocel2_xml_from_reader,
};
pub use parquet::{
    ParquetReadOptions, ParquetWriteOptions, read_parquet, read_parquet_from_reader, write_parquet,
    write_parquet_to_writer,
};
pub use pnml::{
    PnmlDocument, PnmlReadOptions, PnmlVariable, PnmlWriteOptions, StochasticInfo, TransitionData,
    read_pnml, read_pnml_from_reader, write_pnml, write_pnml_to_writer,
};
pub use ptml::{
    PtmlReadOptions, PtmlWriteOptions, read_ptml, read_ptml_from_reader, write_ptml,
    write_ptml_to_writer,
};
pub use xes::{
    XesReadOptions, XesWriteOptions, read_xes, read_xes_from_reader, write_xes, write_xes_to_writer,
};
