use std::collections::HashSet;
use std::sync::Arc;

use rofd_core::{
    Clip as CoreClip, ClipPath as CoreClipPath, Color, FillRule, ImageObject, LayerSource, LineCap,
    LineJoin, Page, PageObject, PathData, PathObject, ResourceKind, StrokeStyle, TextObject,
    Transform, UnsupportedObjectKind,
};

use crate::{
    position_glyph_runs, DecodedImage, Error, FontDiagnostic, FontResolver, GlyphRun, ImageDecoder,
    Result, SystemFontResolver,
};

/// A backend-neutral display-list command.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Command {
    /// Saves the current graphics state.
    Save,
    /// Concatenates an affine transform with the current transform.
    ///
    /// The command transform maps new local coordinates into the prior user
    /// space. If the prior active transform is `T` and this command contains
    /// `M`, the new active transform is `M.then(T)`: `M` is applied first,
    /// followed by `T`.
    ConcatTransform(Transform),
    /// Intersects subsequent drawing with the geometric union of area paths.
    ///
    /// Each entry is a distinct OFD `Area`. A backend must compute the true
    /// coverage union of their individually filled regions, then intersect it
    /// with the prior clip. Appending every entry to one Cairo compound path
    /// and clipping once is not equivalent: overlapping even-odd paths or
    /// opposite-winding non-zero paths can cancel. Separate commands continue
    /// to represent intersection operands.
    ///
    /// Task 6 raster backends must include overlap and opposite-winding
    /// regression images before interpreting this command.
    ClipPath {
        /// Paths whose filled regions form one union operand.
        paths: Vec<ClipPath>,
        /// The fill rule used to determine the clipping region.
        rule: FillRule,
    },
    /// Sets or disables the stroke paint.
    SetStroke(Option<Color>),
    /// Sets or disables the fill paint.
    SetFill(Option<Color>),
    /// Sets the rule used to fill paths.
    SetFillRule(FillRule),
    /// Sets the stroke width in millimetres.
    SetLineWidth(f64),
    /// Sets the shape used at stroked segment joins.
    SetLineJoin(LineJoin),
    /// Sets the shape used at open stroke endpoints.
    SetLineCap(LineCap),
    /// Sets the stroke miter limit.
    SetMiterLimit(f64),
    /// Sets the stroke dash phase and pattern in millimetres.
    SetDash {
        /// Non-negative dash phase.
        offset: f64,
        /// Positive alternating dash and gap lengths, or empty for a solid stroke.
        pattern: Vec<f64>,
    },
    /// Sets object opacity for raster-image compositing.
    SetAlpha(u8),
    /// Draws a validated path.
    DrawPath(PathData),
    /// Draws one exact, unshaped, positioned text run.
    DrawGlyphRun(GlyphRun),
    /// Draws decoded pixels into the object's local boundary extent.
    DrawImage {
        /// Immutable validated RGBA8 pixels.
        image: DecodedImage,
        /// Target width in normalized image-object coordinates.
        width: f64,
        /// Target height in normalized image-object coordinates.
        height: f64,
    },
    /// Restores the most recently saved graphics state.
    Restore,
}

/// One path in a display-list clipping union, already mapped to page space.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipPath {
    transform: Transform,
    path: PathData,
}

impl ClipPath {
    /// Creates a clip path with its complete local-to-page transform.
    pub fn new(transform: Transform, path: PathData) -> Self {
        Self { transform, path }
    }

    /// Returns the complete clip-local-to-page transform.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Returns the validated clipping path data.
    pub fn path(&self) -> &PathData {
        &self.path
    }
}

/// An immutable sequence of drawing commands and non-fatal diagnostics.
#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    commands: Arc<Vec<Command>>,
    diagnostics: Arc<Vec<RenderDiagnostic>>,
}

impl PartialEq for DisplayList {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.commands, &other.commands)
            || self.commands.as_ref() == other.commands.as_ref())
            && (Arc::ptr_eq(&self.diagnostics, &other.diagnostics)
                || self.diagnostics.as_ref() == other.diagnostics.as_ref())
    }
}

impl DisplayList {
    /// Lowers a validated page into effective-paint-order display commands.
    ///
    /// The page has already merged template and direct layers. Source order is
    /// retained within each effective layer category. Page groups are
    /// recursively flattened in source order; their depth is bounded by
    /// `rofd-core`'s page object validation limit.
    pub fn from_page(page: &Page) -> Result<Self> {
        let resolver = SystemFontResolver::empty(Vec::new(), page.resource_limits().max_font_bytes);
        let decoder = ImageDecoder::default();
        DisplayListBuilder::new(&resolver, &decoder).build(page)
    }

