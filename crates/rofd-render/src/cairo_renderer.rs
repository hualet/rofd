use std::f64::consts::PI;

use cairo::{
    Antialias, Context, FillRule as CairoFillRule, Format, ImageSurface, LineCap, LineJoin, Matrix,
    Operator, Path,
};
use rofd_core::{Color, FillRule, Page, PageObject, PathCommand, PathData, Point, Rect, Transform};

use crate::{ClipPath, Command, DisplayCommandKind, DisplayList, Error, RenderDiagnostic, Result};

const MILLIMETRES_PER_INCH: f64 = 25.4;
const DEFAULT_MITER_LIMIT: f64 = 3.528;
const DEFAULT_CURVE_TOLERANCE: f64 = 0.1;
const MAX_CAIRO_IMAGE_DIMENSION: i32 = 32_767;
const DEFAULT_MAX_RASTER_BYTES: u64 = 256 * 1024 * 1024;

/// Options controlling page rasterization.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderOptions {
    /// Output resolution in pixels per inch.
    pub dpi: f64,
    /// Additional positive output scale factor.
    pub scale: f64,
    /// Clockwise page rotation in degrees: 0, 90, 180, or 270.
    pub rotation_degrees: u16,
    /// Color painted behind the page content.
    pub background: Color,
    /// Optional clipping rectangle in absolute page-space millimetres.
    pub clip: Option<Rect>,
    /// Maximum bytes for all simultaneously live full-page raster surfaces.
    ///
    /// Validation conservatively includes the caller's ARGB32 target, one A8
    /// clip mask, and one ARGB32 clipped-drawing intermediate even when the
    /// current page does not use clipping.
    pub max_raster_bytes: u64,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            dpi: 96.0,
            scale: 1.0,
            rotation_degrees: 0,
            background: Color {
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
            },
            clip: None,
            max_raster_bytes: DEFAULT_MAX_RASTER_BYTES,
        }
    }
}

/// Information produced while rendering one page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderReport {
    diagnostics: Vec<RenderDiagnostic>,
}

impl RenderReport {
    /// Returns non-fatal display-list diagnostics in effective paint order.
    pub fn diagnostics(&self) -> &[RenderDiagnostic] {
        &self.diagnostics
    }
}

/// Rasterizes validated pages by interpreting their display lists with Cairo.
#[derive(Clone, Copy, Debug, Default)]
pub struct CairoRenderer;

impl CairoRenderer {
    /// Returns the required output dimensions in pixels.
    pub fn pixel_size(page: &Page, options: &RenderOptions) -> Result<(i32, i32)> {
        let geometry = RenderGeometry::new(page.size(), options)?;
        Ok((geometry.pixel_width, geometry.pixel_height))
    }

    /// Renders a page into a caller-owned Cairo context.
    ///
    /// The caller's graphics state and current path are restored on both
    /// successful rendering and recoverable errors. If Cairo enters an error
    /// state during rendering, restoration is still attempted and concurrent
    /// cleanup failures are retained in [`Error::Cleanup`].
    pub fn render_page(
        &self,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
    ) -> Result<RenderReport> {
        let caller_path = cairo(context.copy_path(), "capture caller path")?;
        let rendered = self.render_page_preserving_path(page, context, options);
        let restored_path = restore_path(context, &caller_path);
        combine_results(rendered, restored_path, "restore caller path")
    }

    fn render_page_preserving_path(
        &self,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
    ) -> Result<RenderReport> {
        let geometry = RenderGeometry::new(page.size(), options)?;
        ensure_page_supported(page)?;
        let display_list = DisplayList::from_page(page)?;
        ensure_supported_commands(display_list.commands())?;
        if let Ok(surface) = ImageSurface::try_from(context.target()) {
            if surface.width() < geometry.pixel_width || surface.height() < geometry.pixel_height {
                return Err(Error::SurfaceTooSmall {
                    required_width: geometry.pixel_width,
                    required_height: geometry.pixel_height,
                    actual_width: surface.width(),
                    actual_height: surface.height(),
                });
            }
        }
        cairo(context.save(), "save caller state")?;

        let rendered = render_saved(context, page.size(), options, &geometry, &display_list);
        let restored = cairo(context.restore(), "restore caller state");
        combine_results(rendered, restored, "restore caller state").map(|()| RenderReport {
            diagnostics: display_list.diagnostics().to_vec(),
        })
    }
}

