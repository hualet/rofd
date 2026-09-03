#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Safe, read-only OFD document model.

mod container;
mod document;
mod error;
mod geometry;
mod options;
mod path;
mod raw;

pub use document::{Document, Metadata};
pub use error::{Error, Result};
pub use geometry::Rect;
pub use options::{LoadOptions, ResourceLimits, Strictness};

/// The crate API version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
