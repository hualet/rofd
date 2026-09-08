use std::collections::HashMap;
use std::f64::consts::PI;
use std::io::Cursor;

use cairo::{
    Antialias, Context, Extend, FillRule as CairoFillRule, Filter, FontFace, FontOptions, Format,
    Glyph, HintMetrics, HintStyle, ImageSurface, LineCap, LineJoin, Matrix, Operator, Path,
    SubpixelOrder, SurfacePattern,
};
use image::{ImageReader, Limits};
use rofd_core::{
    Color, FillRule, Page, PathCommand, PathData, Point, Rect, SealPictureKind, StampAnnotation,
    Transform,
};

use crate::fonts::{FontAllocationKey, SystemFontResolver};
use crate::{
    ClipPath, Command, DisplayList, DisplayListBuilder, Error, FontResolver, GlyphRun,
    ImageDecoder, RenderDiagnostic, Result,
};

const MILLIMETRES_PER_INCH: f64 = 25.4;
const DEFAULT_MITER_LIMIT: f64 = 3.528;
const DEFAULT_CURVE_TOLERANCE: f64 = 0.1;
const MAX_CAIRO_IMAGE_DIMENSION: i32 = 32_767;
const DEFAULT_MAX_RASTER_BYTES: u64 = 256 * 1024 * 1024;

