use std::collections::HashSet;

use crate::paint::PaintParameters;
use crate::raw;
use crate::{
    CharacterGlyphMap, Color, Error, ImageObject, LineCap, LineJoin, PathData, Rect, ResourceKind,
    ResourceLimits, Result, StrokeStyle, TextCode, TextObject, Transform,
};

/// The stacking category assigned to a page layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerType {
    /// Content painted behind the page body.
    Background,
    /// Ordinary page content.
    Body,
    /// Content painted in front of the page body.
    Foreground,
}

/// The package source that contributed an effective page layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerSource {
    /// A layer loaded from the template with the given object identifier.
    Template(u64),
    /// A layer declared directly by the real page.
    Page,
}

/// An ordered layer of page objects.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    object_id: u64,
    kind: LayerType,
    source: LayerSource,
    objects: Vec<PageObject>,
}

impl Layer {
    /// Returns the OFD object identifier.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the stacking category.
    pub fn kind(&self) -> LayerType {
        self.kind
    }

    /// Returns whether this layer came from the real page or a template.
    pub fn source(&self) -> LayerSource {
        self.source
    }

    /// Returns child objects in source order.
    pub fn objects(&self) -> &[PageObject] {
        &self.objects
    }
}

/// A page-level graphic object.
#[derive(Clone, Debug, PartialEq)]
pub enum PageObject {
    /// A supported vector path.
    Path(PathObject),
    /// A validated text object retained for later shaping and rendering.
    Text(TextObject),
    /// A validated encoded-image reference retained for later decoding and rendering.
    Image(ImageObject),
    /// An ordered group originating from a page block.
    Group(PageGroup),
    /// A known object kind not rendered by this version.
    Unsupported(UnsupportedObject),
}

impl PageObject {
    /// Returns the OFD object identifier.
    pub fn object_id(&self) -> u64 {
        match self {
            Self::Path(object) => object.object_id(),
            Self::Text(object) => object.object_id(),
            Self::Image(object) => object.object_id(),
            Self::Group(object) => object.object_id(),
            Self::Unsupported(object) => object.object_id(),
        }
    }
}

/// An ordered group of page objects.
#[derive(Clone, Debug, PartialEq)]
pub struct PageGroup {
    object_id: u64,
    objects: Vec<PageObject>,
}

impl PageGroup {
    /// Returns the OFD object identifier.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns child objects in source order.
    pub fn objects(&self) -> &[PageObject] {
        &self.objects
    }
}

/// A known page object retained without interpreting its payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedObject {
    object_id: u64,
    kind: UnsupportedObjectKind,
}

impl UnsupportedObject {
    /// Returns the OFD object identifier.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the unsupported object category.
    pub fn kind(&self) -> UnsupportedObjectKind {
        self.kind
    }
}

/// Known graphic object categories retained as explicit placeholders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedObjectKind {
    /// A text object not yet rendered by a downstream backend.
    Text,
    /// An image object not yet rendered by a downstream backend.
    Image,
    /// A composite object.
    Composite,
}

/// The algorithm used to determine the interior of a path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillRule {
    /// The non-zero winding rule.
    NonZero,
    /// The even-odd rule.
    EvenOdd,
}

/// A validated vector path object.
#[derive(Clone, Debug, PartialEq)]
pub struct PathObject {
    object_id: u64,
    boundary: Rect,
    transform: Transform,
    path_data: PathData,
    stroke: Option<Color>,
    fill: Option<Color>,
    line_width: f64,
    stroke_style: StrokeStyle,
    fill_rule: FillRule,
    clips: Vec<Clip>,
}

impl PathObject {
    /// Returns the OFD object identifier.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the object boundary with a finite origin and positive dimensions.
    pub fn boundary(&self) -> Rect {
        self.boundary
    }

    /// Returns the object transform.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Returns the validated abbreviated path.
    pub fn path_data(&self) -> &PathData {
        &self.path_data
    }

    /// Returns the stroke color when stroking is enabled.
    pub fn stroke(&self) -> Option<Color> {
        self.stroke
    }

    /// Returns the fill color when filling is enabled.
    pub fn fill(&self) -> Option<Color> {
        self.fill
    }

    /// Returns the positive line width in millimetres.
    pub fn line_width(&self) -> f64 {
        self.line_width
    }

    /// Returns the effective validated stroke geometry.
    pub fn stroke_style(&self) -> &StrokeStyle {
        &self.stroke_style
    }

    /// Returns the path fill rule.
    pub fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    /// Returns source-ordered clipping intersection operands.
    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }
}

/// One clipping intersection operand containing unioned path areas.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    paths: Vec<ClipPath>,
    affected_by_object_transform: bool,
}

impl Clip {
    /// Returns path areas whose filled regions are unioned for this operand.
    pub fn paths(&self) -> &[ClipPath] {
        &self.paths
    }

