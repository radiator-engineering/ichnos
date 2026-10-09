//! Object-centric event logs: model, flattening, discovery and filtering.

mod consistency;
pub mod constants;
pub mod filtering;
mod graphs;
mod log_to_ocel;
mod networkx;
mod ocdfg;
mod ocel;
mod summary;

pub use graphs::{ObjectGraph, ObjectGraphKind, discover_objects_graph};
pub use log_to_ocel::{LogToOcelOptions, convert_log_to_ocel};
pub use networkx::{
    OcelFeaturesToNxOptions, OcelGraph, OcelGraphEdge, OcelGraphNode, OcelToNxOptions,
    convert_ocel_features_to_networkx, convert_ocel_to_networkx,
};
pub use ocdfg::{
    Ocdfg, OcdfgActivities, OcdfgActivity, OcdfgDurations, OcdfgEdge, OcdfgEdges, OcdfgError,
    OcdfgOptions, SecondsBetween, discover_ocdfg,
};
pub use ocel::{
    EventEvent, EventObject, ExtendedRow, ExtendedTable, ObjectChange, ObjectObject, Ocel,
    OcelEvent, OcelObject,
};
pub use summary::OcelSummary;

pub use filtering::*;

pub mod clustering;
pub mod olap;
pub mod transformations;
pub use clustering::*;
pub use olap::*;
pub use transformations::*;
