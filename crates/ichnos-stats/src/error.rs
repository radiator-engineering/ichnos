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
    /// A feature table repeats a column name.
    #[error("duplicate feature column {0}")]
    DuplicateColumn(String),
    /// A feature column has the wrong number of rows.
    #[error("column {column} has {actual} rows, expected {expected}")]
    ColumnLength {
        /// Column name.
        column: String,
        /// Number of case ids.
        expected: usize,
        /// Number of column values.
        actual: usize,
    },
    /// A feature column contains an infinite value.
    #[error("column {0} contains infinity")]
    InfiniteColumn(String),
    /// The aggregation column does not exist.
    #[error("unknown feature column {0}")]
    UnknownColumn(String),
    /// Cube boundaries are not finite, exceed the limit, or have fewer than two distinct values.
    #[error("at least two distinct finite cube boundaries, at most 10001, required")]
    InvalidCubeBoundaries,
    /// A relation or performance observation references an absent event.
    #[error("unknown OCEL event {0}")]
    MissingOcelEvent(String),
    /// A relation references an absent object.
    #[error("unknown OCEL object {0}")]
    MissingOcelObject(String),
    /// An option is outside its valid range.
    #[error("invalid option: {0}")]
    InvalidOption(&'static str),
}
/// Statistics result.
pub type Result<T> = std::result::Result<T, Error>;
