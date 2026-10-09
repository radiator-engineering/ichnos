//! Conformance checking: token replay, alignments, footprints, fitness, precision, generalization and simplicity.
//!
//! - [`alignments`]: optimal alignments of traces against Petri nets, with
//!   alignment-based fitness and precision.
//!
//! Every module returns [`Error`].

pub mod alignments;
mod error;

pub use error::{Error, Result};
