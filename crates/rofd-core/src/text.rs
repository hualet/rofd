use crate::{Clip, Color, Rect, StrokeStyle, Transform};

/// One source-ordered text run with fully resolved positioning values.
#[derive(Clone, Debug, PartialEq)]
pub struct TextCode {
    pub(crate) text: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) delta_x: Vec<f64>,
    pub(crate) delta_y: Vec<f64>,
    pub(crate) has_delta_x: bool,
    pub(crate) has_delta_y: bool,
}

impl TextCode {
    /// Returns the original Unicode scalar stream.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the explicit or inherited run origin on the x axis.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Returns the explicit or inherited run origin on the y axis.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Returns one x displacement slot per scalar, with omitted trailing values zero-filled.
    pub fn delta_x(&self) -> &[f64] {
        &self.delta_x
    }

    /// Returns one y displacement slot per scalar, with omitted trailing values zero-filled.
    pub fn delta_y(&self) -> &[f64] {
        &self.delta_y
    }

    /// Returns whether the source run supplied a nonempty `DeltaX` value.
    pub fn has_explicit_delta_x(&self) -> bool {
        self.has_delta_x
    }

    /// Returns whether the source run supplied a nonempty `DeltaY` value.
    pub fn has_explicit_delta_y(&self) -> bool {
        self.has_delta_y
    }
}

/// A validated mapping from a character range to explicit font glyph identifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterGlyphMap {
    pub(crate) code_position: usize,
    pub(crate) code_count: usize,
    pub(crate) glyphs: Vec<u32>,
}

impl CharacterGlyphMap {
    /// Returns the zero-based scalar position in the concatenated text stream.
    pub fn code_position(&self) -> usize {
        self.code_position
    }

    /// Returns the number of Unicode scalars replaced by this mapping.
    pub fn code_count(&self) -> usize {
        self.code_count
    }

    /// Returns the explicit glyph identifiers.
    pub fn glyphs(&self) -> &[u32] {
        &self.glyphs
    }
}

/// An immutable validated OFD text object.
#[derive(Clone, Debug, PartialEq)]
pub struct TextObject {
    pub(crate) object_id: u64,
    pub(crate) boundary: Rect,
    pub(crate) transform: Transform,
    pub(crate) font_id: u64,
    pub(crate) font_size: f64,
    pub(crate) stroke: Option<Color>,
    pub(crate) fill: Option<Color>,
    pub(crate) stroke_style: StrokeStyle,
    pub(crate) clips: Vec<Clip>,
    pub(crate) runs: Vec<TextCode>,
    pub(crate) glyph_maps: Vec<CharacterGlyphMap>,
}

impl TextObject {
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
    /// Returns the validated font resource identifier.
    pub fn font_id(&self) -> u64 {
        self.font_id
    }
    /// Returns the positive font size in millimetres.
    pub fn font_size(&self) -> f64 {
        self.font_size
    }
    /// Returns the effective stroke color when stroking is enabled.
    pub fn stroke(&self) -> Option<Color> {
        self.stroke
    }
    /// Returns the effective fill color when filling is enabled.
    pub fn fill(&self) -> Option<Color> {
        self.fill
    }
    /// Returns the effective stroke geometry.
    pub fn stroke_style(&self) -> &StrokeStyle {
        &self.stroke_style
    }
    /// Returns source-ordered clipping intersection operands.
    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }
    /// Returns source-ordered positioned text runs.
    pub fn runs(&self) -> &[TextCode] {
        &self.runs
    }
    /// Returns source-ordered explicit character-to-glyph mappings.
    pub fn glyph_maps(&self) -> &[CharacterGlyphMap] {
        &self.glyph_maps
    }
}
