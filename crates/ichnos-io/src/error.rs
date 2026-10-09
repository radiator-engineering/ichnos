/// Errors returned by log readers and writers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// File or stream I/O failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The XML stream is malformed.
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    /// Invalid XES structure or typed value.
    #[error("invalid XES: {0}")]
    Xes(String),
    /// A core log conversion failed.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
}

/// Result returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;
