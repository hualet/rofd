# rofd-render

`rofd-render` lowers validated, GUI-independent `rofd-core` page objects into an
immutable backend-neutral display list. It does not read OFD archives or XML;
rendering backends consume its commands in order. Effective page layers already
include recursively resolved background and foreground templates, so display
commands and diagnostics preserve the merged paint order without introducing a
dependency from `rofd-core` back to the renderer.

## Display lists and Cairo rendering

```no_run
use cairo::{Context, Format, ImageSurface};
use rofd_core::{Document, LoadOptions};
use rofd_render::{CairoRenderer, DisplayList, RenderOptions};

let document = Document::open("document.ofd", LoadOptions::default())?;
let page = document.page(0)?;

let display_list = DisplayList::from_page(&page)?;
for diagnostic in display_list.diagnostics() {
    eprintln!(
        "deferred object {} ({:?}) from {:?}",
        diagnostic.object_id(),
        diagnostic.kind(),
        diagnostic.source()
    );
}

let options = RenderOptions::default();
let (width, height) = CairoRenderer::pixel_size(&page, &options)?;
let surface = ImageSurface::create(Format::ARgb32, width, height)?;
let context = Context::new(&surface)?;
let report = CairoRenderer.render_page(&page, &context, &options)?;
assert_eq!(report.diagnostics(), display_list.diagnostics());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The phase-2 renderer supports path move, line, quadratic and cubic curves,
elliptical arcs, and close operations; solid RGBA fills and strokes; non-zero
and even-odd fill rules; affine object and clip transforms; true-union OFD clip
areas with intersection between separate clips; recursively merged background
and foreground templates; optional page-space clipping; and clockwise page
rotation by 0, 90, 180, or 270 degrees. Page coordinates are millimetres, page
boxes may have nonzero origins, and output dimensions follow the configured DPI
and scale.

`RenderOptions::default()` selects 96 DPI, scale 1, no rotation, an opaque white
background, no optional page-space clip, and a 256 MiB raster budget. The
`max_raster_bytes` budget conservatively covers the required ARGB32 target plus
the full-page clip mask and clipped-drawing intermediate. Invalid options,
unsupported Cairo dimensions, insufficient target surfaces, and budget excesses
are returned as structured errors. Rendering preserves the caller's Cairo
graphics state and current path on both success and recoverable failure.

## Font resolution and glyph positioning

`SystemFontResolver` owns one configured `fontdb` snapshot, so callers may use
an empty or custom database for reproducible output or explicitly request a
one-time system scan. `position_glyph_runs` resolves an embedded font before a
declared system family/name, then tries configured fallback families per
character. It returns exact, unshaped `GlyphRun` values in OFD object-space
millimetres. CGTransform glyph identifiers override character-map lookup;
explicit deltas, including zero, remain authoritative, while an absent axis
uses the selected face's FreeType advance. Returned sources and errors use
stable identities and never expose host font paths.

Resolved encoded bytes and face metadata are cached per resolver with
single-flight initialization. System font bytes are checked against the
resolver's configured limit before ownership. Cache failures are retryable,
and constructing a new resolver is how callers observe a newer installed-font
snapshot.

Text and image objects remain omitted from Cairo drawing and are reported
through display-list and render-report diagnostics, including object kind,
identifier, and page/template source. Phase 3 will later connect positioned
glyphs and decoded images to the display list and Cairo backend. Composite
objects, annotations, and signatures are also not rendered yet.
