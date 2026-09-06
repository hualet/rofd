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
per-face single-flight initialization. The default cache retains at most 64
successfully resolved faces with deterministic least-recently-used eviction;
`SystemFontResolver::from_database_with_cache_capacity` configures another
positive finite bound. Eviction does not invalidate returned `ResolvedFont`
clones. System font bytes are checked against the resolver's configured limit
before ownership. Cache failures are retryable, and constructing a new resolver
is how callers observe a newer installed-font snapshot.

## Image decoding

`ImageDecoder` validates PNG/JPEG byte signatures against the resource catalog,
reads and bounds dimensions before allocating pixels, configures the underlying
decoder's allocation limits, and returns immutable top-to-bottom native RGBA8
`DecodedImage` values. Pixel, stride, dimension, format, corruption, overflow,
and limit failures are structured and retain the safe package-local asset path.

Decoded pixels use a per-resource single-flight cache keyed by opaque resource
identity, so equal numeric IDs from different documents cannot collide. The
default cache retains at most 64 MiB of decoded RGBA bytes with deterministic
least-recently-used eviction; `ImageDecoder::with_cache_byte_budget` selects a
different positive byte bound. Valid images larger than that cache policy are
returned uncached. Failures are never retained, unrelated resources may decode
concurrently, and existing `DecodedImage` clones remain valid after eviction.

`DisplayListBuilder` lowers text and image objects into backend-neutral
`DrawGlyphRun` and `DrawImage` commands with injectable font and image services.
It bounds the aggregate unique decoded-image allocations retained by one list;
callers may override that default with `with_max_decoded_image_bytes`.
Fallbacks, missing glyphs, and deferred image extensions are reported with the
owning object and page/template source. Cairo renders positioned glyph IDs from
resolved FreeType faces, batches consecutive glyphs using the same face, and
uses glyph outlines when fill plus stroke is required. Missing glyphs use a
deterministic visible box. Text honors object transforms, page rotation, paint,
and the same A8 union/intersection clip masks as paths. Cairo still rejects
`DrawImage` before display-list image decoding or painting until image
compositing lands. Composite objects, annotations, and signatures are also not
rendered yet.

The expanded diagnostic API uses `RenderDiagnostic::kind()` to return
`RenderDiagnosticKind`; callers of the earlier renderer preview should migrate
unsupported-object checks to `RenderDiagnostic::unsupported_kind()`.
