//! Object-centric event logs: model, flattening, discovery and filtering.

pub mod constants;
mod ocel;
mod summary;

pub use ocel::{
    EventEvent, EventObject, ExtendedRow, ExtendedTable, ObjectChange, ObjectObject, Ocel,
    OcelEvent, OcelObject,
};
pub use summary::OcelSummary;
