use std::io::{Cursor, Write};

use cairo::{
    Antialias, Context, Format, ImageSurface, LineCap, LineJoin, Matrix, PathSegment, SolidPattern,
};
use rofd_core::{Color, Document, LoadOptions, Rect, UnsupportedObjectKind};
use rofd_render::{CairoRenderer, DisplayCommandKind, Error, RenderOptions};
use zip::{write::SimpleFileOptions, ZipWriter};

const PNG: &[u8] = include_bytes!("fixtures/images/asymmetric-rgba.png");

fn minimal_ofd(page_xml: &str) -> Vec<u8> {
    let entries = [
        (
            "OFD.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">
  <ofd:DocBody>
    <ofd:DocInfo><ofd:DocID>cairo-render-fixture</ofd:DocID></ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
  </ofd:DocBody>
</ofd:OFD>"#,
        ),
        (
            "Doc_0/Document.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea>
    <ofd:DocumentRes>Res.xml</ofd:DocumentRes>
    <ofd:MaxUnitID>100</ofd:MaxUnitID>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="100" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#,
        ),
        ("Doc_0/Pages/Page_0/Content.xml", page_xml),
        (
            "Doc_0/Res.xml",
            r#"<Res><Fonts><Font ID="900" FontName="Fixture"/></Fonts><MultiMedias><MultiMedia ID="901" Type="Image" Format="PNG"><MediaFile>image.png</MediaFile></MultiMedia></MultiMedias></Res>"#,
        ),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer
        .start_file("Doc_0/image.png", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(PNG).unwrap();
    writer.finish().unwrap().into_inner()
}

fn open_page(page_box: &str, content: &str) -> rofd_core::Page {
    let page_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>{page_box}</ofd:PhysicalBox></ofd:Area>
  {content}
</ofd:Page>"#
    );
    Document::from_bytes(minimal_ofd(&page_xml), LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap()
}

fn options_at_one_pixel_per_mm() -> RenderOptions {
    RenderOptions {
        dpi: 25.4,
        ..RenderOptions::default()
    }
}

fn render(page: &rofd_core::Page, options: &RenderOptions) -> ImageSurface {
    let (width, height) = CairoRenderer::pixel_size(page, options).unwrap();
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface).unwrap();
    CairoRenderer.render_page(page, &context, options).unwrap();
    drop(context);
    surface
}

fn pixel(surface: &mut ImageSurface, x: i32, y: i32) -> [u8; 4] {
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().unwrap();
    let offset = y as usize * stride + x as usize * 4;
    let native = u32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap());
    [
        ((native >> 16) & 0xff) as u8,
        ((native >> 8) & 0xff) as u8,
        (native & 0xff) as u8,
        ((native >> 24) & 0xff) as u8,
    ]
}

fn assert_red(pixel: [u8; 4]) {
    assert!(
        pixel[0] > 220 && pixel[1] < 30 && pixel[2] < 30,
        "{pixel:?}"
    );
}

fn assert_white(pixel: [u8; 4]) {
    assert!(
        pixel[0] > 245 && pixel[1] > 245 && pixel[2] > 245,
        "{pixel:?}"
    );
}

fn assert_green(pixel: [u8; 4]) {
    assert!(
        pixel[1] > 220 && pixel[0] < 80 && pixel[2] < 80,
        "{pixel:?}"
    );
}

fn assert_blue(pixel: [u8; 4]) {
    assert!(
        pixel[2] > 220 && pixel[0] < 80 && pixel[1] < 80,
        "{pixel:?}"
    );
}

fn assert_magenta(pixel: [u8; 4]) {
    assert!(
        pixel[0] > 220 && pixel[2] > 220 && pixel[1] < 180,
        "{pixel:?}"
    );
}

fn path_segments(context: &Context) -> Vec<PathSegment> {
    context.copy_path().unwrap().iter().collect()
}