fn ensure_page_supported(page: &Page) -> Result<()> {
    fn deferred(objects: &[PageObject]) -> Option<DisplayCommandKind> {
        for object in objects {
            match object {
                PageObject::Text(_) => return Some(DisplayCommandKind::GlyphRun),
                PageObject::Image(_) => return Some(DisplayCommandKind::Image),
                PageObject::Group(group) => {
                    if let Some(command) = deferred(group.objects()) {
                        return Some(command);
                    }
                }
                PageObject::Path(_) | PageObject::Unsupported(_) => {}
            }
        }
        None
    }

    for layer in page.layers() {
        if let Some(command) = deferred(layer.objects()) {
            return Err(Error::UnsupportedDisplayCommand { command });
        }
    }
    Ok(())
}

fn render_saved(
    context: &Context,
    page_box: Rect,
    options: &RenderOptions,
    geometry: &RenderGeometry,
    display_list: &DisplayList,
) -> Result<()> {
    context.set_matrix(geometry.page_to_device);
    context.set_operator(Operator::Over);
    set_raster_defaults(context)?;
    set_stroke_defaults(context)?;
    context.rectangle(page_box.x, page_box.y, page_box.width, page_box.height);
    context.clip();
    cairo(context.status(), "establish page graphics state")?;
    set_source_color(context, options.background)?;
    cairo(context.paint(), "paint page background")?;

    if let Some(clip) = options.clip {
        context.rectangle(clip.x, clip.y, clip.width, clip.height);
        context.clip();
        cairo(context.status(), "apply page-space clip")?;
    }

    let mut interpreter = Interpreter::new(context, geometry);
    interpreter.run(display_list.commands())
}

fn ensure_supported_commands(commands: &[Command]) -> Result<()> {
    for command in commands {
        let command = match command {
            Command::DrawGlyphRun(_) => Some(DisplayCommandKind::GlyphRun),
            Command::DrawImage { .. } => Some(DisplayCommandKind::Image),
            _ => None,
        };
        if let Some(command) = command {
            return Err(Error::UnsupportedDisplayCommand { command });
        }
    }
    Ok(())
}

struct RenderGeometry {
    pixel_width: i32,
    pixel_height: i32,
    page_to_device: Matrix,
}

