use std::path::PathBuf;

/// Errors returned while loading or querying an OFD document.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An internal synchronization primitive or invariant became unusable.
    #[error("internal rofd state error: {0}")]
    Internal(String),
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
    /// A resource object identifier is not declared by this document.
    #[error("unknown resource object ID {object_id}")]
    UnknownResource {
        /// Requested OFD object identifier.
        object_id: u64,
    },
    /// A resource exists but is not of the requested kind.
    #[error("resource object ID {object_id} is {actual:?}, expected {expected:?}")]
    ResourceKindMismatch {
        /// Requested OFD object identifier.
        object_id: u64,
        /// Kind required by the lookup operation.
        expected: crate::ResourceKind,
        /// Kind declared in the catalog.
        actual: crate::ResourceKind,
    },
    /// Two resource declarations use the same document-wide object identifier.
    #[error(
        "duplicate resource object ID {object_id}: first declared as {first_kind:?} in {first_path}, then as {duplicate_kind:?} in {duplicate_path}"
    )]
    DuplicateResourceId {
        /// Repeated OFD object identifier.
        object_id: u64,
        /// Package path of the first declaration.
        first_path: String,
        /// Resource kind of the first declaration.
        first_kind: crate::ResourceKind,
        /// Package path of the later declaration.
        duplicate_path: String,
        /// Resource kind of the later declaration.
        duplicate_kind: crate::ResourceKind,
    },
    /// A resource declaration is invalid or unsupported.
    #[error("invalid resource{resource_id} in {path}: {field}: {message}", resource_id = optional_resource_id(*.object_id))]
    InvalidResource {
        /// Resource catalog path.
        path: String,
        /// Declared object identifier, if it could be parsed.
        object_id: Option<u64>,
        /// Invalid field name.
        field: &'static str,
        /// Validation diagnostic.
        message: String,
    },
    /// A page object contains a value that cannot be represented safely.
    #[error("invalid page object {object_id} in {path}: {field}: {message}")]
    InvalidPageObject {
        /// Package path of the page or template containing the object.
        path: String,
        /// OFD object identifier.
        object_id: u64,
        /// Invalid field or child name.
        field: &'static str,
        /// Validation diagnostic.
        message: String,
    },
    /// A configured resource limit was exceeded.
    #[error("resource limit exceeded: {0}")]
    LimitExceeded(String),
    /// The document uses a feature that this version cannot process correctly.
    #[error("unsupported OFD feature: {0}")]
    UnsupportedFeature(String),
    /// A signed value (SES_Signature) cannot be parsed.
    #[error("invalid SES signature value: {0}")]
    InvalidSignatureValue(String),
}

fn optional_resource_id(id: Option<u64>) -> String {
    id.map(|id| format!(" object ID {id}")).unwrap_or_default()
}

fn invalid_value_location(path: &Option<String>) -> String {
    path.as_ref()
        .map(|path| format!(" at {path}"))
        .unwrap_or_default()
}

/// Result type used by rofd-core.
pub type Result<T> = std::result::Result<T, Error>;