    /// Returns display commands in execution order.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Returns non-fatal lowering diagnostics in effective paint order.
    pub fn diagnostics(&self) -> &[RenderDiagnostic] {
        &self.diagnostics
    }

    fn push_command(&mut self, command: Command) {
        Arc::get_mut(&mut self.commands)
            .expect("display-list commands are private while building")
            .push(command);
    }
}

/// Controlled display-list construction with injected font and image services.
///
/// Construction is transactional: a list is returned only after every referenced
/// font and image needed by the page has been loaded and validated. The builder
/// borrows reusable services so their bounded caches can be shared across pages.
pub struct DisplayListBuilder<'a> {
    font_resolver: &'a dyn FontResolver,
    image_decoder: &'a ImageDecoder,
    max_decoded_image_bytes: Option<u64>,
}

struct DecodedImageBudget {
    allocations: HashSet<usize>,
    used_bytes: u64,
    max_bytes: u64,
}

impl DecodedImageBudget {
    fn account(&mut self, image: &DecodedImage) -> Result<()> {
        if !self.allocations.insert(image.allocation_id()) {
            return Ok(());
        }
        let required_bytes = self.used_bytes.checked_add(image.byte_len()).ok_or(
            Error::DisplayListImageBudgetExceeded {
                required_bytes: u64::MAX,
                max_bytes: self.max_bytes,
            },
        )?;
        if required_bytes > self.max_bytes {
            return Err(Error::DisplayListImageBudgetExceeded {
                required_bytes,
                max_bytes: self.max_bytes,
            });
        }
        self.used_bytes = required_bytes;
        Ok(())
    }
}

impl<'a> DisplayListBuilder<'a> {
    /// Creates a builder borrowing deterministic, caller-configured services.
    pub fn new(font_resolver: &'a dyn FontResolver, image_decoder: &'a ImageDecoder) -> Self {
        Self {
            font_resolver,
            image_decoder,
            max_decoded_image_bytes: None,
        }
    }

    /// Sets the aggregate decoded-image bytes one display list may retain.
    ///
    /// Without this override, the document's per-image decoded-byte limit is
    /// also used as the display list's aggregate unique-allocation limit.
    pub fn with_max_decoded_image_bytes(mut self, max_bytes: u64) -> Result<Self> {
        if max_bytes == 0 {
            return Err(Error::InvalidOption {
                field: "display_list_max_decoded_image_bytes",
                value: "0".to_owned(),
            });
        }
        self.max_decoded_image_bytes = Some(max_bytes);
        Ok(self)
    }

    /// Resolves and lowers one validated page without publishing partial output on failure.
    pub fn build(&self, page: &Page) -> Result<DisplayList> {
        let mut display_list = DisplayList::default();
        let mut image_budget = DecodedImageBudget {
            allocations: HashSet::new(),
            used_bytes: 0,
            max_bytes: self
                .max_decoded_image_bytes
                .unwrap_or(page.resource_limits().max_decoded_image_bytes),
        };
        for layer in page.layers() {
            for object in layer.objects() {
                self.lower_object(
                    &mut display_list,
                    page,
                    object,
                    layer.source(),
                    &mut image_budget,
                )?;
            }
        }
        Ok(display_list)
    }

