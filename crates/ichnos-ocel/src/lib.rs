//! Object-centric event logs: model, flattening, discovery and filtering.

mod consistency;
pub mod constants;
mod graphs;
mod log_to_ocel;
mod networkx;
mod ocel;
mod summary;

pub use graphs::{ObjectGraph, ObjectGraphKind, discover_objects_graph};
pub use log_to_ocel::{LogToOcelOptions, convert_log_to_ocel};
pub use networkx::{
    OcelFeaturesToNxOptions, OcelGraph, OcelGraphEdge, OcelGraphNode, OcelToNxOptions,
    convert_ocel_features_to_networkx, convert_ocel_to_networkx,
};
pub use ocel::{
    EventEvent, EventObject, ExtendedRow, ExtendedTable, ObjectChange, ObjectObject, Ocel,
    OcelEvent, OcelObject,
};
pub use summary::OcelSummary;
