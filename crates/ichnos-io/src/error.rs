/// Errors returned by log and model readers and writers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A model-format document is malformed or cannot represent the model.
    #[error("invalid {format}: {detail}")]
    ModelFormat {
        /// The format name.
        format: &'static str,
        /// The validation failure.
        detail: String,
    },
    /// Model construction or validation failed.
    #[error(transparent)]
    Model(#[from] ichnos_model::Error),
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
    /// Invalid OCEL structure or value.
    #[error("invalid OCEL: {0}")]
    Ocel(String),
    /// A SQLite database could not be opened, read or written.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// The JSON document is malformed or does not have the expected shape.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A core log conversion failed.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
}

/// Result returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;
