use std::ops::Range;

use crate::{Error, PageObject, Point, Rect, Result, TextObject, Transform};

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

pub(crate) fn build_page_text(page: &crate::Page) -> Result<PageText> {
    let mut builder = TextBuilder::default();
    for layer in page.layers() {
        builder.objects(layer.objects(), Transform::IDENTITY)?;
    }
    for annotation in page.annotations() {
        if annotation.visible() {
            builder.objects(annotation.objects(), translation(annotation.boundary())?)?;
        }
    }
    Ok(builder.text)
}

#[derive(Default)]
struct TextBuilder {
    text: PageText,
}

impl TextBuilder {
    fn objects(&mut self, objects: &[PageObject], parent: Transform) -> Result<()> {
        for object in objects {
            match object {
                PageObject::Group(group) => self.objects(group.objects(), parent)?,
                PageObject::Composite(composite) if !composite.transform().is_singular() => {
                    let transform =
                        object_to_page(composite.transform(), composite.boundary(), parent)?;
                    if !transform.is_singular() {
                        self.objects(composite.objects(), transform)?;
                    }
                }
                PageObject::Text(text) if !text.transform().is_singular() => {
                    self.text_object(text, parent)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn text_object(&mut self, object: &TextObject, parent: Transform) -> Result<()> {
        if object.runs().iter().all(|run| run.text().is_empty()) {
            return Ok(());
        }
        let transform = object_to_page(object.transform(), object.boundary(), parent)?;
        if transform.is_singular() {
            return Ok(());
        }
        if !self.text.text.is_empty() {
            self.push('\n', None, TextCharFlags::SYNTHESIZED_SEPARATOR, None);
        }
        let size = object.font_size();
        for run in object.runs() {
            let (mut x, mut y) = (run.x(), run.y());
            for (index, character) in run.text().chars().enumerate() {
                let dx = if run.has_explicit_delta_x() {
                    run.delta_x()[index]
                } else {
                    size
                };
                let dy = if run.has_explicit_delta_y() {
                    run.delta_y()[index]
                } else {
                    0.0
                };
                let next_x = finite_add(x, dx)?;
                let next_y = finite_add(y, dy)?;
                // Bound the horizontal baseline sweep with a direction-independent
                // font-size footprint. Explicit zero advances still receive
                // nonempty conservative geometry.
                let left = x.min(next_x);
                let right = finite_add(left, dx.abs().max(size))?;
                let top = finite_add(y.min(next_y), -size)?;
                let bottom = y.max(next_y);
                let rect = transformed_box(left, top, right, bottom, transform)?;
                let flags = if character.is_whitespace() {
                    TextCharFlags::WHITESPACE
                } else {
                    TextCharFlags::default()
                };
                self.push(character, Some(rect), flags, Some(object.object_id()));
                (x, y) = (next_x, next_y);
            }
        }
        Ok(())
    }

    fn push(
        &mut self,
        character: char,
        rect_mm: Option<Rect>,
        flags: TextCharFlags,
        object_id: Option<u64>,
    ) {
        let start = self.text.text.len();
        self.text.text.push(character);
        self.text.characters.push(TextChar {
            utf8_range: start..self.text.text.len(),
            rect_mm,
            flags,
            precision: TextGeometryPrecision::Conservative,
            object_id,
        });
    }
}

fn translation(boundary: Rect) -> Result<Transform> {
    Transform::new(1.0, 0.0, 0.0, 1.0, boundary.x, boundary.y)
}

fn object_to_page(local: Transform, boundary: Rect, parent: Transform) -> Result<Transform> {
    local.then(translation(boundary)?)?.then(parent)
}

fn transformed_box(
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    transform: Transform,
) -> Result<Rect> {
    let corners = [(left, top), (right, top), (right, bottom), (left, bottom)];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in corners {
        let point = transform.apply(Point::new(x, y)?)?;
        min_x = min_x.min(point.x());
        min_y = min_y.min(point.y());
        max_x = max_x.max(point.x());
        max_y = max_y.max(point.y());
    }
    Ok(Rect {
        x: min_x,
        y: min_y,
        width: finite_add(max_x, -min_x)?,
        height: finite_add(max_y, -min_y)?,
    })
}

fn finite_add(left: f64, right: f64) -> Result<f64> {
    let sum = left + right;
    if sum.is_finite() {
        Ok(sum)
    } else {
        Err(Error::InvalidValue {
            field: "semantic text coordinate",
            value: format!("{left} + {right}"),
            path: None,
        })
    }
}
