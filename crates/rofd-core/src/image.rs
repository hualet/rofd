use crate::paint::StrokeStyle;
use crate::{Clip, Color, ImageFormat, Rect, Transform};

/// A validated image border (GB/T 33190-2016 table 43).
///
/// The border runs along the image boundary in the object's local
/// coordinate system, so a non-uniform CTM scales it with the image.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageBorder {
    line_width: f64,
    horizontal_corner_radius: f64,
    vertical_corner_radius: f64,
    stroke: StrokeStyle,
    color: Color,
}

impl ImageBorder {
    pub(crate) fn new(
        line_width: f64,
        horizontal_corner_radius: f64,
        vertical_corner_radius: f64,
        stroke: StrokeStyle,
        color: Color,
    ) -> Self {
        Self {
            line_width,
            horizontal_corner_radius,
            vertical_corner_radius,
            stroke,
            color,
        }
    }

    /// Returns the border line width in millimetres; zero suppresses drawing.
    pub fn line_width(&self) -> f64 {
        self.line_width
    }

    /// Returns the horizontal corner radius in millimetres.
    pub fn horizontal_corner_radius(&self) -> f64 {
        self.horizontal_corner_radius
    }

    /// Returns the vertical corner radius in millimetres.
    pub fn vertical_corner_radius(&self) -> f64 {
        self.vertical_corner_radius
    }

    /// Returns the effective stroke geometry, including dash style.
    pub fn stroke_style(&self) -> &StrokeStyle {
        &self.stroke
    }

    /// Returns the border colour; the standard default is black.
    pub fn color(&self) -> Color {
        self.color
    }
}

/// An immutable validated OFD raster-image object.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageObject {
    pub(crate) object_id: u64,
    pub(crate) boundary: Rect,
    pub(crate) transform: Transform,
    pub(crate) resource_id: u64,
    pub(crate) resource_format: ImageFormat,
    pub(crate) alpha: u8,
    pub(crate) clips: Vec<Clip>,
    pub(crate) substitution_id: Option<u64>,
    pub(crate) image_mask_id: Option<u64>,
    pub(crate) border: Option<ImageBorder>,
}

impl ImageObject {
    /// Returns the OFD object identifier.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }
    /// Returns the finite positive object boundary.
    pub fn boundary(&self) -> Rect {
        self.boundary
    }
    /// Returns the object transform.
    pub fn transform(&self) -> Transform {
        self.transform
    }
    /// Returns the primary image resource identifier.
    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }
    /// Returns the declared format of the primary image resource.
    pub fn resource_format(&self) -> ImageFormat {
        self.resource_format
    }
    /// Returns the object alpha value.
    pub fn alpha(&self) -> u8 {
        self.alpha
    }
    /// Returns source-ordered clipping intersection operands.
    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }
    /// Returns the optional substitution image identifier.
    pub fn substitution_id(&self) -> Option<u64> {
        self.substitution_id
    }
    /// Returns the optional image-mask identifier.
    pub fn image_mask_id(&self) -> Option<u64> {
        self.image_mask_id
    }
    /// Returns the validated image border, when one was declared.
    pub fn border(&self) -> Option<&ImageBorder> {
        self.border.as_ref()
    }
}
