#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod display_list;

pub use display_list::{Command, DisplayList, RenderDiagnostic};

/// An error encountered while lowering a validated page.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A page object contains a value that cannot form a valid display command.
    #[error("path object {object_id} has invalid {field}: {value}")]
    InvalidModel {
        /// The OFD object identifier containing the invalid value.
        object_id: u64,
        /// The invalid model field.
        field: &'static str,
        /// A description of the invalid value.
        value: String,
    },
}

/// A result produced by `rofd-render`.
pub type Result<T> = std::result::Result<T, Error>;
