//! Event log statistics for attributes, variants, cases, and time.

pub mod attributes;
pub mod error;
pub use error::{Error, Result};
pub mod cases;
pub mod filters;
pub mod time;
pub mod variants;

/// Numeric and one-hot process cubes.
pub mod cube;
