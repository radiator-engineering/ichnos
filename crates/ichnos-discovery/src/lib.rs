//! Process discovery: alpha, inductive, heuristics, ILP and DFG miners.
//!
//! - [`inductive`]: the inductive miner family (IM, IMf, IMd), producing
//!   process trees and Petri nets.
//!
//! Every miner takes an [`ichnos_core::EventLog`] with
//! [`ichnos_core::EventKeys`] and a plain options struct, and returns a model
//! from `ichnos-model`. Errors are this crate's [`Error`].

mod error;
pub mod inductive;

pub use error::{Error, Result};
pub use inductive::{
    InductiveOptions, InductiveVariant, petri_net_inductive, petri_net_inductive_dfg,
    process_tree_inductive, process_tree_inductive_dfg, process_tree_inductive_variants,
};
