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

pub mod footprints;
mod label;
pub mod petri;
pub mod process_tree;

pub use footprints::{Footprints, TreeFootprints};
pub use label::Label;
pub use petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};
pub use process_tree::{Operator, ProcessTree};
