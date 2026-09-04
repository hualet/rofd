# OFD Text, Font, and Image Rendering Design

**Date:** 2026-09-05  
**Status:** Approved direction; implementation plan pending review

## Goal

Phase 3 makes the reusable renderer useful for ordinary invoices, receipts, and
form-like OFD documents. `rofd-core` will expose validated text, font, image,
and drawing-resource models; `rofd-render` will lower them into backend-neutral
commands and render them with Cairo. The Qt reader, C ABI, text search,
annotations, and signature validation remain outside this phase.

The repository invoice is the acceptance fixture: its 47 text objects and QR
image must render instead of appearing as deferred diagnostics. Existing path,
clip, template, rotation, resource-limit, and caller-state guarantees remain
unchanged.

## Architecture and Dependency Direction

The dependency direction stays `rofd-render -> rofd-core`. Resource XML and ZIP
entries are read only by `rofd-core`; Cairo never receives a package path or
archive handle. GUI dependencies remain absent from both crates.

`Document` indexes `PublicRes` and `DocumentRes` when opening the document, but
loads resource XML, font bytes, and image bytes lazily. Parsed resources and
successful decoded assets are immutable and shared through `Arc`. Failed loads
do not publish partial cache entries and remain deterministic under concurrent
page rendering.

Public and document resources form one document-wide ID space. Duplicate IDs
across loaded resource files are invalid rather than resolved by load order.
References are resolved relative to the file declaring them: resource XML paths
are relative to `Document.xml`, while `FontFile` and `MediaFile` are relative to
the resource file's `BaseLoc`.

## Core Resource Model

`rofd-core` adds immutable models for:

- `FontResource`: object ID, font/family names, optional charset metadata, and
  optional embedded font data;
- `ImageResource`: object ID, declared media type/format, and bounded encoded
  bytes for PNG or JPEG;
- `DrawParam`: validated inheritable paint and stroke values needed by page
  graphics;
- `TextObject`: boundary, CTM, font ID, size, fill/stroke state, alpha, clips,
  character transforms, and ordered `TextCode` runs;
- `ImageObject`: boundary, CTM, primary resource ID, optional substitution and
  image-mask resource IDs, alpha, and clips.

`TextCode` retains the original Unicode string plus validated `X`, `Y`,
`DeltaX`, and `DeltaY`. Repeated-delta syntax is expanded with checked counts.
Character-to-glyph mappings (`CGTransform`) are retained and validated so a
subsetted embedded font can use the producer's glyph IDs. The model preserves
text independently from rendering so phase 4 can implement extraction and
search without reverse-engineering pixels.

Unsupported resource kinds and image formats produce explicit diagnostics in
lenient mode and structured errors in strict mode when correct rendering is not
possible. Missing referenced resources are always errors.

## Font Resolution and Glyph Positioning

Font resolution follows a deterministic priority:

1. a valid embedded `FontFile` from the referenced OFD font resource;
2. an installed font matching the declared family/name;
3. a configured generic fallback that contains the required character;
4. a visible missing-gbox plus a `MissingGlyph` diagnostic.

The default resolver is owned by `rofd-render` and is replaceable through a
small `FontResolver` trait for applications that need controlled font sets.
Resolved font data is cached by resource ID and fallback identity. The core API
does not expose backend-specific font handles.

OFD coordinates remain authoritative. `TextCode` origins and deltas determine
glyph placement; shaping-derived advances are used only when the OFD run omits
the corresponding deltas. `CGTransform` overrides Unicode-to-glyph mapping for
its declared code range. Font size is expressed in millimetres before the page
transform. Object CTM, boundary translation, alpha, fill/stroke, and clipping
apply exactly as they do to paths.

The backend-neutral display list adds `DrawGlyphRun`, containing resolved font
identity, original text range, positioned glyph IDs, and per-glyph transforms.
Cairo renders outlines rather than relying on its toy text API, keeping
embedded fonts, explicit glyph IDs, arbitrary CTMs, and raster output
consistent across systems. Font fallback choices are returned in the render
report as diagnostics.

