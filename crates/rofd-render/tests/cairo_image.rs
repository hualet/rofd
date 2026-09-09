use std::io::{Cursor, Write};

use cairo::{
    Antialias, Context, FillRule, FontOptions, Format, HintMetrics, HintStyle, ImageSurface,
    Matrix, Operator, PathSegment, SolidPattern, SubpixelOrder,
};
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use rofd_core::{Document, LoadOptions, Rect};
use rofd_render::{CairoRenderer, Error, ImageInterpolation, RenderOptions};
use zip::{write::SimpleFileOptions, ZipWriter};

const PNG: &[u8] = include_bytes!("fixtures/images/asymmetric-rgba.png");

fn package(content: &str, image: &[u8]) -> Vec<u8> {
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1">{content}</ofd:Layer></ofd:Content></ofd:Page>"#,
    );
    let files = [
        (
            "OFD.xml",
            br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>cairo-image</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#.as_slice(),
        ),
        (
            "Doc_0/Document.xml",
            br#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes></ofd:CommonData><ofd:Pages><ofd:Page ID="1" BaseLoc="Page.xml"/></ofd:Pages></ofd:Document>"#.as_slice(),
        ),
        (
            "Doc_0/Res.xml",
            br#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:MultiMedias><ofd:MultiMedia ID="10" Type="Image" Format="PNG"><ofd:MediaFile>image.png</ofd:MediaFile></ofd:MultiMedia><ofd:MultiMedia ID="11" Type="Image" Format="PNG"><ofd:MediaFile>image.png</ofd:MediaFile></ofd:MultiMedia></ofd:MultiMedias></ofd:Res>"#.as_slice(),
        ),
        ("Doc_0/Page.xml", page.as_bytes()),
        ("Doc_0/image.png", image),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in files {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn page(content: &str) -> rofd_core::Page {
    Document::from_bytes(package(content, PNG), LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap()
}

fn options(interpolation: ImageInterpolation, rotation_degrees: u16) -> RenderOptions {
    RenderOptions {
        dpi: 25.4,
        rotation_degrees,
        image_interpolation: interpolation,
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

fn path(context: &Context) -> Vec<PathSegment> {
    context.copy_path().unwrap().iter().collect()
}

fn wide_png() -> Vec<u8> {
    let pixels = vec![255u8; 32_768 * 4];
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(&pixels, 32_768, 1, ExtendedColorType::Rgba8)
        .unwrap();
    encoded
}

#[test]
fn defaults_to_bilinear_and_nearest_preserves_orientation_boundary_and_alpha() {
    assert_eq!(
        RenderOptions::default().image_interpolation,
        ImageInterpolation::Bilinear
    );
    let page = page(
        r#"<ofd:ImageObject ID="2" Boundary="4 5 6 4" CTM="6 0 0 4 0 0" ResourceID="10" Alpha="128"/>"#,
    );
    let mut surface = render(&page, &options(ImageInterpolation::Nearest, 0));

    assert_eq!(pixel(&mut surface, 4, 5), [255, 127, 127, 255]);
    assert_eq!(pixel(&mut surface, 6, 5), [191, 255, 191, 255]);
    assert_eq!(pixel(&mut surface, 8, 5), [127, 127, 255, 255]);
    assert_eq!(pixel(&mut surface, 4, 8), [255, 255, 127, 255]);
    assert_eq!(pixel(&mut surface, 6, 8), [255, 127, 255, 255]);
    assert_eq!(pixel(&mut surface, 8, 8), [127, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 3, 5), [255, 255, 255, 255]);
}

#[test]
fn object_transform_ofd_clip_and_page_clip_compose() {
    let page = page(
        r#"<ofd:ImageObject ID="2" Boundary="2 3 6 4" CTM="6 0 0 4 3 2" ResourceID="10"><ofd:Clips TransFlag="true"><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 0.5 1" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 0.5 0 L 0.5 1 L 0 1 C</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips></ofd:ImageObject>"#,
    );
    let mut options = options(ImageInterpolation::Nearest, 0);
    options.clip = Some(Rect::parse("0 0 7 20").unwrap());
    let mut surface = render(&page, &options);

    assert_eq!(pixel(&mut surface, 5, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut surface, 6, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut surface, 7, 5), [255, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 8, 5), [255, 255, 255, 255]);
}

#[test]
fn ctm_scales_the_normalized_image_without_multiplying_boundary_dimensions() {
    let page =
        page(r#"<ofd:ImageObject ID="2" Boundary="4 5 6 4" CTM="6 0 0 4 0 0" ResourceID="10"/>"#);
    let mut surface = render(&page, &options(ImageInterpolation::Nearest, 0));

    assert_eq!(pixel(&mut surface, 4, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut surface, 9, 8), [0, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 10, 8), [255, 255, 255, 255]);
}

#[test]
fn page_quarter_turns_rotate_asymmetric_pixels() {
    let page =
        page(r#"<ofd:ImageObject ID="2" Boundary="4 5 6 4" CTM="6 0 0 4 0 0" ResourceID="10"/>"#);
    let samples = [(0, (4, 5)), (90, (14, 4)), (180, (15, 14)), (270, (5, 15))];
    for (rotation, (x, y)) in samples {
        let mut surface = render(&page, &options(ImageInterpolation::Nearest, rotation));
        assert_eq!(pixel(&mut surface, x, y), [255, 0, 0, 255]);
    }
}

#[test]
fn bilinear_blends_interior_samples_and_pads_image_edges() {
    let page =
        page(r#"<ofd:ImageObject ID="2" Boundary="4 5 9 6" CTM="9 0 0 6 0 0" ResourceID="10"/>"#);
    let mut nearest_surface = render(&page, &options(ImageInterpolation::Nearest, 0));
    let mut bilinear_surface = render(&page, &options(ImageInterpolation::Bilinear, 0));
    let nearest = pixel(&mut nearest_surface, 6, 6);
    let bilinear = pixel(&mut bilinear_surface, 6, 6);
    assert_eq!(nearest, [255, 0, 0, 255]);
    assert!(
        (205..=220).contains(&bilinear[0])
            && (75..=95).contains(&bilinear[1])
            && (35..=50).contains(&bilinear[2])
            && bilinear[3] == 255,
        "unexpected bilinear sample {bilinear:?}"
    );
    assert_eq!(pixel(&mut bilinear_surface, 4, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut bilinear_surface, 12, 10), [0, 255, 255, 255]);
}

#[test]
fn image_extensions_render_silently_when_mask_matches() {
    // Both ResourceID 10 and the ImageMask/Substitution resource 11 point to
    // the same PNG, so the mask dimensions match and the image renders without
    // diagnostics.
    let page = page(
        r#"<ofd:ImageObject ID="2" Boundary="4 5 6 4" ResourceID="10" Substitution="11" ImageMask="11"/>"#,
    );
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    let report = CairoRenderer
        .render_page(&page, &context, &options(ImageInterpolation::Nearest, 0))
        .unwrap();
    assert!(
        report.diagnostics().is_empty(),
        "expected no diagnostics, got {:?}",
        report.diagnostics()
    );
}

#[test]
fn decode_error_preserves_caller_state_path_and_pixels() {
    let page = Document::from_bytes(
        package(
            r#"<ofd:ImageObject ID="2" Boundary="4 5 6 4" ResourceID="10"/>"#,
            b"\x89PNG\r\n\x1a\ntruncated",
        ),
        LoadOptions::default(),
    )
    .unwrap()
    .page(0)
    .unwrap();
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    let matrix = Matrix::new(2.0, 0.25, 0.5, 3.0, 4.0, 5.0);
    context.set_matrix(matrix);
    context.set_operator(Operator::Xor);
    context.move_to(1.0, 2.0);
    context.line_to(3.0, 4.0);
    let caller_path = path(&context);

    assert!(matches!(
        CairoRenderer.render_page(&page, &context, &options(ImageInterpolation::Nearest, 0)),
        Err(Error::ObjectResourceProcessing { .. })
    ));
    assert_eq!(context.matrix(), matrix);
    assert_eq!(context.operator(), Operator::Xor);
    assert_eq!(path(&context), caller_path);
    drop(context);
    let mut surface = surface;
    assert_eq!(pixel(&mut surface, 10, 10), [0, 0, 0, 0]);
}

#[test]
fn native_image_buffer_counts_toward_the_raster_working_set() {
    let page =
        page(r#"<ofd:ImageObject ID="2" Boundary="4 5 6 4" CTM="6 0 0 4 0 0" ResourceID="10"/>"#);
    let exact = RenderOptions {
        max_raster_bytes: 3_624,
        ..options(ImageInterpolation::Nearest, 0)
    };
    let mut surface = render(&page, &exact);
    assert_eq!(pixel(&mut surface, 4, 5), [255, 0, 0, 255]);

    let one_under = RenderOptions {
        max_raster_bytes: 3_623,
        ..exact
    };
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    assert!(matches!(
        CairoRenderer.render_page(&page, &context, &one_under),
        Err(Error::RasterBudgetExceeded {
            required_bytes: 3_624,
            max_bytes: 3_623,
        })
    ));
}

#[test]
fn repeated_resources_share_one_native_buffer_but_distinct_resources_are_aggregated() {
    let repeated = page(
        r#"<ofd:ImageObject ID="2" Boundary="0 0 3 2" CTM="3 0 0 2 0 0" ResourceID="10"/><ofd:ImageObject ID="3" Boundary="3 0 3 2" CTM="3 0 0 2 0 0" ResourceID="10"/>"#,
    );
    let exact_one_buffer = RenderOptions {
        max_raster_bytes: 3_624,
        ..options(ImageInterpolation::Nearest, 0)
    };
    let mut surface = render(&repeated, &exact_one_buffer);
    assert_eq!(pixel(&mut surface, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut surface, 3, 0), [255, 0, 0, 255]);

    let distinct = page(
        r#"<ofd:ImageObject ID="2" Boundary="0 0 3 2" ResourceID="10"/><ofd:ImageObject ID="3" Boundary="3 0 3 2" ResourceID="11"/>"#,
    );
    let one_under_two_buffers = RenderOptions {
        max_raster_bytes: 3_647,
        ..options(ImageInterpolation::Nearest, 0)
    };
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    assert!(matches!(
        CairoRenderer.render_page(&distinct, &context, &one_under_two_buffers),
        Err(Error::RasterBudgetExceeded {
            required_bytes: 3_648,
            max_bytes: 3_647,
        })
    ));
}

#[test]
fn oversized_source_is_rejected_before_painting_and_preserves_caller_state() {
    let oversized = wide_png();
    let page = Document::from_bytes(
        package(
            r#"<ofd:PathObject ID="2" Boundary="0 0 4 4" Fill="true"><ofd:FillColor Value="255 0 0"/><ofd:AbbreviatedData>M 0 0 L 4 0 L 4 4 L 0 4 C</ofd:AbbreviatedData></ofd:PathObject><ofd:ImageObject ID="3" Boundary="4 5 6 4" ResourceID="10"/>"#,
            &oversized,
        ),
        LoadOptions::default(),
    )
    .unwrap()
    .page(0)
    .unwrap();
    let surface = ImageSurface::create(Format::ARgb32, 20, 20).unwrap();
    let context = Context::new(&surface).unwrap();
    let matrix = Matrix::new(2.0, 0.25, 0.5, 3.0, 4.0, 5.0);
    let font_matrix = Matrix::new(7.0, 0.0, 0.0, 9.0, 0.0, 0.0);
    let mut font_options = FontOptions::new().unwrap();
    font_options.set_antialias(Antialias::Subpixel);
    font_options.set_subpixel_order(SubpixelOrder::Bgr);
    font_options.set_hint_style(HintStyle::Full);
    font_options.set_hint_metrics(HintMetrics::On);
    context.set_matrix(matrix);
    context.set_font_matrix(font_matrix);
    context.set_font_options(&font_options);
    context.set_operator(Operator::Xor);
    context.set_fill_rule(FillRule::EvenOdd);
    context.set_antialias(Antialias::Gray);
    context.set_tolerance(3.0);
    context.set_line_width(7.0);
    context.set_source_rgba(0.2, 0.3, 0.4, 0.5);
    context.rectangle(0.0, 0.0, 9.0, 8.0);
    context.clip();
    let clip = context.clip_extents().unwrap();
    context.move_to(1.0, 2.0);
    context.line_to(3.0, 4.0);
    let caller_path = path(&context);

    assert!(matches!(
        CairoRenderer.render_page(&page, &context, &options(ImageInterpolation::Nearest, 0)),
        Err(Error::InvalidImageSurfaceSize {
            resource_id: 10,
            width: 32_768,
            height: 1,
        })
    ));
    assert_eq!(context.matrix(), matrix);
    assert_eq!(context.font_matrix(), font_matrix);
    assert_eq!(context.font_options().unwrap(), font_options);
    assert_eq!(context.operator(), Operator::Xor);
    assert_eq!(context.fill_rule(), FillRule::EvenOdd);
    assert_eq!(context.antialias(), Antialias::Gray);
    assert_eq!(context.tolerance(), 3.0);
    assert_eq!(context.line_width(), 7.0);
    assert_eq!(context.clip_extents().unwrap(), clip);
    assert_eq!(path(&context), caller_path);
    assert_eq!(
        SolidPattern::try_from(context.source())
            .unwrap()
            .rgba()
            .unwrap(),
        (0.2, 0.3, 0.4, 0.5)
    );
    drop(context);
    let mut surface = surface;
    assert_eq!(pixel(&mut surface, 1, 1), [0, 0, 0, 0]);
}
