use std::path::PathBuf;

/// Errors returned while loading or querying an OFD document.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Host file I/O failed.
    #[error("I/O error for {path}: {source}")]
    Io {
        /// Host path being accessed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The ZIP container is invalid.
    #[error("invalid OFD container: {0}")]
    Container(String),
    /// An XML file cannot be decoded.
    #[error("invalid XML in {path}: {message}")]
    Xml {
        /// Path inside the OFD package.
        path: String,
        /// Parser diagnostic.
        message: String,
    },
    /// A required OFD structure is missing or inconsistent.
    #[error("invalid OFD structure at {path}: {message}")]
    InvalidStructure {
        /// Path inside the OFD package.
        path: String,
        /// Validation diagnostic.
        message: String,
    },
    /// A scalar or compound value has invalid syntax.
    #[error("invalid {field} value `{value}`{location}", location = invalid_value_location(.path))]
    InvalidValue {
        /// Field category.
        field: &'static str,
        /// Original value.
        value: String,
        /// Package path containing the invalid value, when available.
        path: Option<String>,
    },
    /// A requested package entry does not exist.
    #[error("missing OFD entry: {0}")]
    MissingEntry(String),
    /// A requested page index is outside the document.
    #[error("page index {index} is out of range for {page_count} pages")]
    PageOutOfRange {
        /// Requested zero-based index.
        index: usize,
        /// Available page count.
        page_count: usize,
    },
    /// A configured resource limit was exceeded.
    #[error("resource limit exceeded: {0}")]
    LimitExceeded(String),
    /// The document uses a feature that this version cannot process correctly.
    #[error("unsupported OFD feature: {0}")]
    UnsupportedFeature(String),
}

fn invalid_value_location(path: &Option<String>) -> String {
    path.as_ref()
        .map(|path| format!(" at {path}"))
        .unwrap_or_default()
}

/// Result type used by rofd-core.
pub type Result<T> = std::result::Result<T, Error>;