## Image Decoding and Drawing

The display list adds `DrawImage`, referencing immutable decoded RGBA pixels,
pixel dimensions, and the image object's page transform. PNG and JPEG are the
only formats required in this phase. Format detection validates encoded bytes;
it does not trust only the XML extension or `Format` string.

Images map their full pixel rectangle into the OFD object boundary and then
apply the object CTM. Interpolation is deterministic and selected through a
render option with a documented default. Object alpha, page rotation, optional
page clip, and OFD clips apply to image composition. Phase 3 retains
`Substitution` and `ImageMask` resource references but reports them as explicit
unsupported-image diagnostics; high-resolution substitution selection and
binary image-mask composition are deferred rather than silently misrendered.

Decoding occurs before mutating the caller's Cairo context. Decoded images are
cached by resource ID. Corrupt data, dimension overflow, allocation failure,
and format mismatch are structured errors with the declaring OFD path and
resource ID.

## Resource Limits and Failure Semantics

`ResourceLimits` gains finite defaults for resource-file count, resources per
document, encoded font/image bytes, decoded image pixels/bytes, text objects,
characters per page, glyphs per page, and expanded delta/CGTransform entries.
All products and sums use checked arithmetic. Limits are enforced before large
allocation, decompression, decoding, or expansion.

Resource XML receives the existing XML depth and entry-size checks. Package
path normalization and archive total-size limits continue to apply. A cached
resource cannot bypass a tighter per-operation expansion limit, and acceptance
must not depend on cache warm-up order.

Strict mode fails when referenced content cannot be rendered faithfully.
Lenient mode may substitute a font or skip an unsupported unreferenced resource
only when it records a stable diagnostic. It never converts corrupt referenced
data, missing IDs, unsafe paths, or resource-limit violations into warnings.

## Public Rendering Contract

`DisplayList::from_page` resolves required resources and emits path,
`DrawGlyphRun`, and `DrawImage` commands in the existing effective paint order.
Unsupported composite objects remain diagnostics. `CairoRenderer::render_page`
continues to preserve the caller's complete graphics state and current path on
success and failure.

Rendering options remain bounded. Font and image caches do not count against
the caller-provided Cairo surface budget, so their own decoded-byte budgets are
reported separately. The render report includes fallback, missing-glyph,
unsupported-format, and deferred-object diagnostics with page/template source
provenance.

## Verification and Acceptance

Unit and integration tests cover resource path resolution, duplicate and
missing IDs, lazy/concurrent cache publication, malformed XML, delta expansion,
CGTransform ranges, embedded and fallback fonts, missing glyphs, PNG/JPEG
decoding, alpha, transforms, clips, and every exact/one-over resource limit.

Small deterministic fixtures verify glyph positions and image orientation at
0/90/180/270-degree page rotations. Tests compare semantic pixels or bounded
regions rather than host-dependent full-page snapshots when system fallback is
involved. Embedded-font fixtures may use full image hashes because their font
bytes are repository-controlled.

For `learning/test.ofd`, phase completion requires:

- all 47 text objects to lower to glyph runs;
- image resource 36 to decode and render at the expected QR-code location;
- zero deferred text/image diagnostics;
- the existing 26 path commands and template/clip behavior to remain stable;
- a controlled-font render to contain verified title, amount, QR, and rule
  regions; and
- formatting, strict Clippy, package/workspace tests, core dependency isolation,
  and CI to pass without Qt or QML packages.

## Deferred Work

Phase 4 adds text extraction/search APIs, hit boxes, annotations, and signature
metadata. Phase 5 freezes the C ABI. Phase 6 builds the Qt/QML reader exclusively
on that ABI. Composite graphics, advanced color spaces, gradients, non-PNG/JPEG
images, full signature validation, editing, and form filling remain later work.
