//! The crate-wide error type.

use std::fmt;

/// A result whose error is [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors produced by ichnos-core.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An event lacks an attribute that an operation needs.
    #[error("{position} has no attribute `{key}`")]
    MissingAttribute {
        /// The attribute key that was looked up.
        key: String,
        /// The event that lacks it.
        position: Position,
    },
    /// An attribute holds a value of the wrong type.
    #[error("attribute `{key}` of {position} is a {found}, expected a {expected}")]
    AttributeType {
        /// The attribute key.
        key: String,
        /// The event that holds the value.
        position: Position,
        /// The XES type name the operation needs.
        expected: &'static str,
        /// The XES type name of the value found.
        found: &'static str,
    },
    /// The log defines no classifier with this name.
    #[error("the log defines no classifier named `{0}`")]
    UnknownClassifier(String),
    /// A list or container attribute cannot go into a flat Arrow column.
    #[error("attribute `{key}` holds a {kind}, which has no flat column representation")]
    NestedAttribute {
        /// The attribute key.
        key: String,
        /// `"list"` or `"container"`.
        kind: &'static str,
    },
    /// A date falls outside the range of nanosecond timestamps (years 1677 to 2262).
    #[error("date {value} in attribute `{key}` is outside the nanosecond timestamp range")]
    TimestampOutOfRange {
        /// The attribute key.
        key: String,
        /// The date, formatted as RFC 3339.
        value: String,
    },
    /// An Arrow column has a data type with no attribute equivalent.
    #[error("column `{column}` has unsupported Arrow type {data_type}")]
    UnsupportedColumn {
        /// The column name.
        column: String,
        /// The Arrow data type, formatted.
        data_type: String,
    },
    /// A table lacks a column that an operation needs.
    #[error("the table has no column `{0}`")]
    MissingColumn(String),
    /// A string in a timestamp column does not parse as a date.
    #[error("value `{value}` in column `{column}` does not parse as a timestamp")]
    UnparseableTimestamp {
        /// The column name.
        column: String,
        /// The value that failed to parse.
        value: String,
    },
    /// An error from the Arrow library.
    #[error(transparent)]
    Arrow(#[from] arrow::error::ArrowError),
}

/// Where an event sits, for error messages. Indices are zero-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Position {
    /// Event `event` of trace `trace` in an [`EventLog`](crate::EventLog).
    Event {
        /// Trace index.
        trace: usize,
        /// Event index within the trace.
        event: usize,
    },
    /// Event `index` of an [`EventStream`](crate::EventStream).
    StreamEvent(usize),
    /// Event `index` of a [`Trace`](crate::Trace) that is not part of a log.
    TraceEvent(usize),
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Event { trace, event } => write!(f, "event {event} of trace {trace}"),
            Self::StreamEvent(index) => write!(f, "stream event {index}"),
            Self::TraceEvent(index) => write!(f, "event {index} of the trace"),
        }
    }
}