    fn lower_object(
        &self,
        display_list: &mut DisplayList,
        page: &Page,
        object: &PageObject,
        source: LayerSource,
        image_budget: &mut DecodedImageBudget,
    ) -> Result<()> {
        match object {
            PageObject::Path(path) => {
                if path.transform().is_singular() {
                    display_list.push_diagnostic(
                        path.object_id(),
                        source,
                        RenderDiagnosticKind::SingularTransform,
                    );
                    return Ok(());
                }
                display_list.lower_path(path)
            }
            PageObject::Text(text) => {
                if text.transform().is_singular() {
                    display_list.push_diagnostic(
                        text.object_id(),
                        source,
                        RenderDiagnosticKind::SingularTransform,
                    );
                    return Ok(());
                }
                let resource =
                    page.font_resource(text.font_id())
                        .map_err(|source| Error::ObjectResource {
                            object_id: text.object_id(),
                            resource_id: text.font_id(),
                            kind: ResourceKind::Font,
                            source,
                        })?;
                let runs = position_glyph_runs(
                    self.font_resolver,
                    &resource,
                    text,
                    page.resource_limits(),
                )
                .map_err(|source| Error::ObjectResourceProcessing {
                    object_id: text.object_id(),
                    resource_id: text.font_id(),
                    kind: ResourceKind::Font,
                    asset_path: resource
                        .asset_path()
                        .unwrap_or("<external-font>")
                        .to_owned(),
                    source: Box::new(source),
                })?;
                display_list.lower_text(text, runs, source)
            }
            PageObject::Image(image) => {
                if image.transform().is_singular() {
                    display_list.push_diagnostic(
                        image.object_id(),
                        source,
                        RenderDiagnosticKind::SingularTransform,
                    );
                    return Ok(());
                }
                let resource = match page.image_resource(image.resource_id()) {
                    Ok(resource) => resource,
                    // ofdrw logs the lookup failure and draws the rest of the
                    // page (its containsJPEG.ofd reference renders the images
                    // as nothing because the package stores them under
                    // `DOC_0/` while the XML references `Doc_0/`).
                    Err(rofd_core::Error::MissingEntry(_)) => {
                        display_list.push_diagnostic(
                            image.object_id(),
                            source,
                            RenderDiagnosticKind::ImageResourceMissing {
                                resource_id: image.resource_id(),
                            },
                        );
                        return Ok(());
                    }
                    Err(source) => {
                        return Err(Error::ObjectResource {
                            object_id: image.object_id(),
                            resource_id: image.resource_id(),
                            kind: ResourceKind::Image,
                            source,
                        });
                    }
                };
                let decoded = self
                    .image_decoder
                    .decode(&resource, page.resource_limits())
                    .map_err(|source| Error::ObjectResourceProcessing {
                        object_id: image.object_id(),
                        resource_id: image.resource_id(),
                        kind: ResourceKind::Image,
                        asset_path: resource.asset_path().to_owned(),
                        source: Box::new(source),
                    })?;
                image_budget.account(&decoded)?;
                display_list.lower_image(image, decoded, source)
            }
            PageObject::Group(group) => {
                for child in group.objects() {
                    self.lower_object(display_list, page, child, source, image_budget)?;
                }
                Ok(())
            }
            PageObject::Unsupported(object) => {
                display_list.push_diagnostic(
                    object.object_id(),
                    source,
                    RenderDiagnosticKind::UnsupportedObject {
                        kind: object.kind(),
                    },
                );
                Ok(())
            }
        }
    }
}

impl DisplayList {
    fn push_diagnostic(&mut self, object_id: u64, source: LayerSource, kind: RenderDiagnosticKind) {
        Arc::get_mut(&mut self.diagnostics)
            .expect("display-list diagnostics are private while building")
            .push(RenderDiagnostic {
                object_id,
                source,
                message: diagnostic_message(&kind),
                kind,
            });
    }

    fn lower_path(&mut self, path: &PathObject) -> Result<()> {
        let line_width = path.line_width();
        if !line_width.is_finite() || line_width <= 0.0 {
            return Err(Error::InvalidModel {
                object_id: path.object_id(),
                field: "line width",
                value: line_width.to_string(),
            });
        }

        let (translation, object_to_page) =
            object_transforms(path.object_id(), path.boundary(), path.transform())?;

        self.push_command(Command::Save);
        self.lower_clips(
            path.object_id(),
            path.transform(),
            path.clips(),
            translation,
        )?;
        self.push_command(Command::ConcatTransform(object_to_page));
        self.push_command(Command::SetStroke(path.stroke()));
        self.push_command(Command::SetFill(path.fill()));
        self.push_command(Command::SetFillRule(path.fill_rule()));
        self.push_stroke_style(path.stroke_style());
        self.push_command(Command::DrawPath(path.path_data().clone()));
        self.push_command(Command::Restore);
        Ok(())
    }

    fn lower_text(
        &mut self,
        text: &TextObject,
        runs: Vec<GlyphRun>,
        source: LayerSource,
    ) -> Result<()> {
        let (translation, object_to_page) =
            object_transforms(text.object_id(), text.boundary(), text.transform())?;
        self.push_command(Command::Save);
        self.lower_clips(
            text.object_id(),
            text.transform(),
            text.clips(),
            translation,
        )?;
        self.push_command(Command::ConcatTransform(object_to_page));
        self.push_command(Command::SetStroke(text.stroke()));
        self.push_command(Command::SetFill(text.fill()));
        self.push_stroke_style(text.stroke_style());
        for run in runs {
            for diagnostic in run.diagnostics() {
                self.push_font_diagnostic(text.object_id(), source, diagnostic);
            }
            self.push_command(Command::DrawGlyphRun(run));
        }
        self.push_command(Command::Restore);
        Ok(())
    }

