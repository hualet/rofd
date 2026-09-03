#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod container;
mod document;
mod error;
mod geometry;
mod options;
mod paint;
mod path;
mod raw;

pub use document::{Document, Metadata, Page, Warning, WarningCode};
pub use error::{Error, Result};
pub use geometry::{Point, Rect, Transform};
pub use options::{LoadOptions, ResourceLimits, Strictness};
pub use paint::Color;

/// The crate API version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