    /// Returns whether the owning object's CTM affects this clip.
    pub fn affected_by_object_transform(&self) -> bool {
        self.affected_by_object_transform
    }
}

/// A validated fill-only path area used by a clipping operand.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipPath {
    boundary: Rect,
    transform: Transform,
    area_transform: Transform,
    path_data: PathData,
    fill_rule: FillRule,
}

impl ClipPath {
    /// Returns the clip path boundary relative to its owning object.
    pub fn boundary(&self) -> Rect {
        self.boundary
    }

    /// Returns the clip path CTM, or identity when it was omitted.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Returns the containing Area CTM, or identity when it was omitted.
    pub fn area_transform(&self) -> Transform {
        self.area_transform
    }

    /// Returns the validated abbreviated clip path.
    pub fn path_data(&self) -> &PathData {
        &self.path_data
    }

    /// Returns the rule used to fill the clip path.
    pub fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }
}

pub(crate) fn convert_layers(
    content: Option<raw::PageContent>,
    document: &crate::Document,
    limits: &ResourceLimits,
    path: &str,
    source: LayerSource,
) -> Result<(Vec<Layer>, ContentUsage)> {
    let Some(content) = content else {
        return Ok((Vec::new(), ContentUsage::default()));
    };
    let mut context = ConversionContext {
        limits,
        document,
        path,
        object_ids: HashSet::new(),
        remaining_path_commands: limits.max_path_commands,
        remaining_text_characters: limits.max_text_characters_per_page,
        remaining_glyphs: limits.max_glyphs_per_page,
        remaining_text_expansion_entries: limits.max_text_expansion_entries,
    };
    let mut layers = Vec::new();
    for layer in content.layers {
        let object_id = parse_object_id(&layer.id)?;
        context.register_id(object_id)?;
        let kind = match layer.kind.as_deref() {
            None | Some("Body") => LayerType::Body,
            Some("Background") => LayerType::Background,
            Some("Foreground") => LayerType::Foreground,
            Some(value) => return Err(invalid_value("layer type", value)),
        };
        let objects = context.convert_objects(layer.objects)?;
        layers.push(Layer {
            object_id,
            kind,
            source,
            objects,
        });
    }
    let usage = ContentUsage::from_layers(&layers);
    Ok((layers, usage))
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ContentUsage {
    pub(crate) page_objects: usize,
    pub(crate) path_commands: usize,
    pub(crate) text_characters: usize,
    pub(crate) glyphs: usize,
    pub(crate) text_expansion_entries: usize,
}

impl ContentUsage {
    pub(crate) const fn template_reference() -> Self {
        Self {
            page_objects: 1,
            path_commands: 0,
            text_characters: 0,
            glyphs: 0,
            text_expansion_entries: 0,
        }
    }

    pub(crate) fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            page_objects: self.page_objects.checked_add(other.page_objects)?,
            path_commands: self.path_commands.checked_add(other.path_commands)?,
            text_characters: self.text_characters.checked_add(other.text_characters)?,
            glyphs: self.glyphs.checked_add(other.glyphs)?,
            text_expansion_entries: self
                .text_expansion_entries
                .checked_add(other.text_expansion_entries)?,
        })
    }

    fn from_layers(layers: &[Layer]) -> Self {
        let mut usage = Self::default();
        for layer in layers {
            usage.page_objects += 1;
            for object in &layer.objects {
                usage.add_object(object);
            }
        }
        usage
    }

    fn add_object(&mut self, object: &PageObject) {
        self.page_objects += 1;
        match object {
            PageObject::Path(path) => {
                self.path_commands += path.path_data.commands().len();
                for clip in &path.clips {
                    self.page_objects += 1;
                    for clip_path in &clip.paths {
                        self.page_objects += 2;
                        self.path_commands += clip_path.path_data.commands().len();
                    }
                }
            }
            PageObject::Group(group) => {
                for child in &group.objects {
                    self.add_object(child);
                }
            }
            PageObject::Text(text) => {
                let characters = text
                    .runs
                    .iter()
                    .map(|run| run.text.chars().count())
                    .sum::<usize>();
                self.text_characters += characters;
                let mapped_characters = text
                    .glyph_maps
                    .iter()
                    .map(|map| map.code_count)
                    .sum::<usize>();
                let mapped_glyphs = text
                    .glyph_maps
                    .iter()
                    .map(|map| map.glyphs.len())
                    .sum::<usize>();
                self.glyphs += characters - mapped_characters + mapped_glyphs;
                self.text_expansion_entries += text
                    .runs
                    .iter()
                    .map(|run| run.delta_x.len() + run.delta_y.len())
                    .sum::<usize>()
                    + text
                        .glyph_maps
                        .iter()
                        .map(|map| map.glyphs.len())
                        .sum::<usize>();
                self.add_clips(&text.clips);
            }
            PageObject::Image(image) => self.add_clips(&image.clips),
            PageObject::Unsupported(_) => {}
        }
    }

    fn add_clips(&mut self, clips: &[Clip]) {
        for clip in clips {
            self.page_objects += 1;
            for clip_path in &clip.paths {
                self.page_objects += 2;
                self.path_commands += clip_path.path_data.commands().len();
            }
        }
    }
}