    fn lower_image(
        &mut self,
        image: &ImageObject,
        decoded: DecodedImage,
        source: LayerSource,
    ) -> Result<()> {
        let boundary = image.boundary();
        let (translation, object_to_page) =
            object_transforms(image.object_id(), boundary, image.transform())?;
        if let Some(resource_id) = image.substitution_id() {
            self.push_diagnostic(
                image.object_id(),
                source,
                RenderDiagnosticKind::ImageSubstitutionUnsupported { resource_id },
            );
        }
        if let Some(resource_id) = image.image_mask_id() {
            self.push_diagnostic(
                image.object_id(),
                source,
                RenderDiagnosticKind::ImageMaskUnsupported { resource_id },
            );
        }
        if image.has_border() {
            self.push_diagnostic(
                image.object_id(),
                source,
                RenderDiagnosticKind::ImageBorderUnsupported,
            );
        }
        self.push_command(Command::Save);
        self.lower_clips(
            image.object_id(),
            image.transform(),
            image.clips(),
            translation,
        )?;
        self.push_command(Command::ConcatTransform(object_to_page));
        self.push_command(Command::SetAlpha(image.alpha()));
        self.push_command(Command::DrawImage {
            image: decoded,
            width: 1.0,
            height: 1.0,
        });
        self.push_command(Command::Restore);
        Ok(())
    }

    fn push_stroke_style(&mut self, style: &StrokeStyle) {
        self.push_command(Command::SetLineWidth(style.line_width()));
        self.push_command(Command::SetLineJoin(style.line_join()));
        self.push_command(Command::SetLineCap(style.line_cap()));
        self.push_command(Command::SetMiterLimit(style.miter_limit()));
        self.push_command(Command::SetDash {
            offset: style.dash_offset(),
            pattern: style.dash_pattern().to_vec(),
        });
    }

    fn lower_clips(
        &mut self,
        object_id: u64,
        object_transform: Transform,
        clips: &[CoreClip],
        object_boundary_translation: Transform,
    ) -> Result<()> {
        for clip in clips {
            let Some(first) = clip.paths().first() else {
                return Err(Error::InvalidModel {
                    object_id,
                    field: "clip paths",
                    value: "empty clipping operand".to_owned(),
                });
            };
            let paths = clip
                .paths()
                .iter()
                .map(|clip_path| {
                    lower_clip_path(
                        object_id,
                        object_transform,
                        clip_path,
                        clip.affected_by_object_transform(),
                        object_boundary_translation,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            self.push_command(Command::ClipPath {
                paths,
                rule: first.fill_rule(),
            });
        }
        Ok(())
    }

    fn push_font_diagnostic(
        &mut self,
        object_id: u64,
        source: LayerSource,
        diagnostic: &FontDiagnostic,
    ) {
        let kind = match diagnostic {
            FontDiagnostic::FamilyFallback {
                character,
                scalar_index,
                requested,
                selected,
            } => RenderDiagnosticKind::FontFallback {
                character: *character,
                scalar_index: *scalar_index,
                requested: requested.clone(),
                selected: selected.clone(),
            },
            FontDiagnostic::MissingGlyph {
                character,
                scalar_index,
                used_visible_replacement,
                ..
            } => RenderDiagnosticKind::MissingGlyph {
                character: *character,
                scalar_index: *scalar_index,
                used_visible_replacement: *used_visible_replacement,
            },
        };
        self.push_diagnostic(object_id, source, kind);
    }
}

fn object_transforms(
    object_id: u64,
    boundary: rofd_core::Rect,
    object_transform: Transform,
) -> Result<(Transform, Transform)> {
    let translation =
        Transform::new(1.0, 0.0, 0.0, 1.0, boundary.x, boundary.y).map_err(|error| {
            Error::InvalidModel {
                object_id,
                field: "boundary translation",
                value: error.to_string(),
            }
        })?;
    let object_to_page =
        object_transform
            .then(translation)
            .map_err(|error| Error::InvalidModel {
                object_id,
                field: "object-to-page transform",
                value: error.to_string(),
            })?;
    Ok((translation, object_to_page))
}

fn lower_clip_path(
    object_id: u64,
    object_transform: Transform,
    clip_path: &CoreClipPath,
    affected_by_object_transform: bool,
    object_boundary_translation: Transform,
) -> Result<ClipPath> {
    let boundary = clip_path.boundary();
    let path_boundary_translation = Transform::new(1.0, 0.0, 0.0, 1.0, boundary.x, boundary.y)
        .map_err(|error| clip_transform_error(object_id, error))?;
    let mut transform = clip_path
        .transform()
        .then(path_boundary_translation)
        .and_then(|transform| transform.then(clip_path.area_transform()))
        .map_err(|error| clip_transform_error(object_id, error))?;
    if affected_by_object_transform {
        transform = transform
            .then(object_transform)
            .map_err(|error| clip_transform_error(object_id, error))?;
    }
    transform = transform
        .then(object_boundary_translation)
        .map_err(|error| clip_transform_error(object_id, error))?;
    Ok(ClipPath::new(transform, clip_path.path_data().clone()))
}

fn clip_transform_error(object_id: u64, error: rofd_core::Error) -> Error {
    Error::InvalidModel {
        object_id,
        field: "clip transform",
        value: error.to_string(),
    }
}

/// A stable structured non-fatal lowering diagnostic category.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RenderDiagnosticKind {
    /// A page object category remains unsupported by the display-list model.
    UnsupportedObject {
        /// Unsupported object category.
        kind: UnsupportedObjectKind,
    },
    /// The declared font lacked a scalar and a configured family fallback was selected.
    FontFallback {
        /// Original Unicode scalar.
        character: char,
        /// Zero-based scalar index in the text object.
        scalar_index: usize,
        /// Declared family or font name.
        requested: String,
        /// Stable selected fallback identity.
        selected: String,
    },
    /// No selected font contained the original scalar.
    MissingGlyph {
        /// Original Unicode scalar.
        character: char,
        /// Zero-based scalar index in the text object.
        scalar_index: usize,
        /// Whether a visible font replacement glyph was used.
        used_visible_replacement: bool,
    },
    /// An image substitution reference is retained but not composited yet.
    ImageSubstitutionUnsupported {
        /// Referenced substitution image resource.
        resource_id: u64,
    },
    /// An image-mask reference is retained but not composited yet.
    ImageMaskUnsupported {
        /// Referenced mask image resource.
        resource_id: u64,
    },
    /// An explicitly declared image border is retained but not drawn yet.
    ImageBorderUnsupported,
    /// The object transform is singular; the object is invisible and skipped.
    SingularTransform,
    /// The referenced image resource is missing from the package; the object
    /// is skipped so the rest of the page still renders.
    ImageResourceMissing {
        /// Referenced image resource that could not be resolved.
        resource_id: u64,
    },
}

/// A non-fatal source-aware notice produced while lowering a page object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderDiagnostic {
    object_id: u64,
    kind: RenderDiagnosticKind,
    source: LayerSource,
    message: String,
}

impl RenderDiagnostic {
    /// Returns the OFD object identifier associated with this notice.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the structured diagnostic category.
    pub fn kind(&self) -> &RenderDiagnosticKind {
        &self.kind
    }

