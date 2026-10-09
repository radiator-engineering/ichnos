//! Process models: Petri nets, process trees, BPMN, DFGs, transition systems and POWL.
//!
//! This crate holds the model types that discovery algorithms produce and
//! that conformance checking, simulation and visualization consume.
//!
//! - [`petri`]: Petri nets in arena storage, markings, firing rules and
//!   reachability graphs.
//! - [`process_tree`]: process trees, pm4py's string syntax, simplification
//!   and random playout.
//! - [`bpmn`]: BPMN diagrams, their gateway reductions and token semantics.
//! - [`dfg`]: directly-follows graphs and their filters.
//! - [`heuristics_net`]: heuristics nets and their AND and loop measures.
//! - [`transition_system`]: transition systems and reachability graphs as
//!   transition systems.
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

pub mod bpmn;
pub mod conversion;
pub mod dfg;
mod error;
pub mod footprints;
pub mod heuristics_net;
mod label;
pub mod petri;
pub mod process_tree;
pub mod transition_system;

pub use bpmn::Bpmn;
pub use dfg::Dfg;
pub use error::Error;
pub use footprints::{Footprints, TreeFootprints};
pub use heuristics_net::HeuristicsNet;
pub use label::Label;
pub use petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};
pub use process_tree::{Operator, ProcessTree};
pub use transition_system::TransitionSystem;
