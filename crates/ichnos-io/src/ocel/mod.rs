//! Object-centric event log readers: OCEL 1.0 and OCEL 2.0, as JSON and XML
//! (pm4py's `read_ocel_json`, `read_ocel2_json`, `read_ocel_xml`,
//! `read_ocel2_xml`, `read_ocel` and `read_ocel2`).
//!
//! Every reader ends as pm4py's do: it sorts the events by timestamp, keeping
//! file order for ties, orders the relations the same way, then runs
//! [`Ocel::make_consistent`] and [`Ocel::retain_related`]. So an event or
//! object without a relation is not in the result.

mod json;
mod time;
mod xml;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use flate2::read::MultiGzDecoder;
use ichnos_ocel::Ocel;

use crate::error::{Error, Result};

pub use json::{
    read_ocel_json, read_ocel_json_from_reader, read_ocel2_json, read_ocel2_json_from_reader,
};
pub use xml::{
    read_ocel_xml, read_ocel_xml_from_reader, read_ocel2_xml, read_ocel2_xml_from_reader,
};

/// Reads an OCEL 1.0 log, choosing the format by extension (pm4py's
/// `read_ocel`): `.jsonocel` or `.json` for JSON, `.xmlocel` or `.xml` for
/// XML, each optionally gzipped (`.gz`).
pub fn read_ocel(path: impl AsRef<Path>) -> Result<Ocel> {
    let path = path.as_ref();
    match format(path)? {
        Format::Json => read_ocel_json(path),
        Format::Xml => read_ocel_xml(path),
    }
}

/// Reads an OCEL 2.0 log, choosing the format by extension (pm4py's
/// `read_ocel2`): `.jsonocel` or `.json` for JSON, `.xmlocel` or `.xml` for
/// XML, each optionally gzipped (`.gz`).
pub fn read_ocel2(path: impl AsRef<Path>) -> Result<Ocel> {
    let path = path.as_ref();
    match format(path)? {
        Format::Json => read_ocel2_json(path),
        Format::Xml => read_ocel2_xml(path),
    }
}

enum Format {
    Json,
    Xml,
}

fn format(path: &Path) -> Result<Format> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let name = name.strip_suffix(".gz").unwrap_or(&name);
    if name.ends_with(".jsonocel") || name.ends_with(".json") {
        Ok(Format::Json)
    } else if name.ends_with(".xmlocel") || name.ends_with(".xml") {
        Ok(Format::Xml)
    } else {
        Err(Error::Ocel(format!(
            "unsupported OCEL extension: {}",
            path.display()
        )))
    }
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
