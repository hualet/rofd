use rofd_core::{
    ClipPath as CoreClipPath, Color, FillRule, Page, PageObject, PathData, PathObject, Transform,
    UnsupportedObjectKind,
};

use crate::{Error, Result};

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
    /// Intersects subsequent drawing with the union of one or more paths.
    ///
    /// A backend must append all `paths` and apply a single clip operation so
    /// their filled regions are unioned. Separate commands intersect.
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
    /// Draws a validated path.
    DrawPath(PathData),
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    commands: Vec<Command>,
    diagnostics: Vec<RenderDiagnostic>,
}

impl DisplayList {
    /// Lowers a validated page into source-ordered display commands.
    ///
    /// Layers retain their current source order. Page groups are recursively
    /// flattened in source order; their depth is bounded by `rofd-core`'s page
    /// object validation limit.
    pub fn from_page(page: &Page) -> Result<Self> {
        let mut display_list = Self::default();
        for layer in page.layers() {
            for object in layer.objects() {
                display_list.lower_object(object)?;
            }
        }
        Ok(display_list)
    }

    /// Returns display commands in execution order.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Returns non-fatal lowering diagnostics in source order.
    pub fn diagnostics(&self) -> &[RenderDiagnostic] {
        &self.diagnostics
    }

    fn lower_object(&mut self, object: &PageObject) -> Result<()> {
        match object {
            PageObject::Path(path) => self.lower_path(path),
            PageObject::Group(group) => {
                for child in group.objects() {
                    self.lower_object(child)?;
                }
                Ok(())
            }
            PageObject::Unsupported(object) => {
                self.diagnostics.push(RenderDiagnostic {
                    object_id: object.object_id(),
                    kind: object.kind(),
                    message: unsupported_message(object.kind()).to_owned(),
                });
                Ok(())
            }
        }
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

        let boundary = path.boundary();
        let translation =
            Transform::new(1.0, 0.0, 0.0, 1.0, boundary.x, boundary.y).map_err(|error| {
                Error::InvalidModel {
                    object_id: path.object_id(),
                    field: "boundary translation",
                    value: error.to_string(),
                }
            })?;
        let object_to_page =
            path.transform()
                .then(translation)
                .map_err(|error| Error::InvalidModel {
                    object_id: path.object_id(),
                    field: "object-to-page transform",
                    value: error.to_string(),
                })?;

        self.commands.push(Command::Save);
        for clip in path.clips() {
            let Some(first) = clip.paths().first() else {
                return Err(Error::InvalidModel {
                    object_id: path.object_id(),
                    field: "clip paths",
                    value: "empty clipping operand".to_owned(),
                });
            };
            let paths = clip
                .paths()
                .iter()
                .map(|clip_path| {
                    self.lower_clip_path(
                        path,
                        clip_path,
                        clip.affected_by_object_transform(),
                        translation,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            self.commands.push(Command::ClipPath {
                paths,
                rule: first.fill_rule(),
            });
        }
        self.commands.push(Command::ConcatTransform(object_to_page));
        self.commands.push(Command::SetStroke(path.stroke()));
        self.commands.push(Command::SetFill(path.fill()));
        self.commands.push(Command::SetFillRule(path.fill_rule()));
        self.commands.push(Command::SetLineWidth(line_width));
        self.commands
            .push(Command::DrawPath(path.path_data().clone()));
        self.commands.push(Command::Restore);
        Ok(())
    }

    fn lower_clip_path(
        &self,
        object: &PathObject,
        clip_path: &CoreClipPath,
        affected_by_object_transform: bool,
        object_boundary_translation: Transform,
    ) -> Result<ClipPath> {
        let boundary = clip_path.boundary();
        let path_boundary_translation = Transform::new(1.0, 0.0, 0.0, 1.0, boundary.x, boundary.y)
            .map_err(|error| clip_transform_error(object.object_id(), error))?;
        let mut transform = clip_path
            .transform()
            .then(path_boundary_translation)
            .and_then(|transform| transform.then(clip_path.area_transform()))
            .map_err(|error| clip_transform_error(object.object_id(), error))?;
        if affected_by_object_transform {
            transform = transform
                .then(object.transform())
                .map_err(|error| clip_transform_error(object.object_id(), error))?;
        }
        transform = transform
            .then(object_boundary_translation)
            .map_err(|error| clip_transform_error(object.object_id(), error))?;
        Ok(ClipPath::new(transform, clip_path.path_data().clone()))
    }
}

fn clip_transform_error(object_id: u64, error: rofd_core::Error) -> Error {
    Error::InvalidModel {
        object_id,
        field: "clip transform",
        value: error.to_string(),
    }
}

/// A non-fatal notice about a page object omitted from drawing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderDiagnostic {
    object_id: u64,
    kind: UnsupportedObjectKind,
    message: String,
}

impl RenderDiagnostic {
    /// Returns the OFD object identifier that was omitted.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the unsupported object category.
    pub fn kind(&self) -> UnsupportedObjectKind {
        self.kind
    }

    /// Returns a human-readable explanation.
    pub fn message(&self) -> &str {
        &self.message
    }
}

fn unsupported_message(kind: UnsupportedObjectKind) -> &'static str {
    match kind {
        UnsupportedObjectKind::Text => "text objects are not supported",
        UnsupportedObjectKind::Image => "image objects are not supported",
        UnsupportedObjectKind::Composite => "composite objects are not supported",
    }
}
