/// Errors from log statistics.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A core log validation error.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
    /// A case-based statistic needs a trace identifier.
    #[error("trace {0} has no case identifier")]
    MissingCaseId(usize),
    /// An unsupported attribute value cannot be counted.
    #[error("attribute {key} is not scalar")]
    NonScalar {
        /// Name of the attribute that cannot be counted.
        key: String,
    },
    /// An option is outside its valid range.
    #[error("invalid option: {0}")]
    InvalidOption(&'static str),
}
/// Statistics result.
pub type Result<T> = std::result::Result<T, Error>;
