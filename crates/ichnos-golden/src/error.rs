use std::path::PathBuf;

/// Errors from loading a golden file.
#[derive(Debug, thiserror::Error)]
pub enum GoldenError {
    /// The file could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The file is not valid JSON.
    #[error("{path} is not valid JSON: {source}")]
    Json {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        source: serde_json::Error,
    },
    /// The file is JSON but lacks a required top-level field.
    #[error("{path} has no `{field}` field; regenerate it with tools/golden/generate.py")]
    MissingField {
        /// The file.
        path: PathBuf,
        /// The missing field.
        field: &'static str,
    },
    /// A part of the file does not deserialize into the requested type.
    #[error("{path}: `{part}` does not match the requested type: {source}")]
    Shape {
        /// The file.
        path: PathBuf,
        /// `meta` or `expected`.
        part: &'static str,
        /// The underlying error.
        source: serde_json::Error,
    },
}