impl RenderGeometry {
    fn new(page_box: Rect, options: &RenderOptions) -> Result<Self> {
        if !options.dpi.is_finite() || options.dpi <= 0.0 {
            return Err(invalid_option("dpi", options.dpi));
        }
        if !options.scale.is_finite() || options.scale <= 0.0 {
            return Err(invalid_option("scale", options.scale));
        }
        if !matches!(options.rotation_degrees, 0 | 90 | 180 | 270) {
            return Err(Error::InvalidOption {
                field: "rotation_degrees",
                value: options.rotation_degrees.to_string(),
            });
        }
        if let Some(clip) = options.clip {
            if !clip.x.is_finite()
                || !clip.y.is_finite()
                || !clip.width.is_finite()
                || !clip.height.is_finite()
                || clip.width <= 0.0
                || clip.height <= 0.0
            {
                return Err(Error::InvalidOption {
                    field: "clip",
                    value: format!("{} {} {} {}", clip.x, clip.y, clip.width, clip.height),
                });
            }
        }
        let pixels_per_mm = options.dpi / MILLIMETRES_PER_INCH * options.scale;
        if !pixels_per_mm.is_finite() || pixels_per_mm <= 0.0 {
            return Err(invalid_option("dpi and scale", pixels_per_mm));
        }
        let (width_mm, height_mm) = if matches!(options.rotation_degrees, 90 | 270) {
            (page_box.height, page_box.width)
        } else {
            (page_box.width, page_box.height)
        };
        let width = (width_mm * pixels_per_mm).ceil();
        let height = (height_mm * pixels_per_mm).ceil();
        if !width.is_finite()
            || !height.is_finite()
            || width < 1.0
            || height < 1.0
            || width > f64::from(MAX_CAIRO_IMAGE_DIMENSION)
            || height > f64::from(MAX_CAIRO_IMAGE_DIMENSION)
        {
            return Err(Error::InvalidSurfaceSize { width, height });
        }

        let required_bytes = worst_case_surface_bytes(width as i32, height as i32)
            .ok_or(Error::InvalidSurfaceSize { width, height })?;
        if required_bytes > options.max_raster_bytes {
            return Err(Error::RasterBudgetExceeded {
                required_bytes,
                max_bytes: options.max_raster_bytes,
            });
        }

        let x = page_box.x;
        let y = page_box.y;
        let w = page_box.width;
        let h = page_box.height;
        let page_to_device = match options.rotation_degrees {
            0 => Matrix::new(
                pixels_per_mm,
                0.0,
                0.0,
                pixels_per_mm,
                -pixels_per_mm * x,
                -pixels_per_mm * y,
            ),
            90 => Matrix::new(
                0.0,
                pixels_per_mm,
                -pixels_per_mm,
                0.0,
                pixels_per_mm * (y + h),
                -pixels_per_mm * x,
            ),
            180 => Matrix::new(
                -pixels_per_mm,
                0.0,
                0.0,
                -pixels_per_mm,
                pixels_per_mm * (x + w),
                pixels_per_mm * (y + h),
            ),
            270 => Matrix::new(
                0.0,
                -pixels_per_mm,
                pixels_per_mm,
                0.0,
                -pixels_per_mm * y,
                pixels_per_mm * (x + w),
            ),
            _ => unreachable!("rotation validated"),
        };

        Ok(Self {
            pixel_width: width as i32,
            pixel_height: height as i32,
            page_to_device,
        })
    }
}

fn worst_case_surface_bytes(width: i32, height: i32) -> Option<u64> {
    let width = u32::try_from(width).ok()?;
    let height = u64::try_from(height).ok()?;
    let argb_stride = u64::try_from(Format::ARgb32.stride_for_width(width).ok()?).ok()?;
    let mask_stride = u64::try_from(Format::A8.stride_for_width(width).ok()?).ok()?;

    // Main ARGB32 target + retained A8 clip mask + clipped ARGB32 temporary.
    argb_stride
        .checked_mul(height)?
        .checked_mul(2)?
        .checked_add(mask_stride.checked_mul(height)?)
}

fn invalid_option(field: &'static str, value: f64) -> Error {
    Error::InvalidOption {
        field,
        value: value.to_string(),
    }
}

#[derive(Clone)]
struct PaintState {
    stroke: Option<Color>,
    fill: Option<Color>,
    fill_rule: FillRule,
    line_width: f64,
    line_join: rofd_core::LineJoin,
    line_cap: rofd_core::LineCap,
    miter_limit: f64,
    dash_offset: f64,
    dash_pattern: Vec<f64>,
    alpha: u8,
    clip_mask: Option<ImageSurface>,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            stroke: None,
            fill: None,
            fill_rule: FillRule::NonZero,
            line_width: 1.0,
            line_join: rofd_core::LineJoin::Miter,
            line_cap: rofd_core::LineCap::Butt,
            miter_limit: DEFAULT_MITER_LIMIT,
            dash_offset: 0.0,
            dash_pattern: Vec::new(),
            alpha: 255,
            clip_mask: None,
        }
    }
}

