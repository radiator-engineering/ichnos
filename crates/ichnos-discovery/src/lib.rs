//! Process discovery: alpha, inductive, heuristics, ILP and DFG miners.
//!
//! - [`inductive`]: the inductive miner family (IM, IMf, IMd), producing
//!   process trees, Petri nets and BPMN diagrams.
//! - [`mod@dfg`]: frequency and performance DFGs, minimum self-distances and
//!   eventually-follows counts.
//! - [`temporal_profile`]: the temporal profile, the mean and standard
//!   deviation of the time between each pair of activities.
//!
//! Every miner takes an [`ichnos_core::EventLog`] with
//! [`ichnos_core::EventKeys`] and a plain options struct, and returns a model
//! from `ichnos-model` or a typed summary. Errors are this crate's [`Error`].

pub mod dfg;
mod error;
pub mod inductive;
pub mod temporal_profile;

pub use dfg::{
    DfgOptions, EventuallyFollowsOptions, PerformanceDfg, PerformanceDfgOptions,
    PerformanceSummary, derive_minimum_self_distance, dfg, dfg_typed, directly_follows_graph,
    eventually_follows_graph, performance_dfg,
};
pub use error::{Error, Result};
pub use inductive::{
    InductiveOptions, InductiveVariant, bpmn_inductive, bpmn_inductive_dfg, petri_net_inductive,
    petri_net_inductive_dfg, process_tree_inductive, process_tree_inductive_dfg,
    process_tree_inductive_variants,
};
pub use temporal_profile::{TemporalProfile, TemporalProfileOptions, discover_temporal_profile};
