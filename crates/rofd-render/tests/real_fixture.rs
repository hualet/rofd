use std::path::PathBuf;

use cairo::{Context, Format, ImageSurface};
use rofd_core::{Document, LayerSource, LoadOptions};
use rofd_render::{CairoRenderer, Command, DisplayList, RenderDiagnosticKind, RenderOptions};

#[test]
fn repository_invoice_fixture_lowers_and_renders_text_images_and_paths() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    let document = Document::open(fixture, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let display_list = DisplayList::from_page(&page).unwrap();
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
    assert_eq!(
        display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::DrawImage { .. }))
            .count(),
        1
    );

    let options = RenderOptions {
        dpi: 254.0,
        ..RenderOptions::default()
    };
    let (width, height) = CairoRenderer::pixel_size(&page, &options).unwrap();
    let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface).unwrap();
    let report = CairoRenderer
        .render_page(&page, &context, &options)
        .unwrap();
    assert_eq!(report.diagnostics(), display_list.diagnostics());
    drop(context);
    assert_eq!((width, height), (2115, 1400));
    assert_eq!(pixel(&mut surface, 10, 10), [255, 255, 255, 255]);
    assert!(non_white_pixels(&mut surface) > 10_000);
    eprintln!(
        "fixture: 211.5x140 mm, {draw_paths} paths, {} diagnostics; Cairo rendered",
        display_list.diagnostics().len()
    );
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
