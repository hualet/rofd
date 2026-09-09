use std::ops::Range;

use crate::Rect;

/// Describes the fidelity of a character's reported geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextGeometryPrecision {
    /// Geometry precisely represents the character's painted bounds.
    Exact,
    /// Geometry is a nonempty approximation derived from available OFD data.
    Conservative,
}

/// Bit flags that describe special properties of a semantic text character.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextCharFlags(u32);

impl TextCharFlags {
    /// Marks a separator inserted while flattening distinct source text objects.
    pub const SYNTHESIZED_SEPARATOR: Self = Self(1 << 0);

    /// Marks a whitespace character from the source text.
    pub const WHITESPACE: Self = Self(1 << 1);

    /// Returns whether all flags in `other` are set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns the raw bit representation.
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// One Unicode scalar value in flattened page text and its source metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct TextChar {
    utf8_range: Range<usize>,
    rect_mm: Option<Rect>,
    flags: TextCharFlags,
    precision: TextGeometryPrecision,
    object_id: Option<u64>,
}

impl TextChar {
    /// Returns this character's byte range in [`PageText::as_str`].
    pub fn utf8_range(&self) -> Range<usize> {
        self.utf8_range.clone()
    }

    /// Returns the character's bounding rectangle in physical-page millimetre
    /// coordinates after transforms, when known. Synthesized separators have no
    /// rectangle.
    pub fn rect_mm(&self) -> Option<Rect> {
        self.rect_mm
    }

    /// Returns flags describing this character.
    pub fn flags(&self) -> TextCharFlags {
        self.flags
    }

    /// Returns the fidelity of this character's geometry.
    pub fn geometry_precision(&self) -> TextGeometryPrecision {
        self.precision
    }

    /// Returns the source `TextObject` identifier, if this character was not synthesized.
    pub fn object_id(&self) -> Option<u64> {
        self.object_id
    }
}

/// Immutable flattened text and per-character metadata for one page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageText {
    text: String,
    characters: Vec<TextChar>,
}

impl PageText {
    /// Returns the flattened UTF-8 page text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns metadata for each Unicode scalar value in [`Self::as_str`].
    pub fn characters(&self) -> &[TextChar] {
        &self.characters
    }
}