struct Interpreter<'a> {
    context: &'a Context,
    geometry: &'a RenderGeometry,
    state: PaintState,
    stack: Vec<PaintState>,
}

impl<'a> Interpreter<'a> {
    fn new(context: &'a Context, geometry: &'a RenderGeometry) -> Self {
        Self {
            context,
            geometry,
            state: PaintState::default(),
            stack: Vec::new(),
        }
    }

    fn run(&mut self, commands: &[Command]) -> Result<()> {
        let result = self.run_commands(commands);
        if result.is_err() {
            let cleanup = self.unwind();
            return combine_results(result, cleanup, "unwind display state");
        }
        result
    }

    fn unwind(&mut self) -> Result<()> {
        let mut cleanup = Ok(());
        while self.stack.pop().is_some() {
            let restored = cairo(self.context.restore(), "unwind display state");
            cleanup = combine_results(cleanup, restored, "continue display-state unwind");
        }
        cleanup
    }

    fn run_commands(&mut self, commands: &[Command]) -> Result<()> {
        for command in commands {
            match command {
                Command::Save => {
                    cairo(self.context.save(), "save display state")?;
                    self.stack.push(self.state.clone());
                }
                Command::Restore => {
                    let Some(state) = self.stack.last().cloned() else {
                        return Err(invalid_display_list("restore without matching save"));
                    };
                    cairo(self.context.restore(), "restore display state")?;
                    self.stack.pop();
                    self.state = state;
                }
                Command::ConcatTransform(transform) => {
                    self.context.transform(cairo_matrix(*transform));
                    cairo(self.context.status(), "concatenate transform")?;
                }
                Command::ClipPath { paths, rule } => {
                    if paths.is_empty() {
                        return Err(invalid_display_list("empty clip union operand"));
                    }
                    self.state.clip_mask = Some(self.create_clip_mask(paths, *rule)?);
                }
                Command::SetStroke(color) => self.state.stroke = *color,
                Command::SetFill(color) => self.state.fill = *color,
                Command::SetFillRule(rule) => self.state.fill_rule = *rule,
                Command::SetLineWidth(width) => {
                    if !width.is_finite() || *width <= 0.0 {
                        return Err(invalid_display_list(
                            "non-positive or non-finite line width",
                        ));
                    }
                    self.state.line_width = *width;
                }
                Command::SetLineJoin(join) => self.state.line_join = *join,
                Command::SetLineCap(cap) => self.state.line_cap = *cap,
                Command::SetMiterLimit(limit) => {
                    if !limit.is_finite() || *limit <= 0.0 {
                        return Err(invalid_display_list(
                            "non-positive or non-finite miter limit",
                        ));
                    }
                    self.state.miter_limit = *limit;
                }
                Command::SetDash { offset, pattern } => {
                    if !offset.is_finite()
                        || *offset < 0.0
                        || pattern
                            .iter()
                            .any(|value| !value.is_finite() || *value <= 0.0)
                    {
                        return Err(invalid_display_list("invalid stroke dash parameters"));
                    }
                    self.state.dash_offset = *offset;
                    self.state.dash_pattern.clone_from(pattern);
                }
                Command::SetAlpha(alpha) => self.state.alpha = *alpha,
                Command::DrawPath(path) => self.draw_path(path)?,
                Command::DrawGlyphRun(_) => {
                    return Err(Error::UnsupportedDisplayCommand {
                        command: DisplayCommandKind::GlyphRun,
                    });
                }
                Command::DrawImage { .. } => {
                    return Err(Error::UnsupportedDisplayCommand {
                        command: DisplayCommandKind::Image,
                    });
                }
            }
        }
        if !self.stack.is_empty() {
            return Err(invalid_display_list("unclosed display-list save"));
        }
        Ok(())
    }