/// Sampling filter used while scaling decoded raster images.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ImageInterpolation {
    /// Select the closest source pixel without blending neighbors.
    Nearest,
    /// Blend adjacent source pixels for smoother scaling.
    #[default]
    Bilinear,
}

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
    /// Sampling filter used while mapping images into their object boundaries.
    pub image_interpolation: ImageInterpolation,
    /// Maximum bytes for all simultaneously live raster working surfaces.
    ///
    /// Validation conservatively includes the caller's ARGB32 target, one A8
    /// clip mask, one ARGB32 clipped-drawing intermediate, and one retained
    /// native premultiplied buffer per unique decoded image allocation.
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
            image_interpolation: ImageInterpolation::Bilinear,
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
        let geometry = validate_render_target(page, context, options)?;
        let caller_path = cairo(context.copy_path(), "capture caller path")?;
        let rendered = DisplayList::from_page(page).and_then(|display_list| {
            self.render_display_list(
                page,
                context,
                options,
                &geometry,
                display_list,
                0,
                None,
                None,
            )
        });
        let restored_path = restore_path(context, &caller_path);
        combine_results(rendered, restored_path, "restore caller path")
    }

    /// Renders with caller-provided reusable font and image services.
    ///
    /// This entry point is useful when applications require an explicit system
    /// font snapshot, ordered fallback families, or shared bounded caches.
    pub fn render_page_with_services(
        &self,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
        font_resolver: &dyn FontResolver,
        image_decoder: &ImageDecoder,
    ) -> Result<RenderReport> {
        self.render_with_services(page, context, options, font_resolver, image_decoder, 0)
    }

    fn render_with_services(
        &self,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
        font_resolver: &dyn FontResolver,
        image_decoder: &ImageDecoder,
        depth: u32,
    ) -> Result<RenderReport> {
        let geometry = validate_render_target(page, context, options)?;
        let caller_path = cairo(context.copy_path(), "capture caller path")?;
        let rendered = DisplayListBuilder::new(font_resolver, image_decoder)
            .build(page)
            .and_then(|display_list| {
                self.render_display_list(
                    page,
                    context,
                    options,
                    &geometry,
                    display_list,
                    depth,
                    Some(font_resolver),
                    Some(image_decoder),
                )
            });
        let restored_path = restore_path(context, &caller_path);
        combine_results(rendered, restored_path, "restore caller path")
    }

    #[allow(clippy::too_many_arguments)]
    fn render_display_list(
        &self,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
        geometry: &RenderGeometry,
        display_list: DisplayList,
        depth: u32,
        font_resolver: Option<&dyn FontResolver>,
        image_decoder: Option<&ImageDecoder>,
    ) -> Result<RenderReport> {
        let prepared_text = PreparedText::new(
            display_list.commands(),
            page.resource_limits().max_glyphs_per_page,
        )?;
        let prepared_images =
            PreparedImages::new(display_list.commands(), geometry, options.max_raster_bytes)?;
        let prepared = PreparedRender {
            display_list,
            text: prepared_text,
            images: prepared_images,
        };
        cairo(context.save(), "save caller state")?;

        let rendered = render_saved(context, page.size(), options, geometry, &prepared);
        if rendered.is_ok() {
            self.draw_stamp_annotations(
                page,
                context,
                options,
                geometry,
                depth,
                font_resolver,
                image_decoder,
            );
        }
        let restored = cairo(context.restore(), "restore caller state");
        combine_results(rendered, restored, "restore caller state").map(|()| RenderReport {
            diagnostics: prepared.display_list.diagnostics().to_vec(),
        })
    }

    /// Paints this page's signature stamp annotations over the rendered
    /// content, like ofdrw's `AWTMaker.writeStampAnnot`. Each stamp that
    /// cannot be decoded or painted is skipped silently.
    #[allow(clippy::too_many_arguments)]
    fn draw_stamp_annotations(
        &self,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
        geometry: &RenderGeometry,
        depth: u32,
        font_resolver: Option<&dyn FontResolver>,
        image_decoder: Option<&ImageDecoder>,
    ) {
        let annotations = page.stamp_annotations();
        if annotations.is_empty() {
            return;
        }
        // `render_page` callers supply no services; defaults are created on
        // first use for embedded mini-OFD seals only.
        let mut owned_resolver = None::<SystemFontResolver>;
        let mut owned_decoder = None::<ImageDecoder>;
        for annotation in &annotations {
            let _ = self.draw_stamp_annotation(
                annotation,
                page,
                context,
                options,
                geometry,
                depth,
                font_resolver,
                image_decoder,
                &mut owned_resolver,
                &mut owned_decoder,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_stamp_annotation(
        &self,
        annotation: &StampAnnotation,
        page: &Page,
        context: &Context,
        options: &RenderOptions,
        geometry: &RenderGeometry,
        depth: u32,
        font_resolver: Option<&dyn FontResolver>,
        image_decoder: Option<&ImageDecoder>,
        owned_resolver: &mut Option<SystemFontResolver>,
        owned_decoder: &mut Option<ImageDecoder>,
    ) -> Result<()> {
        let boundary = annotation.boundary;
        if !boundary.x.is_finite()
            || !boundary.y.is_finite()
            || !boundary.width.is_finite()
            || !boundary.height.is_finite()
            || boundary.width <= 0.0
            || boundary.height <= 0.0
        {
            return Err(invalid_display_list(
                "stamp annotation boundary must be finite and positive",
            ));
        }
        let picture = &annotation.picture;
        let prepared = match &picture.kind {
            SealPictureKind::Ofd => {
                // A mini-OFD seal could itself carry a seal; recurse once,
                // like ofdrw's single ImageMaker pass.
                if depth >= 1 {
                    return Ok(());
                }
                let (resolver, decoder): (&dyn FontResolver, &ImageDecoder) =
                    match (font_resolver, image_decoder) {
                        (Some(resolver), Some(decoder)) => (resolver, decoder),
                        _ => (
                            owned_resolver.get_or_insert_with(|| {
                                SystemFontResolver::empty(
                                    Vec::new(),
                                    page.resource_limits().max_font_bytes,
                                )
                            }),
                            owned_decoder.get_or_insert_with(ImageDecoder::default),
                        ),
                    };
                self.render_seal_ofd(&picture.data, options, resolver, decoder, depth)?
            }
            SealPictureKind::Png
            | SealPictureKind::Jpeg
            | SealPictureKind::Gif
            | SealPictureKind::Bmp => decode_seal_raster(
                &picture.data,
                page.resource_limits(),
                options.max_raster_bytes,
            )?,
            _ => return Ok(()),
        };

        cairo(context.save(), "save stamp state")?;
        let drawn = (|| {
            context.set_matrix(geometry.page_to_device);
            if let Some(clip) = annotation.clip {
                context.rectangle(
                    boundary.x + clip.x,
                    boundary.y + clip.y,
                    clip.width,
                    clip.height,
                );
                context.clip();
            }
            context.translate(boundary.x, boundary.y);
            context.rectangle(0.0, 0.0, boundary.width, boundary.height);
            context.clip();
            context.scale(
                boundary.width / f64::from(prepared.width),
                boundary.height / f64::from(prepared.height),
            );
            let pattern = SurfacePattern::create(&prepared.surface);
            pattern.set_extend(Extend::Pad);
            pattern.set_filter(match options.image_interpolation {
                ImageInterpolation::Nearest => Filter::Nearest,
                ImageInterpolation::Bilinear => Filter::Bilinear,
            });
            cairo(context.set_source(&pattern), "set stamp source")?;
            cairo(context.paint(), "paint stamp")
        })();
        let restored = cairo(context.restore(), "restore stamp state");
        combine_results(drawn, restored, "restore stamp state")
    }

    /// Renders the first page of a mini-OFD seal picture to a transparent
    /// surface, matching ofdrw's stamp pipeline.
    fn render_seal_ofd(
        &self,
        data: &[u8],
        options: &RenderOptions,
        font_resolver: &dyn FontResolver,
        image_decoder: &ImageDecoder,
        depth: u32,
    ) -> Result<PreparedImage> {
        let document =
            rofd_core::Document::from_bytes(data.to_vec(), rofd_core::LoadOptions::default())
                .map_err(|error| {
                    invalid_display_list(format!("seal mini document could not be loaded: {error}"))
                })?;
        if document.page_count() == 0 {
            return Err(invalid_display_list("seal mini document has no pages"));
        }
        let seal_page = document.page(0).map_err(|error| {
            invalid_display_list(format!(
                "seal mini document page could not be loaded: {error}"
            ))
        })?;
        let seal_options = RenderOptions {
            rotation_degrees: 0,
            clip: None,
            background: Color {
                alpha: 0,
                ..options.background
            },
            ..options.clone()
        };
        let seal_geometry = RenderGeometry::new(seal_page.size(), &seal_options)?;
        let surface = cairo(
            ImageSurface::create(
                Format::ARgb32,
                seal_geometry.pixel_width,
                seal_geometry.pixel_height,
            ),
            "create seal surface",
        )?;
        let context = cairo(Context::new(&surface), "create seal context")?;
        let rendered = self.render_with_services(
            &seal_page,
            &context,
            &seal_options,
            font_resolver,
            image_decoder,
            depth + 1,
        );
        drop(context);
        surface.flush();
        let flushed = cairo(surface.status(), "flush seal surface");
        combine_results(rendered.map(drop), flushed, "flush seal surface")?;
        Ok(PreparedImage {
            surface,
            width: seal_geometry.pixel_width,
            height: seal_geometry.pixel_height,
        })
    }
}

fn validate_render_target(
    page: &Page,
    context: &Context,
    options: &RenderOptions,
) -> Result<RenderGeometry> {
    let geometry = RenderGeometry::new(page.size(), options)?;
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
    Ok(geometry)
}

fn render_saved(
    context: &Context,
    page_box: Rect,
    options: &RenderOptions,
    geometry: &RenderGeometry,
    prepared: &PreparedRender,
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

    let mut interpreter = Interpreter::new(
        context,
        geometry,
        &prepared.text,
        &prepared.images,
        options.image_interpolation,
    );
    interpreter.run(prepared.display_list.commands())
}

fn cairo_image_layout(image: &crate::DecodedImage) -> Result<(i32, i32, u64)> {
    if image.width() > MAX_CAIRO_IMAGE_DIMENSION as u32
        || image.height() > MAX_CAIRO_IMAGE_DIMENSION as u32
    {
        return Err(Error::InvalidImageSurfaceSize {
            resource_id: image.resource_id(),
            width: image.width(),
            height: image.height(),
        });
    }
    let width = i32::try_from(image.width())
        .map_err(|_| invalid_display_list("image width exceeds the Cairo integer domain"))?;
    let height = i32::try_from(image.height())
        .map_err(|_| invalid_display_list("image height exceeds the Cairo integer domain"))?;
    let stride = Format::ARgb32
        .stride_for_width(image.width())
        .map_err(|_| invalid_display_list("image stride exceeds the Cairo integer domain"))?;
    let bytes = u64::try_from(stride)
        .ok()
        .and_then(|stride| stride.checked_mul(u64::from(image.height())))
        .ok_or_else(|| invalid_display_list("image raster byte length overflow"))?;
    Ok((width, height, bytes))
}

struct PreparedImages {
    images: Vec<Option<PreparedImage>>,
}

struct PreparedRender {
    display_list: DisplayList,
    text: PreparedText,
    images: PreparedImages,
}

#[derive(Clone)]
struct PreparedImage {
    surface: ImageSurface,
    width: i32,
    height: i32,
}

impl PreparedImages {
    fn new(commands: &[Command], geometry: &RenderGeometry, max_bytes: u64) -> Result<Self> {
        let mut allocations = HashMap::<usize, u64>::new();
        for command in commands {
            let Command::DrawImage {
                image,
                width,
                height,
            } = command
            else {
                continue;
            };
            if !width.is_finite() || !height.is_finite() || *width <= 0.0 || *height <= 0.0 {
                return Err(invalid_display_list(
                    "image boundary must contain finite positive dimensions",
                ));
            }
            let bytes = cairo_image_layout(image)?.2;
            allocations.entry(image.allocation_id()).or_insert(bytes);
        }
        let native_bytes = allocations.values().try_fold(0u64, |total, bytes| {
            total
                .checked_add(*bytes)
                .ok_or(Error::RasterBudgetExceeded {
                    required_bytes: u64::MAX,
                    max_bytes,
                })
        })?;
        let required_bytes = geometry.surface_bytes.checked_add(native_bytes).ok_or(
            Error::RasterBudgetExceeded {
                required_bytes: u64::MAX,
                max_bytes,
            },
        )?;
        if required_bytes > max_bytes {
            return Err(Error::RasterBudgetExceeded {
                required_bytes,
                max_bytes,
            });
        }

        let mut surfaces = HashMap::<usize, PreparedImage>::new();
        let mut images = std::iter::repeat_with(|| None)
            .take(commands.len())
            .collect::<Vec<_>>();
        for (index, command) in commands.iter().enumerate() {
            let Command::DrawImage { image, .. } = command else {
                continue;
            };
            let key = image.allocation_id();
            let prepared = if let Some(prepared) = surfaces.get(&key) {
                prepared.clone()
            } else {
                let prepared = prepare_image_surface(image)?;
                surfaces.insert(key, prepared.clone());
                prepared
            };
            images[index] = Some(prepared);
        }
        Ok(Self { images })
    }
}

struct PreparedText {
    runs: Vec<Option<PreparedGlyphRun>>,
}

struct PreparedGlyphRun {
    size_mm: f64,
    batches: Vec<PreparedGlyphBatch>,
}

struct PreparedGlyphBatch {
    transform: Option<Transform>,
    content: PreparedGlyphContent,
}

enum PreparedGlyphContent {
    Font {
        key: FontAllocationKey,
        face: FontFace,
        glyphs: Vec<Glyph>,
    },
    Missing {
        boxes: Vec<(f64, f64, f64, f64)>,
    },
}

impl PreparedText {
    fn new(commands: &[Command], max_glyphs: usize) -> Result<Self> {
        let mut faces = HashMap::<FontAllocationKey, FontFace>::new();
        let mut glyph_count = 0usize;
        let mut runs = std::iter::repeat_with(|| None)
            .take(commands.len())
            .collect::<Vec<_>>();
        for (index, command) in commands.iter().enumerate() {
            let Command::DrawGlyphRun(run) = command else {
                continue;
            };
            glyph_count = glyph_count.checked_add(run.glyphs().len()).ok_or_else(|| {
                invalid_display_list("glyph count overflow while preparing Cairo text")
            })?;
            if glyph_count > max_glyphs {
                return Err(invalid_display_list(
                    "display glyph count exceeds the page glyph limit",
                ));
            }
            runs[index] = Some(PreparedGlyphRun::new(run, &mut faces)?);
        }
        Ok(Self { runs })
    }
}

impl PreparedGlyphRun {
    fn new(run: &GlyphRun, faces: &mut HashMap<FontAllocationKey, FontFace>) -> Result<Self> {
        if !run.size_mm().is_finite() || run.size_mm() <= 0.0 {
            return Err(invalid_display_list(
                "glyph run size must be finite and positive",
            ));
        }
        for glyph in run.glyphs() {
            if !glyph.x().is_finite() || !glyph.y().is_finite() {
                return Err(invalid_display_list(
                    "glyph position must contain finite coordinates",
                ));
            }
            if glyph.is_synthetic_box() {
                if glyph.font().is_some() {
                    return Err(invalid_display_list(
                        "synthetic missing glyph unexpectedly contains a font",
                    ));
                }
            } else if glyph.font().is_none() {
                return Err(invalid_display_list(
                    "font-backed glyph does not contain a resolved font",
                ));
            }
        }

        let mut batches = Vec::<PreparedGlyphBatch>::new();
        for positioned in run.glyphs() {
            let transform = positioned.transform();
            if positioned.is_synthetic_box() {
                let width = run.size_mm() * 0.6;
                let top = positioned.y() - run.size_mm();
                if !width.is_finite() || !top.is_finite() {
                    return Err(invalid_display_list("synthetic glyph box overflow"));
                }
                match batches.last_mut() {
                    Some(PreparedGlyphBatch {
                        transform: prior,
                        content: PreparedGlyphContent::Missing { boxes },
                    }) if *prior == transform => {
                        boxes.push((positioned.x(), top, width, run.size_mm()));
                    }
                    _ => batches.push(PreparedGlyphBatch {
                        transform,
                        content: PreparedGlyphContent::Missing {
                            boxes: vec![(positioned.x(), top, width, run.size_mm())],
                        },
                    }),
                }
                continue;
            }

            let font = positioned
                .font()
                .expect("font presence was validated before batching");
            if positioned.glyph_id() >= font.glyph_count() {
                return Err(invalid_display_list(
                    "positioned glyph identifier exceeds its resolved face",
                ));
            }
            let glyph_index = positioned.glyph_id().into();
            let key = font.allocation_key();
            let face = if let Some(face) = faces.get(&key) {
                face.clone()
            } else {
                let face = font.create_cairo_font_face()?;
                faces.insert(key, face.clone());
                face
            };
            let cairo_glyph = Glyph::new(glyph_index, positioned.x(), positioned.y());
            match batches.last_mut() {
                Some(PreparedGlyphBatch {
                    transform: prior,
                    content:
                        PreparedGlyphContent::Font {
                            key: prior_key,
                            glyphs,
                            ..
                        },
                }) if *prior == transform && *prior_key == key => glyphs.push(cairo_glyph),
                _ => batches.push(PreparedGlyphBatch {
                    transform,
                    content: PreparedGlyphContent::Font {
                        key,
                        face,
                        glyphs: vec![cairo_glyph],
                    },
                }),
            }
        }
        Ok(Self {
            size_mm: run.size_mm(),
            batches,
        })
    }
}

struct RenderGeometry {
    pixel_width: i32,
    pixel_height: i32,
    page_to_device: Matrix,
    surface_bytes: u64,
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
            surface_bytes: required_bytes,
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
    prepared_text: &'a PreparedText,
    prepared_images: &'a PreparedImages,
    image_interpolation: ImageInterpolation,
    state: PaintState,
    stack: Vec<PaintState>,
}

impl<'a> Interpreter<'a> {
    fn new(
        context: &'a Context,
        geometry: &'a RenderGeometry,
        prepared_text: &'a PreparedText,
        prepared_images: &'a PreparedImages,
        image_interpolation: ImageInterpolation,
    ) -> Self {
        Self {
            context,
            geometry,
            prepared_text,
            prepared_images,
            image_interpolation,
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
        for (index, command) in commands.iter().enumerate() {
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
                Command::DrawGlyphRun(_) => self.draw_glyph_run(
                    self.prepared_text.runs[index]
                        .as_ref()
                        .ok_or_else(|| invalid_display_list("glyph run was not prepared"))?,
                )?,
                Command::DrawImage { width, height, .. } => self.draw_image(
                    self.prepared_images.images[index]
                        .as_ref()
                        .ok_or_else(|| invalid_display_list("image was not prepared"))?,
                    *width,
                    *height,
                )?,
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

    fn draw_with_clip(&self, draw_unmasked: impl FnOnce(&Context) -> Result<()>) -> Result<()> {
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
            draw_unmasked(&temporary)?;
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
            draw_unmasked(self.context)
        }
    }

    fn draw_path(&self, path: &PathData) -> Result<()> {
        self.draw_with_clip(|context| draw_path_unmasked(context, &self.state, path))
    }

    fn draw_glyph_run(&self, run: &PreparedGlyphRun) -> Result<()> {
        self.draw_with_clip(|context| draw_glyph_run_unmasked(context, &self.state, run))
    }

    fn draw_image(&self, image: &PreparedImage, width: f64, height: f64) -> Result<()> {
        self.draw_with_clip(|context| {
            draw_image_unmasked(
                context,
                &self.state,
                image,
                width,
                height,
                self.image_interpolation,
            )
        })
    }
}

fn draw_image_unmasked(
    context: &Context,
    state: &PaintState,
    image: &PreparedImage,
    width: f64,
    height: f64,
    interpolation: ImageInterpolation,
) -> Result<()> {
    let pattern = SurfacePattern::create(&image.surface);
    pattern.set_extend(Extend::Pad);
    pattern.set_filter(match interpolation {
        ImageInterpolation::Nearest => Filter::Nearest,
        ImageInterpolation::Bilinear => Filter::Bilinear,
    });

    cairo(context.save(), "save image state")?;
    let drawn = (|| {
        context.rectangle(0.0, 0.0, width, height);
        context.clip();
        context.scale(
            width / f64::from(image.width),
            height / f64::from(image.height),
        );
        cairo(context.set_source(&pattern), "set image source")?;
        cairo(
            context.paint_with_alpha(f64::from(state.alpha) / 255.0),
            "composite image",
        )
    })();
    let restored = cairo(context.restore(), "restore image state");
    combine_results(drawn, restored, "restore image state")
}

fn prepare_image_surface(image: &crate::DecodedImage) -> Result<PreparedImage> {
    let (width, height, byte_len) = cairo_image_layout(image)?;
    let stride = Format::ARgb32
        .stride_for_width(image.width())
        .map_err(|_| invalid_display_list("image stride exceeds the Cairo integer domain"))?;
    let surface = premultiply_rgba_to_surface(
        width,
        height,
        stride,
        byte_len,
        image.stride(),
        image.rgba(),
    )?;
    Ok(PreparedImage {
        surface,
        width,
        height,
    })
}

/// Decodes a raster seal picture (PNG/JPEG/GIF/BMP) into a Cairo source.
fn decode_seal_raster(
    data: &[u8],
    limits: &rofd_core::ResourceLimits,
    max_raster_bytes: u64,
) -> Result<PreparedImage> {
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|error| {
            invalid_display_list(format!(
                "seal picture format could not be detected: {error}"
            ))
        })?;
    let mut decoder_limits = Limits::default();
    decoder_limits.max_alloc = Some(limits.max_decoded_image_bytes.min(max_raster_bytes));
    reader.limits(decoder_limits);
    let decoded = reader.decode().map_err(|error| {
        invalid_display_list(format!("seal picture could not be decoded: {error}"))
    })?;
    let rgba = decoded.into_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0
        || height == 0
        || width > MAX_CAIRO_IMAGE_DIMENSION as u32
        || height > MAX_CAIRO_IMAGE_DIMENSION as u32
    {
        return Err(invalid_display_list(
            "seal picture dimensions are not representable",
        ));
    }
    let decoded_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| invalid_display_list("seal picture size overflow"))?;
    if decoded_bytes > max_raster_bytes {
        return Err(Error::RasterBudgetExceeded {
            required_bytes: decoded_bytes,
            max_bytes: max_raster_bytes,
        });
    }
    let stride = Format::ARgb32.stride_for_width(width).map_err(|_| {
        invalid_display_list("seal picture stride exceeds the Cairo integer domain")
    })?;
    let source_stride = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(|| invalid_display_list("seal picture stride exceeds address space"))?;
    let surface = premultiply_rgba_to_surface(
        width as i32,
        height as i32,
        stride,
        decoded_bytes,
        source_stride,
        rgba.as_raw(),
    )?;
    Ok(PreparedImage {
        surface,
        width: width as i32,
        height: height as i32,
    })
}

/// Converts tightly packed or strided RGBA8 rows into a premultiplied ARGB32
/// Cairo surface (native-endian BGRA bytes in memory).
fn premultiply_rgba_to_surface(
    width: i32,
    height: i32,
    stride: i32,
    byte_len: u64,
    source_stride: usize,
    rgba: &[u8],
) -> Result<ImageSurface> {
    let capacity = usize::try_from(byte_len)
        .map_err(|_| invalid_display_list("image raster byte length exceeds address space"))?;
    let mut native = Vec::new();
    native
        .try_reserve_exact(capacity)
        .map_err(|_| Error::RasterAllocation {
            required_bytes: byte_len,
        })?;
    native.resize(capacity, 0);
    let target_stride =
        usize::try_from(stride).map_err(|_| invalid_display_list("negative Cairo image stride"))?;
    let source_width = usize::try_from(width)
        .map_err(|_| invalid_display_list("image width exceeds address space"))?;
    let source_height = usize::try_from(height)
        .map_err(|_| invalid_display_list("image height exceeds address space"))?;
    for y in 0..source_height {
        for x in 0..source_width {
            let source = y * source_stride + x * 4;
            let target = y * target_stride + x * 4;
            let alpha = u32::from(rgba[source + 3]);
            let red = (u32::from(rgba[source]) * alpha + 127) / 255;
            let green = (u32::from(rgba[source + 1]) * alpha + 127) / 255;
            let blue = (u32::from(rgba[source + 2]) * alpha + 127) / 255;
            let pixel = (alpha << 24) | (red << 16) | (green << 8) | blue;
            native[target..target + 4].copy_from_slice(&pixel.to_ne_bytes());
        }
    }
    cairo(
        ImageSurface::create_for_data(native, Format::ARgb32, width, height, stride),
        "create image source surface",
    )
}

fn draw_glyph_run_unmasked(
    context: &Context,
    state: &PaintState,
    run: &PreparedGlyphRun,
) -> Result<()> {
    apply_stroke_state(context, state)?;
    for batch in &run.batches {
        cairo(context.save(), "save glyph batch state")?;
        let drawn = (|| {
            if let Some(transform) = batch.transform {
                context.transform(cairo_matrix(transform));
                cairo(context.status(), "apply glyph transform")?;
            }
            match &batch.content {
                PreparedGlyphContent::Font { face, glyphs, .. } => {
                    context.set_font_face(face);
                    context.set_font_size(run.size_mm);
                    cairo(context.status(), "apply glyph font")?;
                    if state.stroke.is_none() {
                        if let Some(fill) = state.fill {
                            set_source_color(context, fill)?;
                            cairo(context.show_glyphs(glyphs), "fill positioned glyphs")?;
                        }
                    } else {
                        context.new_path();
                        context.glyph_path(glyphs);
                        cairo(context.status(), "construct positioned glyph path")?;
                        paint_current_path(context, state)?;
                    }
                }
                PreparedGlyphContent::Missing { boxes } => {
                    context.new_path();
                    for &(x, y, width, height) in boxes {
                        context.rectangle(x, y, width, height);
                    }
                    cairo(context.status(), "construct missing glyph boxes")?;
                    paint_current_path(context, state)?;
                }
            }
            Ok(())
        })();
        let restored = cairo(context.restore(), "restore glyph batch state");
        combine_results(drawn, restored, "restore glyph batch state")?;
    }
    Ok(())
}

fn draw_path_unmasked(context: &Context, state: &PaintState, path: &PathData) -> Result<()> {
    append_path(context, path)?;
    context.set_fill_rule(cairo_fill_rule(state.fill_rule));
    apply_stroke_state(context, state)?;
    paint_current_path(context, state)?;
    cairo(context.status(), "finish drawing path")
}

fn apply_stroke_state(context: &Context, state: &PaintState) -> Result<()> {
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
    cairo(context.status(), "apply stroke paint state")
}

fn paint_current_path(context: &Context, state: &PaintState) -> Result<()> {
    context.set_fill_rule(cairo_fill_rule(state.fill_rule));
    cairo(context.status(), "apply fill rule")?;
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
    cairo(context.status(), "finish painting current path")
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
    let mut font_options = FontOptions::new().map_err(|source| Error::Backend {
        operation: "create deterministic font options",
        source,
    })?;
    font_options.set_antialias(Antialias::Gray);
    font_options.set_subpixel_order(SubpixelOrder::Default);
    font_options.set_hint_style(HintStyle::None);
    font_options.set_hint_metrics(HintMetrics::Off);
    context.set_font_options(&font_options);
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
