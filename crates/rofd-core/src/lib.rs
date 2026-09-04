#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod container;
mod content;
mod document;
mod error;
mod geometry;
mod options;
mod paint;
mod path;
mod path_data;
mod raw;
mod resources;

pub use content::{
    Clip, ClipPath, FillRule, Layer, LayerSource, LayerType, PageGroup, PageObject, PathObject,
    UnsupportedObject, UnsupportedObjectKind,
};
pub use document::{Document, Metadata, Page, Warning, WarningCode};
pub use error::{Error, Result};
pub use geometry::{Point, Rect, Transform};
pub use options::{LoadOptions, ResourceLimits, Strictness};
pub use paint::Color;
pub use path_data::{PathCommand, PathData};
pub use resources::{FontResource, ImageFormat, ImageResource, ResourceKind};

/// The crate API version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