    fn create_clip_mask(&self, paths: &[ClipPath], rule: FillRule) -> Result<ImageSurface> {
        let surface = cairo(
            ImageSurface::create(
                Format::A8,
                self.geometry.pixel_width,
                self.geometry.pixel_height,
            ),
            "create clip mask",
        )?;
        let context = cairo(Context::new(&surface), "create clip mask context")?;
        context.set_matrix(self.geometry.page_to_device);
        set_raster_defaults(&context)?;
        context.set_fill_rule(cairo_fill_rule(rule));
        context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        cairo(context.status(), "initialize clip mask state")?;

        for path in paths {
            cairo(context.save(), "save clip path state")?;
            let filled = (|| {
                context.transform(cairo_matrix(path.transform()));
                append_path(&context, path.path())?;
                cairo(context.fill(), "fill clip area")
            })();
            let restored = cairo(context.restore(), "restore clip path state");
            combine_results(filled, restored, "restore clip path state")?;
        }

        if let Some(previous) = &self.state.clip_mask {
            context.identity_matrix();
            context.set_operator(Operator::In);
            cairo(context.status(), "prepare clip-mask intersection")?;
            cairo(
                context.set_source_surface(previous, 0.0, 0.0),
                "set prior clip mask",
            )?;
            cairo(context.paint(), "intersect clip masks")?;
        }
        surface.flush();
        cairo(surface.status(), "flush clip mask")?;
        Ok(surface)
    }

    fn draw_path(&self, path: &PathData) -> Result<()> {
        if let Some(mask) = &self.state.clip_mask {
            let surface = cairo(
                ImageSurface::create(
                    Format::ARgb32,
                    self.geometry.pixel_width,
                    self.geometry.pixel_height,
                ),
                "create clipped drawing surface",
            )?;
            let temporary = cairo(Context::new(&surface), "create clipped drawing context")?;
            temporary.set_matrix(self.context.matrix());
            set_raster_defaults(&temporary)?;
            set_stroke_defaults(&temporary)?;
            cairo(temporary.status(), "initialize clipped drawing state")?;
            draw_path_unmasked(&temporary, &self.state, path)?;
            surface.flush();
            cairo(surface.status(), "flush clipped drawing surface")?;

            cairo(self.context.save(), "save clipped composite state")?;
            let composite = (|| {
                self.context.identity_matrix();
                cairo(self.context.status(), "prepare clipped composite")?;
                cairo(
                    self.context.set_source_surface(&surface, 0.0, 0.0),
                    "set clipped drawing source",
                )?;
                cairo(
                    self.context.mask_surface(mask, 0.0, 0.0),
                    "apply clip union mask",
                )
            })();
            let restored = cairo(self.context.restore(), "restore clipped composite state");
            combine_results(composite, restored, "restore clipped composite state")
        } else {
            draw_path_unmasked(self.context, &self.state, path)
        }
    }
}

fn draw_path_unmasked(context: &Context, state: &PaintState, path: &PathData) -> Result<()> {
    append_path(context, path)?;
    context.set_fill_rule(cairo_fill_rule(state.fill_rule));
    context.set_line_width(state.line_width);
    context.set_line_join(match state.line_join {
        rofd_core::LineJoin::Miter => LineJoin::Miter,
        rofd_core::LineJoin::Round => LineJoin::Round,
        rofd_core::LineJoin::Bevel => LineJoin::Bevel,
    });
    context.set_line_cap(match state.line_cap {
        rofd_core::LineCap::Butt => LineCap::Butt,
        rofd_core::LineCap::Round => LineCap::Round,
        rofd_core::LineCap::Square => LineCap::Square,
    });
    context.set_miter_limit(state.miter_limit);
    context.set_dash(&state.dash_pattern, state.dash_offset);
    cairo(context.status(), "apply path paint state")?;

    match (state.fill, state.stroke) {
        (Some(fill), Some(stroke)) => {
            set_source_color(context, fill)?;
            cairo(context.fill_preserve(), "fill path")?;
            set_source_color(context, stroke)?;
            cairo(context.stroke(), "stroke path")?;
        }
        (Some(fill), None) => {
            set_source_color(context, fill)?;
            cairo(context.fill(), "fill path")?;
        }
        (None, Some(stroke)) => {
            set_source_color(context, stroke)?;
            cairo(context.stroke(), "stroke path")?;
        }
        (None, None) => context.new_path(),
    }
    cairo(context.status(), "finish drawing path")
}

