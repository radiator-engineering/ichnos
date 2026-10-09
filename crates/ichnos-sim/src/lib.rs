//! Seeded process-model simulation and random process-tree generation.
//!
//! ```
//! use ichnos_sim::{Model, PlayOutOptions, parse_process_tree, play_out};
//! let tree = parse_process_tree("->( 'start', X( 'accept', 'reject' ) )")?;
//! let options = PlayOutOptions { traces: 100, ..Default::default() };
//! let log = play_out(Model::Tree(&tree), &options, &mut rand::rng())?;
//! assert_eq!(log.traces.len(), 100);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
/// DECLARE prefix-safe random generation.
pub mod declare;
mod error;
pub use error::Error;
/// Compatibility name for simulation errors.
pub type SimulationError = Error;
pub mod generator;
pub mod playout;
pub use generator::{GeneratorOptions, generate_process_tree};
pub use ichnos_model::process_tree::{ParseError, ProcessTree};
pub use playout::{DfgOptions, Model, PlayOutOptions, play_out, play_out_dfg};

/// Parses the process-tree notation accepted by pm4py's entry point.
pub fn parse_process_tree(text: &str) -> Result<ProcessTree, ParseError> {
    ProcessTree::parse(text)
}
