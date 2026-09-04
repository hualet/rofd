#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod cairo_renderer;
mod display_list;

pub use cairo_renderer::{CairoRenderer, RenderOptions, RenderReport};
pub use display_list::{ClipPath, Command, DisplayList, RenderDiagnostic};

/// An error encountered while lowering or rendering a validated page.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A render option is outside its supported domain.
    #[error("invalid render option {field}: {value}")]
    InvalidOption {
        /// The invalid option name.
        field: &'static str,
        /// A description of the invalid value.
        value: String,
    },
    /// The requested page surface dimensions cannot be represented safely.
    #[error("invalid render surface size {width} by {height} pixels")]
    InvalidSurfaceSize {
        /// The requested width before integer conversion.
        width: f64,
        /// The requested height before integer conversion.
        height: f64,
    },
    /// A caller-provided image surface cannot contain the requested page.
    #[error(
        "render target is {actual_width} by {actual_height} pixels but requires at least {required_width} by {required_height}"
    )]
    SurfaceTooSmall {
        /// Required output width.
        required_width: i32,
        /// Required output height.
        required_height: i32,
        /// Actual image-surface width.
        actual_width: i32,
        /// Actual image-surface height.
        actual_height: i32,
    },
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
    /// The display list contains an invalid graphics-state sequence.
    #[error("invalid display list: {message}")]
    InvalidDisplayList {
        /// A description of the invalid sequence.
        message: String,
    },
    /// Cairo rejected a rendering operation.
    #[error("Cairo {operation} failed: {source}")]
    Backend {
        /// The operation being performed.
        operation: &'static str,
        /// The Cairo error.
        #[source]
        source: cairo::Error,
    },
}

/// A result produced by `rofd-render`.
pub type Result<T> = std::result::Result<T, Error>;
