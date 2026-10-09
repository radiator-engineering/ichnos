//! Conformance checking: token replay, alignments, footprints, fitness, precision, generalization and simplicity.
//!
//! - [`alignments`]: optimal alignments of traces against Petri nets, with
//!   alignment-based fitness and precision.
//! - [`token_replay`]: token-based replay of traces on Petri nets, with
//!   token-based fitness and ETConformance precision.
//! - [`generalization`]: token-based generalization of Petri nets.
//! - [`temporal_profile`]: deviations of traces from a temporal profile.
//!
//! Every module returns [`Error`].

pub mod alignments;
mod error;
pub mod generalization;
pub mod temporal_profile;
pub mod token_replay;

pub use error::{Error, Result};