#[test]
fn defaults_and_pixel_size_cover_nonzero_page_origins_and_quarter_turns() {
    let defaults = RenderOptions::default();
    assert_eq!(defaults.dpi, 96.0);
    assert_eq!(defaults.scale, 1.0);
    assert_eq!(defaults.rotation_degrees, 0);
    assert_eq!(
        defaults.background,
        Color {
            red: 255,
            green: 255,
            blue: 255,
            alpha: 255,
        }
    );
    assert_eq!(defaults.clip, None);
    assert_eq!(defaults.max_raster_bytes, 256 * 1024 * 1024);

    let page = open_page("10 20 30 40", "");
    let mut options = options_at_one_pixel_per_mm();
    options.scale = 2.0;
    assert_eq!(
        CairoRenderer::pixel_size(&page, &options).unwrap(),
        (60, 80)
    );
    options.rotation_degrees = 90;
    assert_eq!(
        CairoRenderer::pixel_size(&page, &options).unwrap(),
        (80, 60)
    );
    options.rotation_degrees = 270;
    assert_eq!(
        CairoRenderer::pixel_size(&page, &options).unwrap(),
        (80, 60)
    );
}

#[test]
fn pixel_size_rejects_cairo_dimension_and_worst_case_surface_budget_limits() {
    let cairo_maximum = open_page("0 0 32767 1", "");
    let cairo_too_wide = open_page("0 0 32768 1", "");
    let unlimited = RenderOptions {
        max_raster_bytes: u64::MAX,
        ..options_at_one_pixel_per_mm()
    };
    assert_eq!(
        CairoRenderer::pixel_size(&cairo_maximum, &unlimited).unwrap(),
        (32_767, 1)
    );
    assert!(matches!(
        CairoRenderer::pixel_size(&cairo_too_wide, &unlimited),
        Err(Error::InvalidSurfaceSize { .. })
    ));

    let huge_allocation = open_page("0 0 20000 20000", "");
    assert!(matches!(
        CairoRenderer::pixel_size(&huge_allocation, &options_at_one_pixel_per_mm()),
        Err(Error::RasterBudgetExceeded {
            required_bytes: 3_600_000_000,
            max_bytes: 268_435_456,
        })
    ));

    let boundary = open_page("0 0 100 100", "");
    let exact = RenderOptions {
        max_raster_bytes: 90_000,
        ..options_at_one_pixel_per_mm()
    };
    assert_eq!(
        CairoRenderer::pixel_size(&boundary, &exact).unwrap(),
        (100, 100)
    );
    let one_under = RenderOptions {
        max_raster_bytes: 89_999,
        ..exact
    };
    assert!(matches!(
        CairoRenderer::pixel_size(&boundary, &one_under),
        Err(Error::RasterBudgetExceeded {
            required_bytes: 90_000,
            max_bytes: 89_999,
        })
    ));

    let padded_mask_stride = open_page("0 0 1 100", "");
    let misses_a8_padding = RenderOptions {
        max_raster_bytes: 1_199,
        ..options_at_one_pixel_per_mm()
    };
    assert!(matches!(
        CairoRenderer::pixel_size(&padded_mask_stride, &misses_a8_padding),
        Err(Error::RasterBudgetExceeded {
            required_bytes: 1_200,
            max_bytes: 1_199,
        })
    ));
}

#[test]
fn invalid_options_and_unsafe_surface_dimensions_are_structured_errors() {
    let page = open_page("0 0 20 20", "");
    for (field, options) in [
        (
            "dpi",
            RenderOptions {
                dpi: 0.0,
                ..RenderOptions::default()
            },
        ),
        (
            "scale",
            RenderOptions {
                scale: f64::NAN,
                ..RenderOptions::default()
            },
        ),
        (
            "rotation_degrees",
            RenderOptions {
                rotation_degrees: 45,
                ..RenderOptions::default()
            },
        ),
        (
            "clip",
            RenderOptions {
                clip: Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 1.0,
                }),
                ..RenderOptions::default()
            },
        ),
    ] {
        assert!(matches!(
            CairoRenderer::pixel_size(&page, &options),
            Err(Error::InvalidOption { field: actual, .. }) if actual == field
        ));
    }

    let huge = RenderOptions {
        dpi: f64::MAX,
        ..RenderOptions::default()
    };
    assert!(matches!(
        CairoRenderer::pixel_size(&page, &huge),
        Err(Error::InvalidSurfaceSize { .. })
    ));
}

