//! Errors from feature extraction.

/// Errors from feature extraction.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An event lacks an attribute the computation needs.
    #[error("event {event} of trace {trace} has no attribute {key}")]
    MissingAttribute {
        /// Position of the trace in the log.
        trace: usize,
        /// Position of the event in the trace.
        event: usize,
        /// The missing attribute.
        key: String,
    },
    /// An event attribute that should be a timestamp is not one.
    #[error("attribute {key} of event {event} in trace {trace} is not a timestamp")]
    NotTimestamp {
        /// Position of the trace in the log.
        trace: usize,
        /// Position of the event in the trace.
        event: usize,
        /// The attribute.
        key: String,
    },
    /// A trace lacks a trace attribute the computation needs.
    #[error("trace {trace} has no trace attribute {key}")]
    MissingTraceAttribute {
        /// Position of the trace in the log.
        trace: usize,
        /// The missing attribute.
        key: String,
    },
    /// No event of a trace has a numeric event attribute the computation needs.
    #[error("no event of trace {trace} has the event attribute {key}")]
    MissingEventAttribute {
        /// Position of the trace in the log.
        trace: usize,
        /// The missing attribute.
        key: String,
    },
    /// An attribute that should be numeric is not.
    #[error("attribute {0} is not numeric")]
    NotNumeric(String),
    /// A requested column is not in the log.
    #[error("the log has no column {0}")]
    MissingColumn(String),
    /// The computation needs at least one trace.
    #[error("the log has no traces")]
    EmptyLog,
    /// An OCEL event has no related objects.
    #[error("event {0} has no related objects")]
    EventWithoutObjects(String),
    /// An option is outside its valid range.
    #[error("invalid option: {0}")]
    InvalidOption(&'static str),
}

/// Feature extraction result.
pub type Result<T> = std::result::Result<T, Error>;