struct ConversionContext<'a> {
    limits: &'a ResourceLimits,
    document: &'a crate::Document,
    path: &'a str,
    object_ids: HashSet<u64>,
    remaining_path_commands: usize,
    remaining_text_characters: usize,
    remaining_glyphs: usize,
    remaining_text_expansion_entries: usize,
}

impl ConversionContext<'_> {
    fn register_id(&mut self, id: u64) -> Result<()> {
        if self.object_ids.len() >= self.limits.max_page_objects {
            return Err(Error::LimitExceeded(format!(
                "page object count {} exceeds limit {}",
                self.object_ids.len().saturating_add(1),
                self.limits.max_page_objects
            )));
        }
        if !self.object_ids.insert(id) {
            return Err(Error::InvalidStructure {
                path: self.path.to_owned(),
                message: format!("duplicate object ID {id}"),
            });
        }
        Ok(())
    }

    fn convert_objects(&mut self, objects: Vec<raw::GraphicUnit>) -> Result<Vec<PageObject>> {
        let mut converted = Vec::new();
        for object in objects {
            let object = match object {
                raw::GraphicUnit::Path(path) => {
                    let object_id = parse_object_id(&path.id)?;
                    self.register_id(object_id)?;
                    PageObject::Path(self.convert_path(*path, object_id)?)
                }
                raw::GraphicUnit::Group(group) => {
                    let object_id = parse_object_id(&group.id)?;
                    self.register_id(object_id)?;
                    let objects = self.convert_objects(group.objects)?;
                    PageObject::Group(PageGroup { object_id, objects })
                }
                raw::GraphicUnit::Text(object) => {
                    let object_id = parse_object_id(&object.id)?;
                    self.register_id(object_id)?;
                    let object = object.object.ok_or_else(|| Error::InvalidStructure {
                        path: self.path.to_owned(),
                        message: format!("TextObject {object_id} payload was not parsed"),
                    })?;
                    PageObject::Text(self.convert_text(*object, object_id)?)
                }
                raw::GraphicUnit::Image(object) => {
                    let object_id = parse_object_id(&object.id)?;
                    self.register_id(object_id)?;
                    let object = object.object.ok_or_else(|| Error::InvalidStructure {
                        path: self.path.to_owned(),
                        message: format!("ImageObject {object_id} payload was not parsed"),
                    })?;
                    PageObject::Image(self.convert_image(*object, object_id)?)
                }
                raw::GraphicUnit::Composite(object) => {
                    self.unsupported(&object.id, UnsupportedObjectKind::Composite)?
                }
            };
            converted.push(object);
        }
        Ok(converted)
    }

    fn unsupported(&mut self, value: &str, kind: UnsupportedObjectKind) -> Result<PageObject> {
        let id = parse_object_id(value)?;
        self.register_id(id)?;
        Ok(PageObject::Unsupported(UnsupportedObject {
            object_id: id,
            kind,
        }))
    }

    fn convert_path(&mut self, path: raw::PathObject, object_id: u64) -> Result<PathObject> {
        let boundary =
            Rect::parse(&path.boundary).map_err(|_| invalid_value("boundary", &path.boundary))?;
        if boundary.width <= 0.0 || boundary.height <= 0.0 {
            return Err(invalid_value("boundary", &path.boundary));
        }
        let transform = path
            .transform
            .as_deref()
            .map(Transform::parse)
            .transpose()?
            .unwrap_or(Transform::IDENTITY);
        let stroke_enabled = parse_bool(path.stroke.as_deref(), true, "stroke")?;
        let fill_enabled = parse_bool(path.fill.as_deref(), false, "fill")?;
        let object_alpha = parse_alpha(path.alpha.as_deref())?;
        let mut parameters = self.resolve_draw_param(path.draw_param.as_deref(), object_id)?;
        apply_local_stroke_style(
            &mut parameters,
            path.line_width.as_deref(),
            path.line_join.as_deref(),
            path.line_cap.as_deref(),
            path.dash_offset.as_deref(),
            path.dash_pattern.as_deref(),
            path.miter_limit.as_deref(),
            self.path,
            object_id,
        )?;
        let stroke = stroke_enabled
            .then(|| {
                effective_paint_color(
                    path.stroke_color.as_ref(),
                    parameters.stroke_color,
                    Color::BLACK,
                    object_alpha,
                    self.path,
                    object_id,
                    "StrokeColor",
                )
            })
            .transpose()?;
        let fill = fill_enabled
            .then(|| {
                effective_paint_color(
                    path.fill_color.as_ref(),
                    parameters.fill_color,
                    Color {
                        alpha: 0,
                        ..Color::BLACK
                    },
                    object_alpha,
                    self.path,
                    object_id,
                    "FillColor",
                )
            })
            .transpose()?;
        let stroke_style = parameters.stroke_style();
        let line_width = stroke_style.line_width();
        let fill_rule = match path.fill_rule.as_deref() {
            None | Some("NonZero") => FillRule::NonZero,
            Some("Even-Odd") => FillRule::EvenOdd,
            Some(value) => return Err(invalid_value("fill rule", value)),
        };
        let path_data =
            PathData::parse_with_limit(&path.abbreviated_data, self.remaining_path_commands)?;
        self.remaining_path_commands = self
            .remaining_path_commands
            .checked_sub(path_data.commands().len())
            .ok_or_else(|| Error::LimitExceeded("page path command budget exhausted".to_owned()))?;
        let clips = self.convert_clips(path.clips)?;

        Ok(PathObject {
            object_id,
            boundary,
            transform,
            path_data,
            stroke,
            fill,
            line_width,
            stroke_style,
            fill_rule,
            clips,
        })
    }

    fn resolve_draw_param(&self, value: Option<&str>, object_id: u64) -> Result<PaintParameters> {
        let Some(value) = value else {
            return Ok(PaintParameters::default());
        };
        let id = parse_nonzero_id(value, "DrawParam", self.path, object_id)?;
        self.document
            .draw_param(id)
            .map_err(|error| reference_error(error, self.path, object_id, "DrawParam"))
    }

    fn convert_text(&mut self, text: raw::TextObject, object_id: u64) -> Result<TextObject> {
        let boundary = parse_positive_boundary(&text.boundary, self.path, object_id)?;
        let transform = parse_transform(text.transform.as_deref(), self.path, object_id)?;
        let font_id = parse_nonzero_id(&text.font, "Font", self.path, object_id)?;
        self.require_resource_kind(font_id, ResourceKind::Font, object_id, "Font")?;
        let font_size = parse_positive_number(&text.size, "Size", self.path, object_id)?;
        let stroke_enabled = parse_bool(text.stroke.as_deref(), false, "stroke")?;
        let fill_enabled = parse_bool(text.fill.as_deref(), true, "fill")?;
        let object_alpha = parse_alpha(text.alpha.as_deref())?;
        let mut parameters = self.resolve_draw_param(text.draw_param.as_deref(), object_id)?;
        apply_local_stroke_style(
            &mut parameters,
            text.line_width.as_deref(),
            text.line_join.as_deref(),
            text.line_cap.as_deref(),
            text.dash_offset.as_deref(),
            text.dash_pattern.as_deref(),
            text.miter_limit.as_deref(),
            self.path,
            object_id,
        )?;

        if text.text_codes.is_empty() {
            return Err(object_error(
                self.path,
                object_id,
                "TextCode",
                "at least one text run is required".to_owned(),
            ));
        }

        let stroke = stroke_enabled
            .then(|| {
                effective_paint_color(
                    text.stroke_color.as_ref(),
                    parameters.stroke_color,
                    Color::BLACK,
                    object_alpha,
                    self.path,
                    object_id,
                    "StrokeColor",
                )
            })
            .transpose()?;
        let fill = fill_enabled
            .then(|| {
                effective_paint_color(
                    text.fill_color.as_ref(),
                    parameters.fill_color,
                    Color::BLACK,
                    object_alpha,
                    self.path,
                    object_id,
                    "FillColor",
                )
            })
            .transpose()?;
        let clips = self.convert_clips(text.clips)?;
        let runs = self.convert_text_runs(text.text_codes, object_id)?;
        let character_count = runs
            .iter()
            .map(|run| run.text.chars().count())
            .sum::<usize>();
        let glyph_maps = self.convert_glyph_maps(text.cg_transforms, character_count, object_id)?;
        let mapped_characters = glyph_maps
            .iter()
            .try_fold(0usize, |total, map| total.checked_add(map.code_count))
            .ok_or_else(|| Error::LimitExceeded("mapped character count overflow".to_owned()))?;
        let mapped_glyphs = glyph_maps
            .iter()
            .try_fold(0usize, |total, map| total.checked_add(map.glyphs.len()))
            .ok_or_else(|| Error::LimitExceeded("mapped glyph count overflow".to_owned()))?;
        let glyph_count = character_count
            .checked_sub(mapped_characters)
            .and_then(|count| count.checked_add(mapped_glyphs))
            .ok_or_else(|| Error::LimitExceeded("effective glyph count overflow".to_owned()))?;
        consume(&mut self.remaining_glyphs, glyph_count, "page glyph")?;

        Ok(TextObject {
            object_id,
            boundary,
            transform,
            font_id,
            font_size,
            stroke,
            fill,
            stroke_style: parameters.stroke_style(),
            clips,
            runs,
            glyph_maps,
        })
    }

    fn convert_text_runs(
        &mut self,
        raw_runs: Vec<raw::TextCode>,
        object_id: u64,
    ) -> Result<Vec<TextCode>> {
        let mut runs = Vec::with_capacity(raw_runs.len());
        let mut inherited_x = None;
        let mut inherited_y = None;
        for (index, run) in raw_runs.into_iter().enumerate() {
            let explicit_x =
                parse_optional_finite(run.x.as_deref(), "TextCode.X", self.path, object_id)?;
            let explicit_y =
                parse_optional_finite(run.y.as_deref(), "TextCode.Y", self.path, object_id)?;
            if index == 0 && (explicit_x.is_none() || explicit_y.is_none()) {
                return Err(object_error(
                    self.path,
                    object_id,
                    "TextCode origin",
                    "the first run must specify both X and Y; later runs inherit each omitted coordinate".to_owned(),
                ));
            }
            if explicit_x.is_some() {
                inherited_x = explicit_x;
            }
            if explicit_y.is_some() {
                inherited_y = explicit_y;
            }
            let character_count = run.text.chars().count();
            consume(
                &mut self.remaining_text_characters,
                character_count,
                "page text character",
            )?;
            let delta_count = character_count;
            let delta_x = parse_delta(
                run.delta_x.as_deref(),
                delta_count,
                &mut self.remaining_text_expansion_entries,
                self.path,
                object_id,
                "DeltaX",
            )?;
            let delta_y = parse_delta(
                run.delta_y.as_deref(),
                delta_count,
                &mut self.remaining_text_expansion_entries,
                self.path,
                object_id,
                "DeltaY",
            )?;
            runs.push(TextCode {
                text: run.text,
                x: inherited_x.expect("first run checked"),
                y: inherited_y.expect("first run checked"),
                delta_x,
                delta_y,
            });
        }
        Ok(runs)
    }

    fn convert_glyph_maps(
        &mut self,
        raw_maps: Vec<raw::CgTransform>,
        character_count: usize,
        object_id: u64,
    ) -> Result<Vec<CharacterGlyphMap>> {
        let mut maps = Vec::with_capacity(raw_maps.len());
        let mut ranges = Vec::with_capacity(raw_maps.len());
        for raw in raw_maps {
            let code_position = parse_usize(
                &raw.code_position,
                "CodePosition",
                self.path,
                object_id,
                true,
            )?;
            let code_count = raw
                .code_count
                .as_deref()
                .map(|value| parse_usize(value, "CodeCount", self.path, object_id, false))
                .transpose()?
                .unwrap_or(1);
            let glyph_count = raw
                .glyph_count
                .as_deref()
                .map(|value| parse_usize(value, "GlyphCount", self.path, object_id, false))
                .transpose()?
                .unwrap_or(1);
            let glyph_text = raw.glyphs.ok_or_else(|| {
                object_error(
                    self.path,
                    object_id,
                    "Glyphs",
                    "required child is missing".to_owned(),
                )
            })?;
            let actual_glyph_count = glyph_text.split_whitespace().count();
            if actual_glyph_count != glyph_count {
                return Err(object_error(
                    self.path,
                    object_id,
                    "GlyphCount",
                    format!(
                        "declares {glyph_count} glyphs but Glyphs contains {actual_glyph_count}"
                    ),
                ));
            }
            consume(
                &mut self.remaining_text_expansion_entries,
                actual_glyph_count,
                "page text expansion",
            )?;
            let glyphs = glyph_text
                .split_whitespace()
                .map(|value| {
                    value.parse::<u32>().map_err(|_| {
                        object_error(
                            self.path,
                            object_id,
                            "Glyphs",
                            format!("invalid glyph ID {value}"),
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let end = code_position.checked_add(code_count).ok_or_else(|| {
                object_error(
                    self.path,
                    object_id,
                    "CodeCount",
                    "range overflow".to_owned(),
                )
            })?;
            if end > character_count {
                return Err(object_error(
                    self.path,
                    object_id,
                    "CodePosition",
                    format!("range {code_position}..{end} exceeds {character_count} characters"),
                ));
            }
            if ranges
                .iter()
                .any(|&(start, prior_end)| code_position < prior_end && start < end)
            {
                return Err(object_error(
                    self.path,
                    object_id,
                    "CodePosition",
                    "CGTransform character ranges overlap".to_owned(),
                ));
            }
            ranges.push((code_position, end));
            maps.push(CharacterGlyphMap {
                code_position,
                code_count,
                glyphs,
            });
        }
        Ok(maps)
    }

    fn convert_image(&mut self, image: raw::ImageObject, object_id: u64) -> Result<ImageObject> {
        let boundary = parse_positive_boundary(&image.boundary, self.path, object_id)?;
        let transform = parse_transform(image.transform.as_deref(), self.path, object_id)?;
        let resource_id = parse_nonzero_id(&image.resource_id, "ResourceID", self.path, object_id)?;
        let resource_format = self
            .document
            .image_resource_format(resource_id)
            .map_err(|error| reference_error(error, self.path, object_id, "ResourceID"))?;
        let substitution_id =
            self.optional_image_id(image.substitution.as_deref(), object_id, "Substitution")?;
        let image_mask_id =
            self.optional_image_id(image.image_mask.as_deref(), object_id, "ImageMask")?;
        let alpha = parse_alpha(image.alpha.as_deref())?;
        let _ = self.resolve_draw_param(image.draw_param.as_deref(), object_id)?;
        let clips = self.convert_clips(image.clips)?;
        Ok(ImageObject {
            object_id,
            boundary,
            transform,
            resource_id,
            resource_format,
            alpha,
            clips,
            substitution_id,
            image_mask_id,
            has_border: image.border.is_some(),
        })
    }

    fn optional_image_id(
        &self,
        value: Option<&str>,
        object_id: u64,
        field: &'static str,
    ) -> Result<Option<u64>> {
        value
            .map(|value| {
                let id = parse_nonzero_id(value, field, self.path, object_id)?;
                self.require_resource_kind(id, ResourceKind::Image, object_id, field)?;
                Ok(id)
            })
            .transpose()
    }

    fn require_resource_kind(
        &self,
        id: u64,
        expected: ResourceKind,
        object_id: u64,
        field: &'static str,
    ) -> Result<()> {
        let actual = self
            .document
            .resource_kind(id)
            .map_err(|error| reference_error(error, self.path, object_id, field))?;
        if actual != expected {
            return Err(object_error(
                self.path,
                object_id,
                field,
                format!("resource {id} is {actual:?}, expected {expected:?}"),
            ));
        }
        Ok(())
    }

    fn convert_clips(&mut self, clips: Option<raw::Clips>) -> Result<Vec<Clip>> {
        let Some(clips) = clips else {
            return Ok(Vec::new());
        };
        if clips.clips.is_empty() {
            return Err(Error::InvalidStructure {
                path: self.path.to_owned(),
                message: "Clips must contain at least one Clip".to_owned(),
            });
        }
        let affected_by_object_transform =
            parse_bool(clips.trans_flag.as_deref(), false, "clip transform flag")?;
        clips
            .clips
            .into_iter()
            .map(|clip| self.convert_clip(clip, affected_by_object_transform))
            .collect()
    }

    fn convert_clip(
        &mut self,
        clip: raw::Clip,
        affected_by_object_transform: bool,
    ) -> Result<Clip> {
        if clip.areas.is_empty() {
            return Err(Error::InvalidStructure {
                path: self.path.to_owned(),
                message: "Clip must contain at least one Area".to_owned(),
            });
        }
        let mut paths = Vec::with_capacity(clip.areas.len());
        for area in clip.areas {
            if area.children.len() != 1 {
                return Err(Error::InvalidStructure {
                    path: self.path.to_owned(),
                    message: "Area must contain exactly one Path or Text".to_owned(),
                });
            }
            let area_transform = area
                .transform
                .as_deref()
                .map(Transform::parse)
                .transpose()?
                .unwrap_or(Transform::IDENTITY);
            let child = area.children.into_iter().next().expect("length checked");
            match child {
                raw::ClipAreaChild::Path(path) => {
                    paths.push(self.convert_clip_path(path, area_transform)?);
                }
                raw::ClipAreaChild::Text(_) => {
                    return Err(Error::UnsupportedFeature(
                        "text clip areas are not supported in phase 2".to_owned(),
                    ));
                }
            }
        }
        let fill_rule = paths[0].fill_rule;
        if paths.iter().any(|path| path.fill_rule != fill_rule) {
            return Err(Error::UnsupportedFeature(
                "mixed fill rules within one Clip are not supported in phase 2".to_owned(),
            ));
        }
        Ok(Clip {
            paths,
            affected_by_object_transform,
        })
    }

    fn convert_clip_path(
        &mut self,
        path: raw::ClipPath,
        area_transform: Transform,
    ) -> Result<ClipPath> {
        let fill_enabled = parse_bool(path.fill.as_deref(), false, "clip path fill")?;
        let stroke_enabled = parse_bool(path.stroke.as_deref(), true, "clip path stroke")?;
        if !fill_enabled || stroke_enabled {
            return Err(Error::UnsupportedFeature(
                "clip paths must be fill-only (Fill=true and Stroke=false) in phase 2".to_owned(),
            ));
        }
        let boundary = Rect::parse(&path.boundary)
            .map_err(|_| invalid_value("clip boundary", &path.boundary))?;
        if boundary.width <= 0.0 || boundary.height <= 0.0 {
            return Err(invalid_value("clip boundary", &path.boundary));
        }
        let transform = path
            .transform
            .as_deref()
            .map(Transform::parse)
            .transpose()?
            .unwrap_or(Transform::IDENTITY);
        let fill_rule = match path.fill_rule.as_deref() {
            None | Some("NonZero") => FillRule::NonZero,
            Some("Even-Odd") => FillRule::EvenOdd,
            Some(value) => return Err(invalid_value("clip fill rule", value)),
        };
        let path_data =
            PathData::parse_with_limit(&path.abbreviated_data, self.remaining_path_commands)?;
        self.remaining_path_commands = self
            .remaining_path_commands
            .checked_sub(path_data.commands().len())
            .ok_or_else(|| Error::LimitExceeded("page path command budget exhausted".to_owned()))?;
        Ok(ClipPath {
            boundary,
            transform,
            area_transform,
            path_data,
            fill_rule,
        })
    }
}

fn parse_bool(value: Option<&str>, default: bool, field: &'static str) -> Result<bool> {
    match value {
        None => Ok(default),
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        Some(value) => Err(invalid_value(field, value)),
    }
}

fn parse_alpha(value: Option<&str>) -> Result<u8> {
    value
        .map(str::parse::<u8>)
        .transpose()
        .map_err(|_| invalid_value("alpha", value.unwrap_or_default()))
        .map(|alpha| alpha.unwrap_or(255))
}

fn effective_paint_color(
    local: Option<&raw::PaintColor>,
    inherited: Option<Color>,
    default: Color,
    object_alpha: u8,
    path: &str,
    object_id: u64,
    field: &'static str,
) -> Result<Color> {
    let mut color = match local {
        Some(color) => Color::parse_rgb(&color.value, color.alpha.as_deref())
            .map_err(|error| object_error(path, object_id, field, error.to_string()))?,
        None => inherited.unwrap_or(default),
    };
    color.alpha = ((u16::from(color.alpha) * u16::from(object_alpha) + 127) / 255) as u8;
    Ok(color)
}

#[allow(clippy::too_many_arguments)]
fn apply_local_stroke_style(
    parameters: &mut PaintParameters,
    line_width: Option<&str>,
    line_join: Option<&str>,
    line_cap: Option<&str>,
    dash_offset: Option<&str>,
    dash_pattern: Option<&str>,
    miter_limit: Option<&str>,
    path: &str,
    object_id: u64,
) -> Result<()> {
    if let Some(value) = line_width {
        parameters.line_width = Some(parse_positive_number(value, "LineWidth", path, object_id)?);
    }
    if let Some(value) = line_join {
        parameters.line_join = Some(match value {
            "Miter" => LineJoin::Miter,
            "Round" => LineJoin::Round,
            "Bevel" => LineJoin::Bevel,
            _ => {
                return Err(object_error(
                    path,
                    object_id,
                    "Join",
                    format!("invalid value {value}"),
                ))
            }
        });
    }
    if let Some(value) = line_cap {
        parameters.line_cap = Some(match value {
            "Butt" => LineCap::Butt,
            "Round" => LineCap::Round,
            "Square" => LineCap::Square,
            _ => {
                return Err(object_error(
                    path,
                    object_id,
                    "Cap",
                    format!("invalid value {value}"),
                ))
            }
        });
    }
    if let Some(value) = dash_offset {
        let parsed = parse_finite_number(value, "DashOffset", path, object_id)?;
        if parsed < 0.0 {
            return Err(object_error(
                path,
                object_id,
                "DashOffset",
                "must be non-negative".to_owned(),
            ));
        }
        parameters.dash_offset = Some(parsed);
    }
    if let Some(value) = dash_pattern {
        let parsed = value
            .split_whitespace()
            .map(|item| parse_positive_number(item, "DashPattern", path, object_id))
            .collect::<Result<Vec<_>>>()?;
        if parsed.is_empty() {
            return Err(object_error(
                path,
                object_id,
                "DashPattern",
                "must not be empty".to_owned(),
            ));
        }
        parameters.dash_pattern = Some(parsed);
    }
    if let Some(value) = miter_limit {
        parameters.miter_limit = Some(parse_positive_number(value, "MiterLimit", path, object_id)?);
    }
    Ok(())
}

fn parse_positive_boundary(value: &str, path: &str, object_id: u64) -> Result<Rect> {
    let boundary = Rect::parse(value)
        .map_err(|error| object_error(path, object_id, "Boundary", error.to_string()))?;
    if boundary.width <= 0.0 || boundary.height <= 0.0 {
        return Err(object_error(
            path,
            object_id,
            "Boundary",
            "width and height must be positive".to_owned(),
        ));
    }
    Ok(boundary)
}

fn parse_transform(value: Option<&str>, path: &str, object_id: u64) -> Result<Transform> {
    value
        .map(Transform::parse)
        .transpose()
        .map_err(|error| object_error(path, object_id, "CTM", error.to_string()))
        .map(|value| value.unwrap_or(Transform::IDENTITY))
}

fn parse_nonzero_id(value: &str, field: &'static str, path: &str, object_id: u64) -> Result<u64> {
    match value.parse::<u64>() {
        Ok(id) if id != 0 => Ok(id),
        _ => Err(object_error(
            path,
            object_id,
            field,
            format!("invalid nonzero ID {value}"),
        )),
    }
}

fn parse_finite_number(
    value: &str,
    field: &'static str,
    path: &str,
    object_id: u64,
) -> Result<f64> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| object_error(path, object_id, field, format!("invalid number {value}")))?;
    if !parsed.is_finite() {
        return Err(object_error(
            path,
            object_id,
            field,
            format!("non-finite number {value}"),
        ));
    }
    Ok(parsed)
}

fn parse_positive_number(
    value: &str,
    field: &'static str,
    path: &str,
    object_id: u64,
) -> Result<f64> {
    let parsed = parse_finite_number(value, field, path, object_id)?;
    if parsed <= 0.0 {
        return Err(object_error(
            path,
            object_id,
            field,
            "must be positive".to_owned(),
        ));
    }
    Ok(parsed)
}

fn parse_optional_finite(
    value: Option<&str>,
    field: &'static str,
    path: &str,
    object_id: u64,
) -> Result<Option<f64>> {
    value
        .map(|value| parse_finite_number(value, field, path, object_id))
        .transpose()
}

fn parse_usize(
    value: &str,
    field: &'static str,
    path: &str,
    object_id: u64,
    allow_zero: bool,
) -> Result<usize> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| object_error(path, object_id, field, format!("invalid integer {value}")))?;
    if !allow_zero && parsed == 0 {
        return Err(object_error(
            path,
            object_id,
            field,
            "must be positive".to_owned(),
        ));
    }
    Ok(parsed)
}