#[test]
fn rejects_an_image_surface_smaller_than_pixel_size_without_changing_state() {
    let page = open_page("0 0 20 20", "");
    let surface = ImageSurface::create(Format::ARgb32, 19, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    context.set_line_width(7.0);

    assert!(matches!(
        CairoRenderer.render_page(&page, &context, &options_at_one_pixel_per_mm()),
        Err(Error::SurfaceTooSmall {
            required_width: 20,
            actual_width: 19,
            ..
        })
    ));
    assert_eq!(context.line_width(), 7.0);
    assert!(matches!(
        context.restore(),
        Err(cairo::Error::InvalidRestore)
    ));
}

fn surface_finished_backend_errors(error: &Error) -> usize {
    match error {
        Error::Backend {
            source: cairo::Error::SurfaceFinished,
            ..
        } => 1,
        Error::Cleanup {
            primary, cleanup, ..
        } => surface_finished_backend_errors(primary) + surface_finished_backend_errors(cleanup),
        _ => 0,
    }
}

#[test]
fn simultaneous_backend_and_cleanup_failures_are_both_reported() {
    let page = open_page("0 0 20 20", "");
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    surface.finish();

    let error = CairoRenderer
        .render_page(&page, &context, &options_at_one_pixel_per_mm())
        .unwrap_err();
    assert!(matches!(error, Error::Cleanup { .. }));
    assert!(surface_finished_backend_errors(&error) >= 2, "{error:?}");
}

#[test]
fn renders_fill_stroke_background_and_optional_page_clip() {
    let page = open_page(
        "10 20 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="12 22 10 10" Fill="true" Stroke="true" LineWidth="2">
  <ofd:FillColor Value="255 0 0"/><ofd:StrokeColor Value="0 0 0"/>
  <ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let mut options = options_at_one_pixel_per_mm();
    options.clip = Some(Rect {
        x: 10.0,
        y: 20.0,
        width: 7.0,
        height: 20.0,
    });
    let mut surface = render(&page, &options);

    assert_white(pixel(&mut surface, 0, 0));
    assert_eq!(pixel(&mut surface, 2, 7)[0..3], [0, 0, 0]);
    assert_red(pixel(&mut surface, 5, 7));
    assert_white(pixel(&mut surface, 9, 7));
}

#[test]
fn clip_areas_are_true_union_and_separate_clips_intersect() {
    for rule in ["NonZero", "Even-Odd"] {
        let content = r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 10" Fill="true" Stroke="false">
  <ofd:Clips>
    <ofd:Clip>
      <ofd:Area><ofd:Path Boundary="1 1 8 8" Fill="true" Stroke="false" Rule="{rule}"><ofd:AbbreviatedData>M 0 0 L 8 0 L 8 8 L 0 8 C</ofd:AbbreviatedData></ofd:Path></ofd:Area>
      <ofd:Area><ofd:Path Boundary="5 1 8 8" Fill="true" Stroke="false" Rule="{rule}"><ofd:AbbreviatedData>M 0 0 L 0 8 L 8 8 L 8 0 C</ofd:AbbreviatedData></ofd:Path></ofd:Area>
    </ofd:Clip>
    <ofd:Clip><ofd:Area><ofd:Path Boundary="3 0 8 10" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 8 0 L 8 10 L 0 10 C</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip>
  </ofd:Clips>
  <ofd:FillColor Value="255 0 0"/>
  <ofd:AbbreviatedData>M 0 0 L 20 0 L 20 10 L 0 10 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#
            .replace("{rule}", rule);
        let page = open_page("0 0 20 10", &content);
        let mut surface = render(&page, &options_at_one_pixel_per_mm());

        assert_white(pixel(&mut surface, 2, 5));
        assert_red(pixel(&mut surface, 4, 5));
        assert_red(pixel(&mut surface, 7, 5));
        assert_red(pixel(&mut surface, 10, 5));
        assert_white(pixel(&mut surface, 12, 5));
    }
}

#[test]
fn quadratics_and_zero_radius_arcs_reach_their_endpoints() {
    let page = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true" LineWidth="2">
  <ofd:StrokeColor Value="255 0 0"/>
  <ofd:AbbreviatedData>M 2 10 Q 6 2 10 10 A 0 5 0 0 1 18 10</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let mut surface = render(&page, &options_at_one_pixel_per_mm());

    assert_red(pixel(&mut surface, 6, 6));
    assert_red(pixel(&mut surface, 14, 10));
}

#[test]
fn elliptical_arcs_cover_sweep_large_and_rotated_branches() {
    let page = open_page(
        "0 0 50 40",
        r#"<ofd:Content><ofd:Layer ID="1">
<ofd:PathObject ID="2" Boundary="0 0 50 40" Fill="false" Stroke="true" LineWidth="1"><ofd:StrokeColor Value="255 0 0"/><ofd:AbbreviatedData>M 5 10 A 8 5 0 0 1 19 10</ofd:AbbreviatedData></ofd:PathObject>
<ofd:PathObject ID="3" Boundary="0 0 50 40" Fill="false" Stroke="true" LineWidth="1"><ofd:StrokeColor Value="0 255 0"/><ofd:AbbreviatedData>M 25 10 A 8 5 0 1 0 39 10</ofd:AbbreviatedData></ofd:PathObject>
<ofd:PathObject ID="4" Boundary="0 0 50 40" Fill="false" Stroke="true" LineWidth="1"><ofd:StrokeColor Value="0 0 255"/><ofd:AbbreviatedData>M 8 30 A 10 4 45 0 1 24 30</ofd:AbbreviatedData></ofd:PathObject>
<ofd:PathObject ID="5" Boundary="0 0 50 40" Fill="false" Stroke="true" LineWidth="1"><ofd:StrokeColor Value="255 0 255"/><ofd:AbbreviatedData>M 8 30 A 10 4 -45 0 1 24 30</ofd:AbbreviatedData></ofd:PathObject>
</ofd:Layer></ofd:Content>"#,
    );
    let mut surface = render(&page, &options_at_one_pixel_per_mm());

    assert_red(pixel(&mut surface, 12, 7));
    assert_white(pixel(&mut surface, 12, 13));
    assert_green(pixel(&mut surface, 31, 17));
    assert_white(pixel(&mut surface, 32, 7));
    assert_blue(pixel(&mut surface, 8, 18));
    assert_magenta(pixel(&mut surface, 19, 20));
    assert_white(pixel(&mut surface, 18, 35));
}

#[test]
fn rejects_arc_geometry_that_overflows_during_endpoint_conversion() {
    let page = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true">
  <ofd:AbbreviatedData>M 1 1 A 1e308 1e308 0 0 1 10 10</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();

    assert!(matches!(
        CairoRenderer.render_page(
            &page,
            &context,
            &options_at_one_pixel_per_mm()
        ),
        Err(Error::InvalidGeometry {
            primitive: "arc",
            field,
            ..
        }) if field == "center denominator"
    ));
    assert!(matches!(
        context.restore(),
        Err(cairo::Error::InvalidRestore)
    ));
}

#[test]
fn rotation_background_and_report_diagnostics_are_applied() {
    let page = open_page(
        "10 20 20 10",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CompositeObject ID="2"/></ofd:Layer></ofd:Content>"#,
    );
    let options = RenderOptions {
        dpi: 25.4,
        rotation_degrees: 90,
        background: Color {
            red: 10,
            green: 20,
            blue: 30,
            alpha: 255,
        },
        ..RenderOptions::default()
    };
    let (width, height) = CairoRenderer::pixel_size(&page, &options).unwrap();
    assert_eq!((width, height), (10, 20));
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface).unwrap();
    let report = CairoRenderer
        .render_page(&page, &context, &options)
        .unwrap();
    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(
        report.diagnostics()[0].unsupported_kind(),
        Some(UnsupportedObjectKind::Composite)
    );
    drop(context);
    let mut surface = surface;
    assert_eq!(pixel(&mut surface, 5, 10)[0..3], [10, 20, 30]);
}

#[test]
fn deferred_image_command_fails_before_cairo_paints() {
    let content = r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="3" Boundary="0 0 2 2" Fill="true"><ofd:AbbreviatedData>M 0 0 L 2 0 L 2 2 C</ofd:AbbreviatedData></ofd:PathObject><ofd:ImageObject ID="2" Boundary="0 0 3 2" ResourceID="901"/></ofd:Layer></ofd:Content>"#;
    let page = open_page("0 0 20 20", content);
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    let matrix = Matrix::new(2.0, 0.25, 0.5, 3.0, 4.0, 5.0);
    context.set_matrix(matrix);
    context.set_line_width(7.0);
    context.move_to(4.0, 5.0);
    context.line_to(6.0, 7.0);
    let caller_path = path_segments(&context);
    assert!(matches!(
        CairoRenderer.render_page(&page, &context, &options_at_one_pixel_per_mm()),
        Err(Error::UnsupportedDisplayCommand {
            command: DisplayCommandKind::Image
        })
    ));
    assert_eq!(context.matrix(), matrix);
    assert_eq!(context.line_width(), 7.0);
    assert_eq!(path_segments(&context), caller_path);
    drop(context);
    let mut surface = surface;
    assert_eq!(pixel(&mut surface, 10, 10), [0, 0, 0, 0]);
}

#[test]
fn quarter_turns_map_content_from_a_nonzero_page_origin() {
    let page = open_page(
        "10 20 10 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="10 20 4 2" Fill="true" Stroke="false">
  <ofd:FillColor Value="255 0 0"/><ofd:AbbreviatedData>M 0 0 L 4 0 L 4 2 L 0 2 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    for (rotation_degrees, red_at, white_at) in [(90, (19, 2), (1, 2)), (270, (1, 8), (18, 8))] {
        let options = RenderOptions {
            dpi: 25.4,
            rotation_degrees,
            ..RenderOptions::default()
        };
        let mut surface = render(&page, &options);
        assert_red(pixel(&mut surface, red_at.0, red_at.1));
        assert_white(pixel(&mut surface, white_at.0, white_at.1));
    }
}

#[test]
fn half_turn_maps_content_from_a_nonzero_page_origin() {
    let page = open_page(
        "10 20 10 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="10 20 4 2" Fill="true" Stroke="false">
  <ofd:FillColor Value="255 0 0"/><ofd:AbbreviatedData>M 0 0 L 4 0 L 4 2 L 0 2 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let options = RenderOptions {
        dpi: 25.4,
        rotation_degrees: 180,
        ..RenderOptions::default()
    };
    let mut surface = render(&page, &options);
    assert_red(pixel(&mut surface, 8, 18));
    assert_white(pixel(&mut surface, 2, 1));
}

#[test]
fn caller_context_state_is_restored_on_success_and_option_failure() {
    let page = open_page("0 0 20 20", "");
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    let matrix = Matrix::new(2.0, 0.25, 0.5, 3.0, 4.0, 5.0);
    context.set_matrix(matrix);
    context.set_line_width(7.0);
    context.set_source_rgba(0.2, 0.3, 0.4, 0.5);

    CairoRenderer
        .render_page(&page, &context, &options_at_one_pixel_per_mm())
        .unwrap();
    assert_eq!(context.matrix(), matrix);
    assert_eq!(context.line_width(), 7.0);
    assert_eq!(
        SolidPattern::try_from(context.source())
            .unwrap()
            .rgba()
            .unwrap(),
        (0.2, 0.3, 0.4, 0.5)
    );

    let invalid = RenderOptions {
        scale: 0.0,
        ..RenderOptions::default()
    };
    assert!(CairoRenderer
        .render_page(&page, &context, &invalid)
        .is_err());
    assert_eq!(context.matrix(), matrix);
    assert_eq!(context.line_width(), 7.0);
    assert!(matches!(
        context.restore(),
        Err(cairo::Error::InvalidRestore)
    ));
}

#[test]
fn caller_operator_does_not_change_renderer_output_and_is_restored() {
    let page = open_page(
        "0 0 10 10",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true" Stroke="false">
  <ofd:FillColor Value="255 0 0"/><ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let surface = ImageSurface::create(Format::ARgb32, 10, 10).unwrap();
    let context = Context::new(&surface).unwrap();
    context.set_operator(cairo::Operator::Clear);

    CairoRenderer
        .render_page(&page, &context, &options_at_one_pixel_per_mm())
        .unwrap();
    assert_eq!(context.operator(), cairo::Operator::Clear);
    drop(context);
    let mut surface = surface;
    assert_red(pixel(&mut surface, 5, 5));
}

#[test]
fn caller_stroke_parameters_do_not_change_output_and_are_restored() {
    let page = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true" LineWidth="2">
  <ofd:AbbreviatedData>M 2 16 L 10 2 L 18 16</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let baseline = render(&page, &options_at_one_pixel_per_mm());
    let contaminated = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&contaminated).unwrap();
    context.set_dash(&[1.0, 3.0], 0.75);
    context.set_line_cap(LineCap::Round);
    context.set_line_join(LineJoin::Bevel);
    context.set_miter_limit(1.5);

    CairoRenderer
        .render_page(&page, &context, &options_at_one_pixel_per_mm())
        .unwrap();
    assert_eq!(context.dash(), (vec![1.0, 3.0], 0.75));
    assert_eq!(context.line_cap(), LineCap::Round);
    assert_eq!(context.line_join(), LineJoin::Bevel);
    assert_eq!(context.miter_limit(), 1.5);
    drop(context);

    baseline.flush();
    contaminated.flush();
    let mut baseline_bytes = Vec::new();
    baseline
        .with_data(|data| baseline_bytes.extend_from_slice(data))
        .unwrap();
    let mut contaminated_bytes = Vec::new();
    contaminated
        .with_data(|data| contaminated_bytes.extend_from_slice(data))
        .unwrap();
    assert_eq!(baseline_bytes, contaminated_bytes);
}

#[test]
fn caller_raster_parameters_do_not_change_output_and_are_restored() {
    let page = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true" LineWidth="1">
  <ofd:AbbreviatedData>M 2.25 17.5 B 5.5 1.25 14.5 18.75 18 2.5</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let baseline = render(&page, &options_at_one_pixel_per_mm());
    let contaminated = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&contaminated).unwrap();
    context.set_antialias(Antialias::None);
    context.set_tolerance(8.0);

    CairoRenderer
        .render_page(&page, &context, &options_at_one_pixel_per_mm())
        .unwrap();
    assert_eq!(context.antialias(), Antialias::None);
    assert_eq!(context.tolerance(), 8.0);
    drop(context);

    baseline.flush();
    contaminated.flush();
    assert_eq!(surface_bytes(&baseline), surface_bytes(&contaminated));
}

#[test]
fn nonrestrictive_clip_does_not_change_path_rasterization() {
    let unclipped = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true" LineWidth="1">
  <ofd:AbbreviatedData>M 2.25 17.5 B 5.5 1.25 14.5 18.75 18 2.5</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let clipped = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true" LineWidth="1">
  <ofd:Clips><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 20 20" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 20 0 L 20 20 L 0 20 C</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips>
  <ofd:AbbreviatedData>M 2.25 17.5 B 5.5 1.25 14.5 18.75 18 2.5</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );
    let baseline = render(&unclipped, &options_at_one_pixel_per_mm());
    let masked = render(&clipped, &options_at_one_pixel_per_mm());
    baseline.flush();
    masked.flush();
    assert_eq!(surface_bytes(&baseline), surface_bytes(&masked));
}

fn surface_bytes(surface: &ImageSurface) -> Vec<u8> {
    let mut bytes = Vec::new();
    surface
        .with_data(|data| bytes.extend_from_slice(data))
        .unwrap();
    bytes
}

#[test]
fn caller_path_is_preserved_on_success_and_post_save_failure() {
    let success_page = open_page("0 0 20 20", "");
    let failure_page = open_page(
        "0 0 20 20",
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 20 20" Fill="false" Stroke="true">
  <ofd:AbbreviatedData>M 1 1 A 1e308 1e308 0 0 1 10 10</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );

    for (page, succeeds) in [(&success_page, true), (&failure_page, false)] {
        let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
        let context = Context::new(&surface).unwrap();
        context.move_to(2.0, 3.0);
        context.line_to(7.0, 11.0);
        context.curve_to(8.0, 12.0, 9.0, 13.0, 14.0, 15.0);
        let expected = path_segments(&context);
        let expected_point = context.current_point().unwrap();
        let expected_extents = context.path_extents().unwrap();

        assert_eq!(
            CairoRenderer
                .render_page(page, &context, &options_at_one_pixel_per_mm())
                .is_ok(),
            succeeds
        );
        assert_eq!(path_segments(&context), expected);
        assert_eq!(context.current_point().unwrap(), expected_point);
        assert_eq!(context.path_extents().unwrap(), expected_extents);
    }
}
