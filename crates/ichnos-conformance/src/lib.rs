//! Conformance checking: token replay, alignments, footprints, fitness, precision, generalization and simplicity.
//!
//! - [`alignments`]: optimal alignments of traces against Petri nets, with
//!   alignment-based fitness and precision.
//! - [`token_replay`]: token-based replay of traces on Petri nets, with
//!   token-based fitness.
//!
//! Every module returns [`Error`].

pub mod alignments;
mod error;
pub mod token_replay;

pub use error::{Error, Result};
