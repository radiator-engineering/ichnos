//! Process discovery: alpha, inductive, heuristics, ILP and DFG miners.
//!
//! - [`inductive`]: the inductive miner family (IM, IMf, IMd), producing
//!   process trees and Petri nets.
//! - [`mod@dfg`]: frequency and performance DFGs, minimum self-distances and
//!   eventually-follows counts.
//! - [`temporal_profile`]: the temporal profile, the mean and standard
//!   deviation of the time between each pair of activities.
//! - [`mod@log_skeleton`]: classic log-skeleton relations and frequencies.
//! - [`mod@declare`]: classic DECLARE constraints and count summaries.
//! - [`alpha`]: classic alpha and alpha+ Petri-net discovery.
//! - [`heuristics`]: classic heuristics nets and their Petri nets.
//!
//! Every miner takes an [`ichnos_core::EventLog`] with
//! [`ichnos_core::EventKeys`] and a plain options struct, and returns a model
//! from `ichnos-model` or a typed summary. Errors are this crate's [`Error`].

pub mod alpha;
/// Batch detection by activity and resource.
pub mod batches;
/// Classic case-independent correlation mining.
pub mod correlation;
pub mod declare;
pub mod dfg;
mod error;
/// Seeded causal-matrix genetic process discovery.
pub mod genetic;
pub mod heuristics;
/// Binary-region ILP process discovery.
pub mod ilp;
pub mod inductive;
pub mod log_skeleton;
pub mod prefix_tree;
pub mod temporal_profile;

pub use alpha::{AlphaOptions, AlphaPlusOptions, petri_net_alpha, petri_net_alpha_plus};
pub use batches::{Batch, BatchEvent, BatchGroup, BatchOptions, BatchType, discover_batches};
pub use correlation::{CorrelationEdge, CorrelationOptions, CorrelationResult, correlation_miner};
pub use declare::{
    DeclareActivities, DeclareCounts, DeclareModel, DeclareOptions, DeclareTemplate, declare,
};
pub use dfg::{
    DfgOptions, EventuallyFollowsOptions, PerformanceDfg, PerformanceDfgOptions,
    PerformanceSummary, derive_minimum_self_distance, dfg, dfg_typed, directly_follows_graph,
    eventually_follows_graph, performance_dfg,
};
pub use error::{Error, Result};
pub use heuristics::{HeuristicsOptions, heuristics_net, petri_net_heuristics};
pub use ilp::{IlpActivity, IlpOptions, petri_net_ilp};
pub use inductive::{
    InductiveOptions, InductiveVariant, petri_net_inductive, petri_net_inductive_dfg,
    process_tree_inductive, process_tree_inductive_dfg, process_tree_inductive_variants,
};
pub use log_skeleton::{LogSkeleton, LogSkeletonOptions, SkeletonRelation, log_skeleton};
pub use prefix_tree::{PrefixNode, PrefixTree, PrefixTreeOptions, prefix_tree};
pub use temporal_profile::{TemporalProfile, TemporalProfileOptions, discover_temporal_profile};

pub use genetic::{
    GeneticMatrix, GeneticOptions, GeneticResult, discover_genetic, genetic_matrix_fitness,
    petri_net_genetic,
};
