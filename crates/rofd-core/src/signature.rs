//! Public model for signature stamp annotations.
//!
//! Stamp annotations place an electronic-seal picture onto a page. They are
//! declared by the signatures listed in `OFD.xml` and their pictures come from
//! each signature's `SignedValue.dat` (a DER-encoded SES_Signature). rofd only
//! exposes and renders the pictures; it never verifies signatures.

use crate::Rect;

/// A stamp annotation declared by one document signature.
#[derive(Clone, Debug)]
pub struct StampAnnotation {
    /// Object identifier of the page the stamp is painted on.
    pub page_ref: u64,
    /// Producer-assigned annotation identifier.
    pub id: Option<String>,
    /// Stamp rectangle in page-space millimetres; the picture is stretched to fill it.
    pub boundary: Rect,
    /// Optional clip rectangle relative to `boundary`'s top-left corner.
    pub clip: Option<Rect>,
    /// The seal picture painted into `boundary`.
    pub picture: SealPicture,
}

/// One electronic-seal picture extracted from a signed value.
#[derive(Clone, Debug)]
pub struct SealPicture {
    /// Detected picture encoding.
    pub kind: SealPictureKind,
    /// Encoded picture bytes: a mini OFD package for [`SealPictureKind::Ofd`],
    /// otherwise an encoded raster image.
    pub data: Vec<u8>,
    /// Producer-declared picture width in millimetres.
    pub width_mm: Option<f64>,
    /// Producer-declared picture height in millimetres.
    pub height_mm: Option<f64>,
}

/// The encoding of a [`SealPicture`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SealPictureKind {
    /// A complete mini OFD package (ZIP bytes).
    Ofd,
    /// PNG raster image.
    Png,
    /// JPEG raster image.
    Jpeg,
    /// GIF raster image.
    Gif,
    /// BMP raster image.
    Bmp,
    /// Any other producer-declared picture type, preserved verbatim.
    Other(String),
}

impl SealPictureKind {
    pub(crate) fn from_type_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "ofd" => Self::Ofd,
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "gif" => Self::Gif,
            "bmp" => Self::Bmp,
            _ => Self::Other(name.to_owned()),
        }
    }
}
