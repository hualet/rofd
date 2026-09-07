use std::fs::File;
use std::path::PathBuf;

use cairo::{Context, Format, ImageSurface};
use image::ImageReader;
use rofd_core::{Document, LayerSource, LoadOptions};
use rofd_render::{
    CairoRenderer, Command, DisplayListBuilder, ImageDecoder, RenderDiagnosticKind, RenderOptions,
    SystemFontResolver,
};

#[test]
fn repository_invoice_fixture_lowers_and_renders_text_images_and_paths() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    let document = Document::open(fixture, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let font_resolver = SystemFontResolver::with_system_fonts(
        vec!["Noto Sans CJK SC".to_owned(), "Noto Sans Mono".to_owned()],
        page.resource_limits().max_font_bytes,
    );
    let image_decoder = ImageDecoder::default();
    let display_list = DisplayListBuilder::new(&font_resolver, &image_decoder)
        .build(&page)
        .unwrap();
    let draw_paths = display_list
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::DrawPath(_)))
        .count();
    assert_eq!(
        (
            page.size().x,
            page.size().y,
            page.size().width,
            page.size().height
        ),
        (0.0, 0.0, 211.5, 140.0)
    );
    assert_eq!(draw_paths, 26);
    let expected_text_ids = [
        39, 40, 44, 45, 46, 47, 51, 53, 55, 57, 59, 60, 61, 65, 66, 68, 70, 72, 74, 76, 77, 78, 82,
        83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
        109, 114, 115, 116,
    ];
    let drawn_text_ids = display_list
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| {
            matches!(
                diagnostic.kind(),
                RenderDiagnosticKind::MissingGlyph { .. }
                    | RenderDiagnosticKind::FontFallback { .. }
            )
            .then_some(diagnostic.object_id())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(drawn_text_ids
        .iter()
        .all(|id| expected_text_ids.contains(id)));
    assert!(display_list
        .diagnostics()
        .iter()
        .all(|diagnostic| diagnostic.source() == LayerSource::Page));
    assert!(display_list
        .diagnostics()
        .iter()
        .all(|diagnostic| !matches!(
            diagnostic.kind(),
            RenderDiagnosticKind::UnsupportedObject { .. }
        )));
    assert_eq!(
        display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::DrawGlyphRun(_)))
            .count(),
        expected_text_ids.len()
    );
    assert!(display_list.commands().iter().all(|command| {
        !matches!(command, Command::DrawGlyphRun(run) if run.glyphs().is_empty())
    }));
    assert_eq!(
        display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::DrawImage { .. }))
            .count(),
        1
    );
    assert!(display_list.commands().iter().any(|command| {
        matches!(command, Command::DrawImage { image, .. } if image.resource_id() == 36)
    }));
    assert!(display_list
        .diagnostics()
        .iter()
        .all(|diagnostic| !matches!(
            diagnostic.kind(),
            RenderDiagnosticKind::MissingGlyph { .. }
                | RenderDiagnosticKind::ImageSubstitutionUnsupported { .. }
                | RenderDiagnosticKind::ImageMaskUnsupported { .. }
        )));

    let options = RenderOptions {
        dpi: 254.0,
        ..RenderOptions::default()
    };
    let (width, height) = CairoRenderer::pixel_size(&page, &options).unwrap();
    let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface).unwrap();
    let report = CairoRenderer
        .render_page_with_services(&page, &context, &options, &font_resolver, &image_decoder)
        .unwrap();
    assert_eq!(report.diagnostics(), display_list.diagnostics());
    drop(context);
    assert_eq!((width, height), (2115, 1400));
    assert_eq!(pixel(&mut surface, 10, 10), [255, 255, 255, 255]);
    assert!(non_white_pixels(&mut surface) > 10_000);
    assert!(
        matching_pixels(&mut surface, (50, 30, 290, 260), |r, g, b| {
            r < 40 && g < 40 && b < 40
        }) > 8_000
    );
    assert!(
        matching_pixels(&mut surface, (600, 50, 1_330, 180), |r, g, b| {
            r > 80 && g < 45 && b < 45
        }) > 4_000
    );
    assert!(
        matching_pixels(&mut surface, (1_300, 820, 1_650, 980), |r, g, b| {
            r < 100 && g < 100 && b < 100
        }) > 100
    );
    assert!(
        matching_pixels(&mut surface, (25, 260, 2_090, 1_170), |r, g, b| {
            r > 70 && g < 50 && b < 50
        }) > 15_000
    );
    let reference = ImageReader::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/reference/invoice-phase3.png"),
    )
    .unwrap()
    .into_dimensions()
    .unwrap();
    assert_eq!(reference, (2115, 1400));
    eprintln!(
        "fixture: 211.5x140 mm, {draw_paths} paths, {} diagnostics; Cairo rendered",
        display_list.diagnostics().len()
    );
}

