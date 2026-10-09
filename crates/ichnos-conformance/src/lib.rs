//! Conformance checking: token replay, alignments, footprints, fitness, precision, generalization and simplicity.
//!
//! - [`alignments`]: optimal alignments of traces against Petri nets, with
//!   alignment-based fitness and precision.
//! - [`temporal_profile`]: deviations of traces from a temporal profile.
//!
//! Every module returns [`Error`].

pub mod alignments;
mod error;
pub mod temporal_profile;

pub use error::{Error, Result};
