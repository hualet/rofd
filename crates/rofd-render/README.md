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

Applications that need host fonts construct one explicit
`SystemFontResolver::with_system_fonts` snapshot and pass it with a reusable
`ImageDecoder` to `CairoRenderer::render_page_with_services`. This keeps default
rendering deterministic while allowing ordered CJK or application-specific
fallback families and shared bounded caches.

The phase-2 renderer supports path move, line, quadratic and cubic curves,
elliptical arcs, and close operations; solid RGBA fills and strokes; non-zero
and even-odd fill rules; affine object and clip transforms; true-union OFD clip
areas with intersection between separate clips; recursively merged background
and foreground templates; optional page-space clipping; and clockwise page
rotation by 0, 90, 180, or 270 degrees. Page coordinates are millimetres, page
boxes may have nonzero origins, and output dimensions follow the configured DPI
and scale.

`RenderOptions::default()` selects 96 DPI, scale 1, no rotation, bilinear image
sampling, an opaque white background, no optional page-space clip, and a 256 MiB
raster budget. The `max_raster_bytes` budget conservatively covers the required
ARGB32 target, full-page clip mask, clipped-drawing intermediate, and the
aggregate premultiplied buffers for unique decoded images. Native buffers are
prepared once per unique allocation and reused by repeated image commands.
Invalid options, unsupported
Cairo dimensions, insufficient target surfaces, allocation failures, and budget
excesses are returned as structured errors. Rendering preserves the caller's
Cairo graphics state and current path on both success and recoverable failure.

## Pixel viewport rendering

`CairoRenderer::pixel_canvas_size` calculates the final rotated/scaled canvas
with `ceil` pixel dimensions. It accepts positive dimensions through `i32::MAX`
without applying raster allocation limits. Existing `pixel_size` and full-page
rendering retain their Cairo dimension and memory-budget checks.

Use `render_page_region` or `render_page_region_with_services` with a `PixelRect`
to render part of that final canvas into the target's top-left corner. The
viewport must have nonnegative coordinates, positive dimensions, and lie
entirely inside the canvas. Output is limited to the viewport extent, including
when the caller provides a larger target. `RenderOptions::clip` remains an
absolute page-space millimetre clip; it does not resize the canvas or viewport.

The page transform subtracts the exact integer viewport origin, preserving the
requested DPI and scale. The raster budget covers viewport-sized destination,
clip-mask and drawing surfaces, plus bounded decoded source images. This allows
small views into canvases larger than Cairo's 32767-pixel image limit. The whole
page display list is still lowered and traversed; this API does not yet perform
object-level spatial culling. Source images are still decoded at their full
bounded source resolution. Embedded mini-OFD stamps preserve their original
sampling dimensions but rasterize only the source region sampled by the current
viewport, with a two-pixel interpolation halo. That source region can be larger
than the output tile when an annotation shrinks the seal; it uses the remaining
raster budget after the parent target and prepared image sources are accounted
for. Existing full-page stamp rendering is unchanged.

Cairo's path coordinates use a smaller fixed-point domain than the canvas-size
API. Region rendering bounds page/option/stamp rectangles and fill-only
axis-aligned path rectangles before passing them to Cairo. It skips fill-only
line/Bezier paths whose complete control hull is provably outside the viewport.
Other display-list path or clip coordinates outside the conservative
±4,000,000 device-pixel range produce `Error::InvalidGeometry` rather than a
successful but incorrectly blank raster. This guard is conservative for
strokes and arcs; it does not implement arbitrary geometric clipping. Full-page
rendering retains its existing behavior. Image boundary rectangles use the same
bounded clipping while retaining the original source sampling transform.

Rectangular geometry, clipping, image sampling, and stamp positioning are tested
against full-page pixels at all four rotations and fractional DPI/scale. Cairo
can produce slightly different antialias coverage when tessellating curves or
culling glyph edges against a smaller destination extent, so byte-for-byte
identity with full-page rendering is not guaranteed. Tests separately bound
these sparse edge differences while retaining exact geometry/image checks.

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
`DrawImage` targets a normalized 1×1 local box; the preceding object transform
maps that box to its OFD boundary. During 0.1.0 development this corrected the
earlier preview's `width_mm`/`height_mm` fields, which multiplied dimensions when
an OFD image also supplied a scaling CTM.
Fallbacks, missing glyphs, and unsupported image extensions are reported with the
owning object and page/template source. Cairo renders positioned glyph IDs from
resolved FreeType faces, batches consecutive glyphs using the same face, and
uses glyph outlines when fill plus stroke is required. Missing glyphs use a
deterministic visible box. Cairo also converts immutable RGBA8 images to
native-endian premultiplied ARGB32 and composites them with explicit nearest or
bilinear sampling. Text and images honor object transforms, page rotation,
opacity, optional page clipping, and the same A8 union/intersection clip masks as
paths. Image substitution, masks, and borders remain explicit diagnostics;
composite objects, annotations, and signatures are also not rendered yet.

The expanded diagnostic API uses `RenderDiagnostic::kind()` to return
`RenderDiagnosticKind`; callers of the earlier renderer preview should migrate
unsupported-object checks to `RenderDiagnostic::unsupported_kind()`.

The reviewed phase-3 invoice reference is generated by `rofd-render` 0.1.0 from
`learning/test.ofd` at 254 DPI with `Noto Sans CJK SC` and `Noto Sans Mono`
fallback. Normal tests only read it. To intentionally refresh it after visual
review, run:

```bash
ROFD_UPDATE_REFERENCES=1 cargo test -p rofd-render --test real_fixture \
  update_invoice_phase3_reference -- --ignored
```