fn parse_delta(
    value: Option<&str>,
    target_len: usize,
    remaining: &mut usize,
    path: &str,
    object_id: u64,
    field: &'static str,
) -> Result<Vec<f64>> {
    if target_len > *remaining {
        return Err(Error::LimitExceeded(format!(
            "page text expansion count exceeds limit while expanding object {object_id} {field} at {path}"
        )));
    }
    let mut values = Vec::with_capacity(target_len);
    let mut tokens = value.unwrap_or_default().split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "g" {
            let count_text = tokens.next().ok_or_else(|| {
                object_error(
                    path,
                    object_id,
                    field,
                    "g repetition is missing its count".to_owned(),
                )
            })?;
            let count = parse_usize(count_text, field, path, object_id, false)?;
            let repetitions = count;
            let repeated_text = tokens.next().ok_or_else(|| {
                object_error(
                    path,
                    object_id,
                    field,
                    "g repetition is missing its value".to_owned(),
                )
            })?;
            let repeated = parse_finite_number(repeated_text, field, path, object_id)?;
            let new_len = values.len().checked_add(repetitions).ok_or_else(|| {
                object_error(
                    path,
                    object_id,
                    field,
                    "repetition count overflow".to_owned(),
                )
            })?;
            if new_len > target_len {
                return Err(object_error(
                    path,
                    object_id,
                    field,
                    format!("contains more than {target_len} displacements"),
                ));
            }
            values.resize(new_len, repeated);
        } else {
            if values.len() == target_len {
                return Err(object_error(
                    path,
                    object_id,
                    field,
                    format!("contains more than {target_len} displacements"),
                ));
            }
            values.push(parse_finite_number(token, field, path, object_id)?);
        }
    }
    values.resize(target_len, 0.0);
    *remaining -= target_len;
    Ok(values)
}

fn consume(remaining: &mut usize, amount: usize, label: &str) -> Result<()> {
    *remaining = remaining
        .checked_sub(amount)
        .ok_or_else(|| Error::LimitExceeded(format!("{label} limit exceeded")))?;
    Ok(())
}

fn object_error(path: &str, object_id: u64, field: &'static str, message: String) -> Error {
    Error::InvalidPageObject {
        path: path.to_owned(),
        object_id,
        field,
        message,
    }
}

fn reference_error(error: Error, path: &str, object_id: u64, field: &'static str) -> Error {
    match error {
        Error::UnknownResource { .. } | Error::ResourceKindMismatch { .. } => {
            object_error(path, object_id, field, error.to_string())
        }
        error => error,
    }
}

fn parse_object_id(value: &str) -> Result<u64> {
    match value.parse::<u64>() {
        Ok(id) if id != 0 => Ok(id),
        _ => Err(invalid_value("object ID", value)),
    }
}

fn invalid_value(field: &'static str, value: &str) -> Error {
    Error::InvalidValue {
        field,
        value: value.to_owned(),
        path: None,
    }
}