#[test]
#[ignore = "explicit reference update command"]
fn update_invoice_phase3_reference() {
    assert_eq!(
        std::env::var("ROFD_UPDATE_REFERENCES").as_deref(),
        Ok("1"),
        "set ROFD_UPDATE_REFERENCES=1 to rewrite the reviewed reference"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let document = Document::open(root.join("learning/test.ofd"), LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let resolver = SystemFontResolver::with_system_fonts(
        vec!["Noto Sans CJK SC".to_owned(), "Noto Sans Mono".to_owned()],
        page.resource_limits().max_font_bytes,
    );
    let decoder = ImageDecoder::default();
    let options = RenderOptions {
        dpi: 254.0,
        ..RenderOptions::default()
    };
    let (width, height) = CairoRenderer::pixel_size(&page, &options).unwrap();
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface).unwrap();
    CairoRenderer
        .render_page_with_services(&page, &context, &options, &resolver, &decoder)
        .unwrap();
    drop(context);
    let output = root.join("tests/fixtures/reference/invoice-phase3.png");
    surface
        .write_to_png(&mut File::create(output).unwrap())
        .unwrap();
}

fn non_white_pixels(surface: &mut ImageSurface) -> usize {
    surface.flush();
    let stride = surface.stride() as usize;
    let width = surface.width() as usize;
    let height = surface.height() as usize;
    let data = surface.data().unwrap();
    (0..height)
        .flat_map(|y| (0..width).map(move |x| y * stride + x * 4))
        .filter(|offset| data[*offset..*offset + 4] != [255, 255, 255, 255])
        .count()
}

fn matching_pixels(
    surface: &mut ImageSurface,
    region: (i32, i32, i32, i32),
    predicate: impl Fn(u8, u8, u8) -> bool,
) -> usize {
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().unwrap();
    let mut count = 0;
    for y in region.1..=region.3 {
        for x in region.0..=region.2 {
            let offset = y as usize * stride + x as usize * 4;
            let native = u32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap());
            count += usize::from(predicate(
                ((native >> 16) & 0xff) as u8,
                ((native >> 8) & 0xff) as u8,
                (native & 0xff) as u8,
            ));
        }
    }
    count
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

#[test]
fn package_structure_fixture_decodes_bmp_gif_jpeg_png_and_tiff_images() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ofd_files/6.2.001 正常文件结构.ofd");
    let document = Document::open(fixture, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let font_resolver = SystemFontResolver::with_system_fonts(
        vec!["Noto Sans CJK SC".to_owned(), "Noto Sans Mono".to_owned()],
        page.resource_limits().max_font_bytes,
    );
    let image_decoder = ImageDecoder::default();
    let display_list = DisplayListBuilder::new(&font_resolver, &image_decoder)
        .build(&page)
        .unwrap();
    let images = display_list
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::DrawImage { .. }))
        .count();
    assert_eq!(images, 6);
}