    /// Returns the legacy unsupported object category, when this is such a notice.
    pub fn unsupported_kind(&self) -> Option<UnsupportedObjectKind> {
        match self.kind {
            RenderDiagnosticKind::UnsupportedObject { kind } => Some(kind),
            _ => None,
        }
    }

    /// Returns the page or template layer source of the associated object.
    pub fn source(&self) -> LayerSource {
        self.source
    }

    /// Returns a human-readable explanation.
    pub fn message(&self) -> &str {
        &self.message
    }
}

fn diagnostic_message(kind: &RenderDiagnosticKind) -> String {
    match kind {
        RenderDiagnosticKind::UnsupportedObject { kind } => match kind {
            UnsupportedObjectKind::Text => "text objects are not supported".to_owned(),
            UnsupportedObjectKind::Image => "image objects are not supported".to_owned(),
            UnsupportedObjectKind::Composite => "composite objects are not supported".to_owned(),
        },
        RenderDiagnosticKind::FontFallback {
            character,
            selected,
            ..
        } => format!("font fallback {selected} selected for {character:?}"),
        RenderDiagnosticKind::MissingGlyph {
            character,
            used_visible_replacement,
            ..
        } => format!(
            "missing glyph for {character:?}; visible replacement: {used_visible_replacement}"
        ),
        RenderDiagnosticKind::ImageSubstitutionUnsupported { resource_id } => {
            format!("image substitution resource {resource_id} is not composited")
        }
        RenderDiagnosticKind::ImageMaskUnsupported { resource_id } => {
            format!("image mask resource {resource_id} is not composited")
        }
        RenderDiagnosticKind::ImageBorderUnsupported => "image border is not drawn".to_owned(),
        RenderDiagnosticKind::SingularTransform => {
            "object transform is singular; object skipped".to_owned()
        }
        RenderDiagnosticKind::ImageResourceMissing { resource_id } => {
            format!("image resource {resource_id} is missing from the package; object skipped")
        }
    }
}
