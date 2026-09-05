#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod cairo_renderer;
mod display_list;
mod fonts;

pub use cairo_renderer::{CairoRenderer, RenderOptions, RenderReport};
pub use display_list::{ClipPath, Command, DisplayList, RenderDiagnostic};
pub use fonts::{
    position_glyph_runs, FontDiagnostic, FontIdentity, FontResolver, FontSource, GlyphRun,
    PositionedGlyph, ResolvedFont, SystemFontResolver,
};

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
    /// A geometric primitive cannot be represented numerically by the backend.
    #[error("invalid {primitive} geometry in {field}: {value}")]
    InvalidGeometry {
        /// The primitive whose geometry failed, such as `arc`.
        primitive: &'static str,
        /// The input or derived field that was invalid.
        field: &'static str,
        /// A description of the invalid value.
        value: String,
    },
    /// The configured memory budget cannot contain the renderer's worst-case surfaces.
    #[error("raster surfaces require {required_bytes} bytes but the limit is {max_bytes}")]
    RasterBudgetExceeded {
        /// Worst-case simultaneously live bytes required for this page.
        required_bytes: u64,
        /// Configured maximum simultaneously live raster bytes.
        max_bytes: u64,
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
    /// A primary failure was followed by another failure while restoring state.
    #[error("{operation} failed after {primary}: {cleanup}")]
    Cleanup {
        /// The cleanup operation that failed.
        operation: &'static str,
        /// The error that initiated cleanup.
        primary: Box<Error>,
        /// The cleanup error.
        cleanup: Box<Error>,
    },
    /// Font bytes or a selected face cannot be used safely.
    #[error("invalid font {identity}: {message}")]
    InvalidFont {
        /// Stable non-path identity of the font.
        identity: String,
        /// Backend validation detail.
        message: String,
    },
    /// An explicit OFD glyph identifier is outside the selected face.
    #[error("glyph ID {glyph_id} is invalid for font {identity} with {glyph_count} glyphs")]
    InvalidGlyph {
        /// Stable non-path identity of the font.
        identity: String,
        /// Invalid glyph identifier.
        glyph_id: u32,
        /// Number of glyphs advertised by the selected face.
        glyph_count: u32,
    },
    /// Text positioning produced an invalid value or exceeded a semantic bound.
    #[error("invalid text layout for object {object_id} in {field}: {message}")]
    InvalidTextLayout {
        /// OFD text object identifier.
        object_id: u64,
        /// Stable field or budget name.
        field: &'static str,
        /// Validation detail.
        message: String,
    },
    /// A system font exceeds the configured encoded-byte limit.
    #[error("font {identity} has {actual_bytes} bytes but the limit is {max_bytes}")]
    FontBytesExceeded {
        /// Stable non-path font identity.
        identity: String,
        /// Encoded byte length.
        actual_bytes: u64,
        /// Configured maximum encoded byte length.
        max_bytes: u64,
    },
    /// Internal font cache synchronization failed.
    #[error("font cache unavailable: {message}")]
    FontCache {
        /// Synchronization detail.
        message: String,
    },
    /// A font resource was paired with a text object referencing another identifier.
    #[error(
        "text object {text_object_id} references font {expected_font_id}, not resource {actual_resource_id}"
    )]
    FontResourceMismatch {
        /// OFD text object identifier.
        text_object_id: u64,
        /// Font identifier referenced by the text object.
        expected_font_id: u64,
        /// Font resource identifier supplied by the caller.
        actual_resource_id: u64,
    },
}

/// A result produced by `rofd-render`.
pub type Result<T> = std::result::Result<T, Error>;
