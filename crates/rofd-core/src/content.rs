use std::collections::HashSet;

use crate::raw;
use crate::{Color, Error, PathData, Rect, ResourceLimits, Result, Transform};

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

/// An ordered layer of page objects.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    object_id: u64,
    kind: LayerType,
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
    /// A text object.
    Text,
    /// An image object.
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
    limits: &ResourceLimits,
    path: &str,
) -> Result<Vec<Layer>> {
    let Some(content) = content else {
        return Ok(Vec::new());
    };
    let mut context = ConversionContext {
        limits,
        path,
        object_ids: HashSet::new(),
        remaining_path_commands: limits.max_path_commands,
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
            objects,
        });
    }
    Ok(layers)
}

struct ConversionContext<'a> {
    limits: &'a ResourceLimits,
    path: &'a str,
    object_ids: HashSet<u64>,
    remaining_path_commands: usize,
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
                    self.unsupported(&object.id, UnsupportedObjectKind::Text)?
                }
                raw::GraphicUnit::Image(object) => {
                    self.unsupported(&object.id, UnsupportedObjectKind::Image)?
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
        let stroke = stroke_enabled
            .then(|| paint_color(path.stroke_color.as_ref(), Color::BLACK, object_alpha))
            .transpose()?;
        let fill = fill_enabled
            .then(|| {
                paint_color(
                    path.fill_color.as_ref(),
                    Color {
                        alpha: 0,
                        ..Color::BLACK
                    },
                    object_alpha,
                )
            })
            .transpose()?;
        let line_width = match path.line_width.as_deref() {
            Some(value) => {
                let parsed = value
                    .parse::<f64>()
                    .map_err(|_| invalid_value("line width", value))?;
                if !parsed.is_finite() || parsed <= 0.0 {
                    return Err(invalid_value("line width", value));
                }
                parsed
            }
            None => 0.353,
        };
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
            fill_rule,
            clips,
        })
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

fn paint_color(color: Option<&raw::PaintColor>, default: Color, object_alpha: u8) -> Result<Color> {
    let mut color = match color {
        Some(color) => Color::parse_rgb(&color.value, color.alpha.as_deref())?,
        None => default,
    };
    color.alpha = ((u16::from(color.alpha) * u16::from(object_alpha) + 127) / 255) as u8;
    Ok(color)
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
    }
}
