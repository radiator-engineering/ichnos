//! Errors raised by the discovery algorithms.

/// A result with this crate's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors raised by the discovery algorithms.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Reading the log failed, for example because an event has no
    /// activity.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
    /// A noise threshold is not a fraction in `[0, 1]`.
    #[error("noise threshold {0} is not in [0, 1]")]
    NoiseThreshold(f64),
}
