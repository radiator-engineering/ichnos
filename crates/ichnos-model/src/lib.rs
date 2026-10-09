//! Process models: Petri nets, process trees, BPMN, DFGs, transition systems and POWL.
//!
//! This crate holds the model types that discovery algorithms produce and
//! that conformance checking, simulation and visualization consume.
//!
//! - [`petri`]: Petri nets in arena storage, markings, firing rules and
//!   reachability graphs.
//! - [`process_tree`]: process trees, pm4py's string syntax, simplification
//!   and random playout.
//! - [`footprints`]: behavioural footprints of nets and trees.
//! - [`conversion`]: conversions between model types.
//!
//! Every module has its own error type; [`Error`] wraps them all.
//!
//! # Features
//!
//! - `serde`: `Serialize` and `Deserialize` for [`Label`], [`Footprints`] and
//!   [`TreeFootprints`]. Footprints serialize in the JSON shape of pm4py's
//!   `discover_footprints`, as the golden files store it.

pub mod conversion;
mod error;
pub mod footprints;
mod label;
pub mod petri;
pub mod process_tree;

pub use error::Error;
pub use footprints::{Footprints, TreeFootprints};
pub use label::Label;
pub use petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};
pub use process_tree::{Operator, ProcessTree};
