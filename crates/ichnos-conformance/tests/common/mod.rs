//! Shared helpers for the golden tests: CSV fixtures read as pandas and
//! pm4py read them, and Petri nets built from their JSON description.
//!
//! The CSV reader is a copy of the one in `ichnos-core`'s golden tests. It
//! goes away when an `ichnos-io` CSV or XES reader is on `main`. The net
//! builder goes away when a PNML reader is.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use ichnos_core::arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray};
use ichnos_core::arrow::datatypes::{DataType, Field, Schema};
use ichnos_core::arrow::record_batch::RecordBatch;
use ichnos_core::{EventKeys, EventLog, format_batch};
use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::{Label, Marking, PetriNet, PlaceId};
use serde_json::Value;

/// Reads a CSV into a batch, typing each column as pandas' `read_csv` would
/// for these fixtures: all ints, else all floats, else strings. Empty cells
/// are nulls. An unnamed column `i` is named `Unnamed: i`, as in pandas.
pub fn read_csv(path: &Path) -> RecordBatch {
    let mut reader = csv::Reader::from_path(path).expect("open csv");
    let headers: Vec<String> = reader
        .headers()
        .expect("csv header")
        .iter()
        .enumerate()
        .map(|(i, h)| {
            if h.is_empty() {
                format!("Unnamed: {i}")
            } else {
                h.to_owned()
            }
        })
        .collect();
    let mut cells: Vec<Vec<Option<String>>> = vec![Vec::new(); headers.len()];
    for record in reader.records() {
        let record = record.expect("csv record");
        for (column, value) in cells.iter_mut().zip(record.iter()) {
            column.push((!value.is_empty()).then(|| value.to_owned()));
        }
    }
    let mut fields = Vec::new();
    let mut columns: Vec<ArrayRef> = Vec::new();
    for (name, values) in headers.iter().zip(cells) {
        let present = || values.iter().flatten();
        let column: ArrayRef = if present().all(|v| v.parse::<i64>().is_ok()) {
            Arc::new(Int64Array::from_iter(
                values
                    .iter()
                    .map(|v| v.as_ref().map(|s| s.parse::<i64>().unwrap())),
            ))
        } else if present().all(|v| v.parse::<f64>().is_ok()) {
            Arc::new(Float64Array::from_iter(
                values
                    .iter()
                    .map(|v| v.as_ref().map(|s| s.parse::<f64>().unwrap())),
            ))
        } else {
            Arc::new(StringArray::from_iter(values.iter().map(|v| v.as_deref())))
        };
        let data_type: DataType = column.data_type().clone();
        fields.push(Field::new(name, data_type, true));
        columns.push(column);
    }
    RecordBatch::try_new(Arc::new(Schema::new(fields)), columns).expect("batch")
}

/// The CSV log at `path`, loaded as the golden harness loads it:
/// `read_csv`, then `format_batch` (pm4py's `format_dataframe`), then grouped
/// by case.
pub fn load_csv_log(path: &Path) -> EventLog {
    let keys = EventKeys::default();
    let table = format_batch(&read_csv(path), &keys, None).expect("format");
    EventLog::from_arrow(&table, &keys).expect("log")
}

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {v}"))
}

fn marking(v: &Value, places: &BTreeMap<String, PlaceId>) -> Marking {
    v.as_object()
        .expect("marking")
        .iter()
        .map(|(p, n)| {
            let n = u32::try_from(n.as_u64().expect("token count")).expect("count fits");
            (places[p], n)
        })
        .collect()
}

/// Builds the net, initial and final marking described by `model` (the
/// JSON form of `tools/golden/cases/conformance.py`).
pub fn build_net(model: &Value) -> (PetriNet, Marking, Marking) {
    let mut net = PetriNet::new("golden");
    let mut places = BTreeMap::new();
    for p in model["places"].as_array().expect("places") {
        let name = p.as_str().expect("place name");
        places.insert(name.to_owned(), net.add_place(name));
    }
    let mut transitions = BTreeMap::new();
    for t in model["transitions"].as_array().expect("transitions") {
        let name = str_field(t, "name");
        let label = t["label"].as_str().map(Label::from);
        transitions.insert(name.to_owned(), net.add_transition(name, label));
    }
    for a in model["arcs"].as_array().expect("arcs") {
        let (source, target) = (str_field(a, "source"), str_field(a, "target"));
        let ends = match (places.get(source), transitions.get(target)) {
            (Some(&p), Some(&t)) => ArcEnds::PlaceToTransition(p, t),
            _ => ArcEnds::TransitionToPlace(transitions[source], places[target]),
        };
        let kind = match str_field(a, "type") {
            "normal" => ArcKind::Normal,
            "inhibitor" => ArcKind::Inhibitor,
            "reset" => ArcKind::Reset,
            other => panic!("unknown arc type {other}"),
        };
        let weight = u32::try_from(a["weight"].as_u64().expect("weight")).expect("weight fits");
        net.add_arc(ends, weight, kind).expect("valid arc");
    }
    let im = marking(&model["initial_marking"], &places);
    let fm = marking(&model["final_marking"], &places);
    (net, im, fm)
}
