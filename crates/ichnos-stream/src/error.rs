//! Typed streaming errors.

/// An error from a reader, conversion or stream observer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid conformance settings or unsupported model structure.
    #[error("invalid streaming conformance setting: {0}")]
    InvalidConformanceOption(&'static str),
    /// A timestamp field has a non-date type.
    #[error("event {event} has a non-date {key:?} attribute")]
    TimestampType {
        /// Timestamp key.
        key: String,
        /// Zero-based input event index.
        event: usize,
    },
    /// A canonical log or Arrow conversion error.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
    /// An underlying input/output error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An invalid CSV record.
    #[error(transparent)]
    Csv(#[from] csv::Error),
    /// An invalid XML token.
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    /// An invalid XES attribute or value.
    #[error(transparent)]
    XesAttribute(#[from] ichnos_io::Error),
    /// An invalid XES document structure.
    #[error("invalid XES stream: {0}")]
    Xes(&'static str),
    /// An event lacks a required field.
    #[error("event {event} has no {key:?} attribute")]
    MissingField {
        /// Required attribute key.
        key: String,
        /// Zero-based input event index.
        event: usize,
    },
    /// A finished or already active stream cannot start again.
    #[error("cannot start a {0:?} stream")]
    StreamState(crate::StreamState),
    /// A shared observer is already borrowed by its caller.
    #[error("stream observer is already borrowed")]
    ObserverBorrowed,
    /// A reader constructed without a path cannot be reopened.
    #[error("reset requires a reader opened from a path")]
    NotRewindable,
}

/// Result of a streaming operation.
pub type Result<T> = std::result::Result<T, Error>;
