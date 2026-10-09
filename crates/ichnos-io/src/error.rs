/// Errors returned by log readers and writers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// CSV syntax or record width is invalid.
    #[error(transparent)]
    Csv(#[from] csv::Error),
    /// Parquet encoding or decoding failed.
    #[error(transparent)]
    Parquet(#[from] parquet::errors::ParquetError),
    /// Arrow table construction failed.
    #[error(transparent)]
    Arrow(#[from] ichnos_core::arrow::error::ArrowError),
    /// Format options are invalid.
    #[error("invalid format option: {0}")]
    InvalidOption(&'static str),
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
