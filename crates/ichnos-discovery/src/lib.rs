//! Process discovery: alpha, inductive, heuristics, ILP and DFG miners.
//!
//! - [`inductive`]: the inductive miner family (IM, IMf, IMd), producing
//!   process trees, Petri nets and BPMN diagrams, and the POWL miner.
//! - [`mod@dfg`]: frequency and performance DFGs, minimum self-distances and
//!   eventually-follows counts.
//! - [`temporal_profile`]: the temporal profile, the mean and standard
//!   deviation of the time between each pair of activities.
//! - [`footprints`]: footprints of logs, traces and DFGs.
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
pub mod footprints;
/// Seeded causal-matrix genetic process discovery.
pub mod genetic;
pub mod heuristics;
/// Binary-region ILP process discovery.
pub mod ilp;
pub mod inductive;
pub mod log_skeleton;
pub mod prefix_tree;
pub mod split_miner;
pub mod temporal_profile;
pub mod transition_system;

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
pub use footprints::{
    DfgFootprints, LogFootprints, TraceFootprints, dfg_footprints, log_footprints, trace_footprints,
};
pub use genetic::{
    GeneticMatrix, GeneticOptions, GeneticResult, discover_genetic, genetic_matrix_fitness,
    petri_net_genetic,
};
pub use heuristics::{HeuristicsOptions, heuristics_net, petri_net_heuristics};
/// Object-type graphs and event type–object type graphs of object-centric
/// event logs. They live in ichnos-conformance, which compares them.
pub use ichnos_conformance::ocel::{
    Etot, ObjectRelation, Otg, OtgEdge, discover_etot, discover_otg,
};
pub use ilp::{IlpActivity, IlpOptions, petri_net_ilp};
pub use inductive::{
    InductiveOptions, InductiveVariant, PowlOptions, PowlVariant, bpmn_inductive,
    bpmn_inductive_dfg, petri_net_inductive, petri_net_inductive_dfg, powl_inductive,
    powl_inductive_variants, process_tree_inductive, process_tree_inductive_dfg,
    process_tree_inductive_variants,
};
pub use log_skeleton::{LogSkeleton, LogSkeletonOptions, SkeletonRelation, log_skeleton};
pub use prefix_tree::{PrefixNode, PrefixTree, PrefixTreeOptions, prefix_tree};
pub use split_miner::{
    SplitMinerOptions, SplitMinerResult, SplitMinerVariant, bpmn_split_miner, discover_split_miner,
};
pub use temporal_profile::{TemporalProfile, TemporalProfileOptions, discover_temporal_profile};
pub use transition_system::{
    TransitionAbstraction, TransitionDirection, TransitionDiscovery, TransitionEvent,
    TransitionStateData, TransitionSystemOptions, TransitionView, discover_transition_system,
    transition_system,
};
