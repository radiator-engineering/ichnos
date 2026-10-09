//! Object-centric event logs: model, flattening, discovery and filtering.

mod consistency;
pub mod constants;
pub mod filtering;
mod ocel;
mod summary;

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