fn append_path(context: &Context, path: &PathData) -> Result<()> {
    context.new_path();
    let mut current: Option<Point> = None;
    let mut subpath_start: Option<Point> = None;
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(point) => {
                context.move_to(point.x(), point.y());
                current = Some(point);
                subpath_start = Some(point);
            }
            PathCommand::LineTo(point) => {
                context.line_to(point.x(), point.y());
                current = Some(point);
            }
            PathCommand::QuadraticTo { control, end } => {
                let start =
                    current.ok_or_else(|| invalid_display_list("quadratic without start"))?;
                context.curve_to(
                    start.x() + (control.x() - start.x()) * 2.0 / 3.0,
                    start.y() + (control.y() - start.y()) * 2.0 / 3.0,
                    end.x() + (control.x() - end.x()) * 2.0 / 3.0,
                    end.y() + (control.y() - end.y()) * 2.0 / 3.0,
                    end.x(),
                    end.y(),
                );
                current = Some(end);
            }
            PathCommand::CubicTo {
                control1,
                control2,
                end,
            } => {
                context.curve_to(
                    control1.x(),
                    control1.y(),
                    control2.x(),
                    control2.y(),
                    end.x(),
                    end.y(),
                );
                current = Some(end);
            }
            PathCommand::ArcTo {
                rx,
                ry,
                rotation,
                large,
                sweep,
                end,
            } => {
                let start = current.ok_or_else(|| invalid_display_list("arc without start"))?;
                append_arc(context, start, rx, ry, rotation, large, sweep, end)?;
                current = Some(end);
            }
            PathCommand::Close => {
                context.close_path();
                current = subpath_start;
            }
        }
    }
    cairo(context.status(), "construct path")
}

#[allow(clippy::too_many_arguments)]
fn append_arc(
    context: &Context,
    start: Point,
    mut rx: f64,
    mut ry: f64,
    rotation_degrees: f64,
    large: bool,
    sweep: bool,
    end: Point,
) -> Result<()> {
    if rx == 0.0 || ry == 0.0 || (start.x() == end.x() && start.y() == end.y()) {
        if start.x() != end.x() || start.y() != end.y() {
            context.line_to(end.x(), end.y());
        }
        return Ok(());
    }
    if !rx.is_finite() || !ry.is_finite() || !rotation_degrees.is_finite() {
        return Err(invalid_arc_geometry(
            "input radii or rotation",
            format!("rx={rx}, ry={ry}, rotation={rotation_degrees}"),
        ));
    }
    rx = rx.abs();
    ry = ry.abs();
    let phi = rotation_degrees.to_radians().rem_euclid(2.0 * PI);
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx = (start.x() - end.x()) / 2.0;
    let dy = (start.y() - end.y()) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;
    let radii_scale = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if !radii_scale.is_finite() {
        return Err(invalid_arc_geometry(
            "radius correction",
            radii_scale.to_string(),
        ));
    }
    if radii_scale > 1.0 {
        let scale = radii_scale.sqrt();
        rx *= scale;
        ry *= scale;
    }

    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let x12 = x1 * x1;
    let y12 = y1 * y1;
    let denominator = rx2 * y12 + ry2 * x12;
    if !denominator.is_finite() {
        return Err(invalid_arc_geometry(
            "center denominator",
            denominator.to_string(),
        ));
    }
    if denominator == 0.0 {
        context.line_to(end.x(), end.y());
        return Ok(());
    }
    let numerator = (rx2 * ry2 - denominator).max(0.0);
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let coefficient = sign * (numerator / denominator).sqrt();
    let cx1 = coefficient * (rx * y1 / ry);
    let cy1 = coefficient * (-ry * x1 / rx);
    let cx = cos_phi * cx1 - sin_phi * cy1 + (start.x() + end.x()) / 2.0;
    let cy = sin_phi * cx1 + cos_phi * cy1 + (start.y() + end.y()) / 2.0;

    let ux = (x1 - cx1) / rx;
    let uy = (y1 - cy1) / ry;
    let vx = (-x1 - cx1) / rx;
    let vy = (-y1 - cy1) / ry;
    let start_angle = uy.atan2(ux);
    let mut delta = vector_angle(ux, uy, vx, vy);
    if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    } else if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    }
    if !cx.is_finite() || !cy.is_finite() || !start_angle.is_finite() || !delta.is_finite() {
        return Err(invalid_arc_geometry(
            "derived center or angle",
            format!("center=({cx}, {cy}), start={start_angle}, sweep={delta}"),
        ));
    }

    cairo(context.save(), "save arc transform state")?;
    let drawn = (|| {
        context.translate(cx, cy);
        context.rotate(phi);
        context.scale(rx, ry);
        cairo(context.status(), "apply arc ellipse transform")?;
        if sweep {
            context.arc(0.0, 0.0, 1.0, start_angle, start_angle + delta);
        } else {
            context.arc_negative(0.0, 0.0, 1.0, start_angle, start_angle + delta);
        }
        cairo(context.status(), "append elliptical arc")
    })();
    let restored = cairo(context.restore(), "restore arc transform state");
    combine_results(drawn, restored, "restore arc transform state")
}

