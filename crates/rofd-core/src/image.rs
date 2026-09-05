use crate::{Clip, ImageFormat, Rect, Transform};

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
    pub(crate) has_border: bool,
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
    /// Returns whether an OFD Border child was explicitly present.
    pub fn has_border(&self) -> bool {
        self.has_border
    }
}
