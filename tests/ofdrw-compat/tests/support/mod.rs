//! Shared helpers for the ofdrw-migrated compatibility tests.
//!
//! This module is compiled into every test binary of this crate, and each
//! binary uses a different subset of the helpers.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use cairo::{Context, Format, ImageSurface};
use image::ImageReader;
use rofd_core::{Document, LoadOptions, Page, PageObject};
use rofd_render::{CairoRenderer, ImageDecoder, RenderOptions, SystemFontResolver};

/// Fallback families matching `crates/rofd-render/tests/real_fixture.rs`; the CI
/// container ships Noto CJK fonts.
const FALLBACK_FAMILIES: [&str; 2] = ["Noto Sans CJK SC", "Noto Sans Mono"];

/// DPI used by ofdrw's `OFD2IMGTest` (`3.78 ppm`) and by the generated references.
pub const REFERENCE_DPI: f64 = 96.0;

/// Per-channel tolerance for pixel comparisons against the ofdrw-rendered
/// references; absorbs Java2D vs cairo antialiasing differences.
pub const CHANNEL_TOLERANCE: u8 = 32;

/// Default upper bound for the fraction of mismatched pixels in a page
/// comparison; specific fixtures override this in [`max_mismatch_fraction`].
pub const DEFAULT_MAX_MISMATCH_FRACTION: f64 = 0.10;

/// Returns the directory holding the fixtures copied from ofdrw.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Returns the directory holding the reference PNGs rendered by ofdrw.
pub fn references_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references")
}

/// Returns the fixture path for a path relative to `fixtures/`, e.g.
/// `fixture("reader/helloworld.ofd")`.
pub fn fixture(relative: &str) -> PathBuf {
    fixtures_dir().join(relative)
}

/// Opens a fixture with the default (lenient) load options.
pub fn open_document(path: &Path) -> Document {
    Document::open(path, LoadOptions::default())
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()))
}

/// Fixtures that rofd-core currently cannot fully load, with the observed
/// failure. These are genuine compatibility gaps versus ofdrw, tracked in
/// README.md's known-differences section. The smoke test in
/// `package_edge.rs` fails when an unlisted fixture breaks or a listed one
/// starts passing (remove the entry once the parser handles it).
pub const KNOWN_LOAD_FAILURES: &[(&str, &str)] = &[
    (
        "reader/path_unstd.ofd",
        "Document.xml with duplicate TemplatePage elements rejected",
    ),
    (
        "reader/发票示例.ofd",
        "template Content.xml declares duplicate object ID 15",
    ),
    ("converter/1.ofd", "GBIG2 image format unsupported"),
    (
        "converter/20240531141733.ofd",
        "template Content.xml declares duplicate object ID 15",
    ),
    (
        "converter/999.ofd",
        "page 2 object 135: negative DeltaY rejected",
    ),
    (
        "converter/ano.ofd",
        "PublicRes.xml with duplicate Fonts elements rejected",
    ),
    (
        "converter/draw_param_ref.ofd",
        "template object 239: DeltaX with more than 4 displacements rejected",
    ),
    (
        "converter/intro-数科.ofd",
        "page 5 object 187: FillColor element without a value rejected",
    ),
    (
        "converter/n.ofd",
        "space-separated color value `#ee #20 #25` rejected",
    ),
    (
        "converter/zsbk.ofd",
        "page 1 object 665: DeltaX with more than 14 displacements rejected",
    ),
    (
        "converter/发票监制章-数科.ofd",
        "page 1 object 4: first TextCode run omits an origin coordinate",
    ),
    (
        "converter/发票示例.ofd",
        "page XML object without ID rejected",
    ),
    (
        "converter/文字横向-数科.ofd",
        "page 1 object 5: first TextCode run omits an origin coordinate",
    ),
    (
        "converter/透明度文字.ofd",
        "PublicRes.xml with duplicate Fonts elements rejected",
    ),
    (
        "layout/no_page_container.ofd",
        "JB2 image format unsupported",
    ),
];

/// Returns the recorded reason when the fixture is a known load failure.
pub fn known_load_failure(relative_fixture: &str) -> Option<&'static str> {
    KNOWN_LOAD_FAILURES
        .iter()
        .find(|(name, _)| *name == relative_fixture)
        .map(|(_, reason)| *reason)
}

/// Tries to open the fixture and load every page, returning the error text on
/// failure. Used by the whole-corpus smoke test.
pub fn try_load_all_pages(path: &Path) -> std::result::Result<usize, String> {
    let document =
        Document::open(path, LoadOptions::default()).map_err(|error| error.to_string())?;
    for index in 0..document.page_count() {
        document.page(index).map_err(|error| error.to_string())?;
    }
    Ok(document.page_count())
}

/// Concatenates the text of every text object on the page, walking layers and
/// nested page-block groups in source order. This mirrors what ofdrw's
/// `ContentExtractor` does for plain page content.
pub fn extract_page_text(page: &Page) -> String {
    let mut text = String::new();
    for layer in page.layers() {
        collect_objects_text(layer.objects(), &mut text);
    }
    text
}

fn collect_objects_text(objects: &[PageObject], text: &mut String) {
    for object in objects {
        match object {
            PageObject::Text(text_object) => {
                for run in text_object.runs() {
                    text.push_str(run.text());
                }
            }
            PageObject::Group(group) => collect_objects_text(group.objects(), text),
            _ => {}
        }
    }
}

