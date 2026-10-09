//! Process discovery: alpha, inductive, heuristics, ILP and DFG miners.
//!
//! - [`inductive`]: the inductive miner family (IM, IMf, IMd), producing
//!   process trees and Petri nets.
//! - [`mod@dfg`]: frequency and performance DFGs, minimum self-distances and
//!   eventually-follows counts.
//! - [`temporal_profile`]: the temporal profile, the mean and standard
//!   deviation of the time between each pair of activities.
//!
//! - [`alpha`]: classic alpha and alpha+ Petri-net discovery.
//! - [`heuristics`]: classic heuristics nets and their Petri nets.
//!
//! Every miner takes an [`ichnos_core::EventLog`] with
//! [`ichnos_core::EventKeys`] and a plain options struct, and returns a model
//! from `ichnos-model` or a typed summary. Errors are this crate's [`Error`].

pub mod alpha;
pub mod dfg;
mod error;
pub mod heuristics;
pub mod inductive;
pub mod temporal_profile;

pub use alpha::{AlphaOptions, AlphaPlusOptions, petri_net_alpha, petri_net_alpha_plus};
pub use dfg::{
    DfgOptions, EventuallyFollowsOptions, PerformanceDfg, PerformanceDfgOptions,
    PerformanceSummary, derive_minimum_self_distance, dfg, dfg_typed, directly_follows_graph,
    eventually_follows_graph, performance_dfg,
};
pub use error::{Error, Result};
pub use heuristics::{HeuristicsOptions, heuristics_net, petri_net_heuristics};
pub use inductive::{
    InductiveOptions, InductiveVariant, petri_net_inductive, petri_net_inductive_dfg,
    process_tree_inductive, process_tree_inductive_dfg, process_tree_inductive_variants,
};
pub use temporal_profile::{TemporalProfile, TemporalProfileOptions, discover_temporal_profile};