fn vector_angle(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    (ux * vy - uy * vx).atan2(ux * vx + uy * vy)
}

fn cairo_matrix(transform: Transform) -> Matrix {
    Matrix::new(
        transform.a(),
        transform.b(),
        transform.c(),
        transform.d(),
        transform.e(),
        transform.f(),
    )
}

fn cairo_fill_rule(rule: FillRule) -> CairoFillRule {
    match rule {
        FillRule::NonZero => CairoFillRule::Winding,
        FillRule::EvenOdd => CairoFillRule::EvenOdd,
    }
}

fn set_source_color(context: &Context, color: Color) -> Result<()> {
    context.set_source_rgba(
        f64::from(color.red) / 255.0,
        f64::from(color.green) / 255.0,
        f64::from(color.blue) / 255.0,
        f64::from(color.alpha) / 255.0,
    );
    cairo(context.status(), "set source color")
}

fn set_stroke_defaults(context: &Context) -> Result<()> {
    context.set_dash(&[], 0.0);
    context.set_line_cap(LineCap::Butt);
    context.set_line_join(LineJoin::Miter);
    context.set_miter_limit(DEFAULT_MITER_LIMIT);
    cairo(context.status(), "set default stroke parameters")
}

fn set_raster_defaults(context: &Context) -> Result<()> {
    context.set_antialias(Antialias::Gray);
    context.set_tolerance(DEFAULT_CURVE_TOLERANCE);
    cairo(context.status(), "set default raster parameters")
}

fn restore_path(context: &Context, path: &Path) -> Result<()> {
    context.new_path();
    context.append_path(path);
    cairo(context.status(), "restore caller path")
}

fn invalid_display_list(message: impl Into<String>) -> Error {
    Error::InvalidDisplayList {
        message: message.into(),
    }
}

fn invalid_arc_geometry(field: &'static str, value: impl Into<String>) -> Error {
    Error::InvalidGeometry {
        primitive: "arc",
        field,
        value: value.into(),
    }
}

fn cairo<T>(result: std::result::Result<T, cairo::Error>, operation: &'static str) -> Result<T> {
    result.map_err(|source| Error::Backend { operation, source })
}

fn combine_results<T>(
    primary: Result<T>,
    cleanup: Result<()>,
    operation: &'static str,
) -> Result<T> {
    match (primary, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(Error::Cleanup {
            operation,
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