/// Renders a page to an ARGB32 surface at the reference DPI using the system
/// font resolver shared with the rofd-render test suite.
pub fn render_page(page: &Page) -> std::result::Result<ImageSurface, String> {
    let options = RenderOptions {
        dpi: REFERENCE_DPI,
        ..RenderOptions::default()
    };
    let font_resolver = SystemFontResolver::with_system_fonts(
        FALLBACK_FAMILIES
            .iter()
            .map(|&family| family.to_owned())
            .collect(),
        page.resource_limits().max_font_bytes,
    );
    let image_decoder = ImageDecoder::default();
    let (width, height) = CairoRenderer::pixel_size(page, &options).map_err(|e| e.to_string())?;
    let surface = ImageSurface::create(Format::ARgb32, width, height).map_err(|e| e.to_string())?;
    let context = Context::new(&surface).map_err(|e| e.to_string())?;
    CairoRenderer
        .render_page_with_services(page, &context, &options, &font_resolver, &image_decoder)
        .map_err(|e| e.to_string())?;
    drop(context);
    Ok(surface)
}

/// Fixtures rofd-render currently cannot render although they load, with the
/// observed failure. Tracked like [`KNOWN_LOAD_FAILURES`]: the comparison
/// test fails when a listed fixture starts rendering.
pub const KNOWN_RENDER_FAILURES: &[(&str, &str)] = &[
    (
        "converter/containsJPEG.ofd",
        "image entries Doc_0/Res/Image_N.JPEG are reported missing although \
         ofdrw resolves them (likely a resource path resolution gap)",
    ),
    (
        "converter/z.ofd",
        "pages 2-4: explicit glyph IDs require a resolved primary font, but \
         the fonts are external (not embedded), so the text cannot be laid out",
    ),
];

/// Returns the recorded reason when the fixture is a known render failure.
pub fn known_render_failure(relative_fixture: &str) -> Option<&'static str> {
    KNOWN_RENDER_FAILURES
        .iter()
        .find(|(name, _)| *name == relative_fixture)
        .map(|(_, reason)| *reason)
}

/// Fixtures whose ofdrw-rendered reference is not a usable baseline.
pub const UNUSABLE_REFERENCES: &[(&str, &str)] = &[(
    "converter/pattern类型.ofd",
    "ofdrw renders an all-black page (its own pattern-fill bug) while rofd \
     renders the visible title text; there is nothing meaningful to diff",
)];

/// Returns the recorded reason when the ofdrw reference is unusable.
pub fn unusable_reference(relative_fixture: &str) -> Option<&'static str> {
    UNUSABLE_REFERENCES
        .iter()
        .find(|(name, _)| *name == relative_fixture)
        .map(|(_, reason)| *reason)
}

/// Outcome of comparing a rendered surface against an ofdrw reference PNG.
#[derive(Debug)]
pub struct DiffReport {
    /// Width and height of the rendered surface and the reference image.
    pub actual_size: (u32, u32),
    /// Reference image dimensions.
    pub reference_size: (u32, u32),
    /// Fraction of pixels whose RGB channels differ by more than
    /// [`CHANNEL_TOLERANCE`]; `1.0` when the dimensions differ.
    pub mismatch_fraction: f64,
}

/// Compares a rendered surface against an ofdrw-rendered reference PNG.
///
/// Dimensions may differ by at most two pixels per axis (ofdrw and rofd round
/// millimetres to pixels slightly differently); the comparison then covers the
/// overlapping region. Larger dimension differences fail with fraction 1.0.
pub fn diff_against_reference(surface: &mut ImageSurface, reference: &Path) -> DiffReport {
    let reference_image = ImageReader::open(reference)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", reference.display()))
        .decode()
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", reference.display()))
        .to_rgb8();
    let (ref_width, ref_height) = reference_image.dimensions();
    let actual_size = (surface.width() as u32, surface.height() as u32);
    let reference_size = (ref_width, ref_height);
    if actual_size.0.abs_diff(ref_width) > 2 || actual_size.1.abs_diff(ref_height) > 2 {
        return DiffReport {
            actual_size,
            reference_size,
            mismatch_fraction: 1.0,
        };
    }

    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().unwrap();
    let width = actual_size.0.min(ref_width) as usize;
    let height = actual_size.1.min(ref_height) as usize;
    let total = (width * height).max(1);
    let mut mismatched = 0usize;
    for y in 0..height {
        for x in 0..width {
            let offset = y * stride + x * 4;
            let native = u32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap());
            let actual = [
                ((native >> 16) & 0xff) as u8,
                ((native >> 8) & 0xff) as u8,
                (native & 0xff) as u8,
            ];
            let expected = reference_image.get_pixel(x as u32, y as u32).0;
            let differs = actual
                .iter()
                .zip(expected.iter())
                .any(|(a, e)| a.abs_diff(*e) > CHANNEL_TOLERANCE);
            if differs {
                mismatched += 1;
            }
        }
    }
    DiffReport {
        actual_size,
        reference_size,
        mismatch_fraction: mismatched as f64 / total as f64,
    }
}

/// Returns the maximum allowed mismatch fraction for a fixture path relative
/// to `fixtures/` (e.g. `converter/999.ofd`). Entries start lenient and are
/// tightened as the renderer converges with ofdrw's output.
pub fn max_mismatch_fraction(relative_fixture: &str) -> f64 {
    const OVERRIDES: &[(&str, f64)] = &[
        // Pages 2-3 carry the article body as explicit glyph IDs whose fonts
        // are not embedded; rofd skips most of that text (observed ~16%).
        ("converter/y.ofd", 0.18),
    ];
    OVERRIDES
        .iter()
        .find(|(name, _)| *name == relative_fixture)
        .map(|(_, fraction)| *fraction)
        .unwrap_or(DEFAULT_MAX_MISMATCH_FRACTION)
}
