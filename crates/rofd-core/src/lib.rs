#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Safe, read-only OFD document model.

mod error;
mod geometry;
mod options;
mod path;

pub use error::{Error, Result};
pub use geometry::Rect;
pub use options::{LoadOptions, ResourceLimits, Strictness};

/// The crate API version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
