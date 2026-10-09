//! Errors raised by the discovery algorithms.

/// A result with this crate's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors raised by the discovery algorithms.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// ILP region solving or model reduction failed.
    #[error("ILP solver: {0}")]
    IlpSolver(String),
    /// A nonempty batch trace lacks a string/ID case attribute.
    #[error("trace {trace} has no string/ID case attribute {key}")]
    BatchCase {
        /// Trace index.
        trace: usize,
        /// Configured trace attribute.
        key: String,
    },
    /// Correlation transportation solving failed or produced an invalid solution.
    #[error("correlation solver: {0}")]
    CorrelationSolver(String),
    /// A DECLARE selection fraction must be finite and in `[0, 1]`.
    #[error("DECLARE threshold {option}={value} is not in [0, 1]")]
    DeclareThreshold {
        /// The invalid option.
        option: &'static str,
        /// Its supplied value.
        value: f64,
    },
    /// A heuristics threshold must be a finite fraction in `[0, 1]`.
    #[error("heuristics threshold {option}={value} is not in [0, 1]")]
    HeuristicsThreshold {
        /// The invalid option.
        option: &'static str,
        /// Its supplied value.
        value: f64,
    },

    /// Reading the log failed, for example because an event has no
    /// activity.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
    /// A noise threshold is not a fraction in `[0, 1]`.
    #[error("noise threshold {0} is not in [0, 1]")]
    NoiseThreshold(f64),
    /// An invalid discovery option.
    #[error("invalid discovery option: {0}")]
    InvalidOption(&'static str),
    /// A shared statistics operation failed.
    #[error(transparent)]
    Stats(#[from] ichnos_stats::Error),
}
