//! Object-centric event log readers and writers: OCEL 1.0 and OCEL 2.0, as
//! JSON, XML, CSV and SQLite (pm4py's `read_ocel_json`, `read_ocel2_json`,
//! `read_ocel_xml`, `read_ocel2_xml`, `read_ocel_csv`, `read_ocel2_csv`,
//! `read_ocel_sqlite`, `read_ocel2_sqlite`, `read_ocel` and `read_ocel2`,
//! and the matching `write_*` functions).
//!
//! Every JSON and XML reader ends as pm4py's do: it sorts the events by
//! timestamp, keeping file order for ties, orders the relations the same
//! way, then runs [`Ocel::make_consistent`] and [`Ocel::retain_related`].
//! So an event or object without a relation is not in the result. The CSV
//! readers skip [`Ocel::retain_related`], as pm4py's do.

mod csv;
mod csv2;
mod frame;
mod json;
mod literal;
mod sqlite;
mod time;
mod write;
mod xml;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use flate2::read::MultiGzDecoder;
use ichnos_ocel::Ocel;

use crate::error::{Error, Result};

pub use csv::{read_ocel_csv, read_ocel_csv_from_reader, write_ocel_csv, write_ocel_csv_to_writer};
pub use csv2::{
    read_ocel2_csv, read_ocel2_csv_from_reader, write_ocel2_csv, write_ocel2_csv_to_writer,
};
pub use json::{
    read_ocel_json, read_ocel_json_from_reader, read_ocel2_json, read_ocel2_json_from_reader,
};
pub use sqlite::{read_ocel_sqlite, read_ocel2_sqlite, write_ocel_sqlite, write_ocel2_sqlite};
pub use write::{
    write_ocel, write_ocel_json, write_ocel_json_to_writer, write_ocel_xml,
    write_ocel_xml_to_writer, write_ocel2, write_ocel2_json, write_ocel2_json_to_writer,
    write_ocel2_xml, write_ocel2_xml_to_writer,
};
pub use xml::{
    read_ocel_xml, read_ocel_xml_from_reader, read_ocel2_xml, read_ocel2_xml_from_reader,
};

/// Options for the OCEL XML readers. The JSON readers take none.
#[derive(Debug, Clone)]
pub struct OcelReadOptions {
    /// Maximum XML nesting.
    pub max_depth: usize,
    /// Maximum XML element count. Every event, object, attribute and
    /// relationship is at least one element, so raise this for large logs.
    pub max_nodes: usize,
}

impl Default for OcelReadOptions {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 1_000_000,
        }
    }
}

/// Reads an OCEL 1.0 log, choosing the format by extension as pm4py's
/// `read_ocel` does: a name ending in `csv` is CSV, without an objects
/// table, one ending in `.sqlite` is SQLite, one ending in `jsonocel` is
/// JSON and one ending in `xmlocel` is XML. `options` applies to XML.
pub fn read_ocel(path: impl AsRef<Path>, options: &OcelReadOptions) -> Result<Ocel> {
    let path = path.as_ref();
    let name = lower_name(path);
    if name.ends_with("csv") {
        read_ocel_csv(path, None)
    } else if name.ends_with(".sqlite") {
        read_ocel_sqlite(path)
    } else if name.ends_with("jsonocel") {
        read_ocel_json(path)
    } else if name.ends_with("xmlocel") {
        read_ocel_xml(path, options)
    } else {
        Err(unsupported(path))
    }
}

/// Reads an OCEL 2.0 log, choosing the format by extension as pm4py's
/// `read_ocel2` does: a name ending in `xml` or `xmlocel` is XML and one
/// ending in `json` or `jsonocel` is JSON, each optionally followed by `.gz`,
/// one ending in `.ocel.csv` is CSV and one ending in `sqlite` is SQLite.
/// pm4py's bundle (`.ocel.zip`) reader is not ported yet. `options` applies
/// to XML.
pub fn read_ocel2(path: impl AsRef<Path>, options: &OcelReadOptions) -> Result<Ocel> {
    let path = path.as_ref();
    let name = lower_name(path);
    let matches = |extensions: [&str; 2]| {
        extensions
            .iter()
            .any(|e| name.ends_with(e) || name.ends_with(&format!("{e}.gz")))
    };
    if name.ends_with(".ocel.zip") {
        Err(not_ported(path))
    } else if name.ends_with("sqlite") {
        read_ocel2_sqlite(path)
    } else if name.ends_with(".ocel.csv") {
        read_ocel2_csv(path)
    } else if matches(["xml", "xmlocel"]) {
        read_ocel2_xml(path, options)
    } else if matches(["json", "jsonocel"]) {
        read_ocel2_json(path)
    } else {
        Err(unsupported(path))
    }
}

fn lower_name(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

fn not_ported(path: &Path) -> Error {
    Error::Ocel(format!(
        "reading this OCEL format is not ported yet: {}",
        path.display()
    ))
}

fn unsupported(path: &Path) -> Error {
    Error::Ocel(format!("unsupported OCEL file format: {}", path.display()))
}

/// Opens a file, decompressing it when its name ends in `.gz`.
fn open(path: &Path) -> Result<Box<dyn BufRead>> {
    let input = BufReader::new(File::open(path)?);
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gz"))
    {
        Ok(Box::new(BufReader::new(MultiGzDecoder::new(input))))
    } else {
        Ok(Box::new(input))
    }
}

/// Sorts the events by timestamp and the relations by their event's
/// timestamp, both stably, then runs pm4py's consistency step and relation
/// filter.
fn finish(mut ocel: Ocel) -> Ocel {
    ocel.events.sort_by_key(|e| e.timestamp);
    let times: HashMap<&str, _> = ocel
        .events
        .iter()
        .map(|e| (&*e.id, e.timestamp))
        .rev()
        .collect();
    let mut keyed: Vec<_> = std::mem::take(&mut ocel.relations)
        .into_iter()
        .map(|r| (times.get(&*r.event).copied(), r))
        .collect();
    keyed.sort_by_key(|(t, _)| *t);
    ocel.relations = keyed.into_iter().map(|(_, r)| r).collect();
    ocel.make_consistent();
    ocel.retain_related();
    ocel
}
