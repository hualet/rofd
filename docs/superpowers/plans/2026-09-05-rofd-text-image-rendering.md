# OFD Text, Font, and Image Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render validated OFD text and PNG/JPEG image resources so the repository invoice becomes a useful, substantially complete page image.

**Architecture:** `rofd-core` lazily indexes and validates document resources, retaining original text and immutable encoded assets. `rofd-render` resolves fonts, lowers text/images into backend-neutral display commands, and consumes those commands through Cairo without exposing archive or GUI types.

**Tech Stack:** Rust 2021, serde-xml-rs, zip, cairo-rs with FreeType support, fontdb, image with only PNG/JPEG codecs, thiserror.

---

### Task 1: Add bounded resource indexing and lookup

**Files:**
- Modify: `crates/rofd-core/src/options.rs`
- Modify: `crates/rofd-core/src/raw.rs`
- Create: `crates/rofd-core/src/resources.rs`
- Modify: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/resources.rs`
- Modify: `crates/rofd-core/tests/support/mod.rs`

- [ ] **Step 1: Write failing resource-index tests**

Build in-memory packages whose `CommonData` names `PublicRes` and
`DocumentRes`. Cover normalized relative paths, `Res/@BaseLoc`, a font and a
PNG multimedia entry, missing optional resource files, unsafe paths, duplicate
IDs within/across files, malformed resource XML, concurrent lookup, and an
unreferenced corrupt asset that remains lazy.

```rust
let document = Document::from_bytes(package_with_resources(), LoadOptions::default())?;
let font = document.font_resource(3)?;
assert_eq!(font.family_name(), Some("Noto Sans CJK SC"));
assert_eq!(document.image_resource(36)?.format(), ImageFormat::Png);
```

- [ ] **Step 2: Confirm the public lookup API is absent**

Run: `cargo test -p rofd-core --test resources`

Expected: FAIL because `font_resource`, `image_resource`, and their public
resource types do not exist.

- [ ] **Step 3: Add explicit resource limits**

Extend `ResourceLimits` with non-zero finite defaults and builder-compatible
public fields:

```rust
pub max_resource_files: usize,       // default 32
pub max_resources: usize,            // default 100_000
pub max_font_bytes: u64,              // default 64 MiB
pub max_encoded_image_bytes: u64,     // default 64 MiB
pub max_decoded_image_pixels: u64,    // default 100_000_000
pub max_decoded_image_bytes: u64,     // default 400 MiB
pub max_text_characters_per_page: usize, // default 1_000_000
pub max_glyphs_per_page: usize,       // default 1_000_000
pub max_text_expansion_entries: usize, // default 2_000_000
```

Update the default-limit test to assert every new value is non-zero.

- [ ] **Step 4: Implement lazy immutable resource catalogs**

Deserialize only the resource index fields required by phase 3. Resolve
resource XML relative to `Document.xml` and asset paths relative to the resource
file plus `BaseLoc`. Publish a catalog only after both files parse and the
combined ID space validates.

```rust
pub enum ImageFormat { Png, Jpeg }

impl Document {
    pub fn font_resource(&self, id: u64) -> Result<FontResource>;
    pub fn image_resource(&self, id: u64) -> Result<ImageResource>;
}
```

Keep encoded bytes behind `Arc<[u8]>`; do not decode images or create font
backend handles in `rofd-core`.

- [ ] **Step 5: Verify limits, paths, concurrency, and isolation**

Run:

```bash
cargo test -p rofd-core --test resources
cargo clippy -p rofd-core --all-targets -- -D warnings
cargo tree -p rofd-core | rg 'cairo|freetype|fontdb|image|rofd-render'
```

Expected: tests and Clippy PASS; the dependency query prints nothing and exits
1.

- [ ] **Step 6: Commit**

```bash
git add crates/rofd-core
git commit -m "feat(core): index OFD document resources"
```

### Task 2: Parse draw parameters and validated text/image objects

**Files:**
- Modify: `crates/rofd-core/src/raw.rs`
- Modify: `crates/rofd-core/src/resources.rs`
- Modify: `crates/rofd-core/src/content.rs`
- Modify: `crates/rofd-core/src/document.rs`
- Create: `crates/rofd-core/src/text.rs`
- Create: `crates/rofd-core/src/image.rs`
- Create: `crates/rofd-core/tests/text_content.rs`
- Create: `crates/rofd-core/tests/image_content.rs`

- [ ] **Step 1: Write failing public-model tests**

Cover multiple `TextCode` runs, Unicode scalar counting, `X`/`Y`, ordinary and
repeated `DeltaX`/`DeltaY`, object alpha, boundary/CTM, fill/stroke defaults,
font/size, image resource ID, clips, nested groups, and page/template source.

```rust
match &page.layers()[0].objects()[0] {
    PageObject::Text(text) => {
        assert_eq!(text.font_id(), 3);
        assert_eq!(text.runs()[0].text(), "发票");
        assert_eq!(text.runs()[0].positions().len(), 2);
    }
    other => panic!("expected text, got {other:?}"),
}
```

Add invalid-value and exact/one-over tests for missing IDs, non-positive size,
non-finite coordinates, delta syntax/count overflow, invalid UTF-8/XML,
malformed `CGTransform`, missing resources, character/glyph/expansion limits,
and duplicate object IDs.

- [ ] **Step 2: Confirm text and image remain placeholders**

Run:

```bash
cargo test -p rofd-core --test text_content
cargo test -p rofd-core --test image_content
```

Expected: FAIL because `PageObject::Text` and `PageObject::Image` do not exist.

- [ ] **Step 3: Implement text positioning and glyph mappings**

Add immutable types with accessors. Expand delta repetition while parsing and
retain original Unicode separately from glyph IDs.

```rust
pub struct TextRun {
    text: String,
    origin: Point,
    offsets: Vec<Point>,
}

pub struct CharacterGlyphMap {
    code_position: usize,
    code_count: usize,
    glyphs: Vec<u32>,
}
```

Reject overlapping/out-of-range mappings and checked-add every expanded count.
An absent delta means zero offset, not an inferred advance; advance inference is
a renderer decision.

- [ ] **Step 4: Resolve inheritable drawing parameters**

Index `DrawParam` entries, resolve `Relative` chains with an active-ID stack,
and merge parent, referenced draw parameter, and object-local values in that
order. Reject unknown IDs, cycles, invalid dash/width/color values, and duplicate
resource IDs. Store only the effective validated paint state in page objects.

- [ ] **Step 5: Implement image objects and resource validation**

Convert `ImageObject` into a public immutable model containing object ID,
boundary, CTM, alpha, clips, primary resource ID, optional substitution and
image-mask resource IDs, and source. Require every referenced ID to name an
image multimedia resource. In phase 3, `Substitution` and `ImageMask` produce
explicit unsupported-image diagnostics (and a strict-mode error when faithful
output is required); do not silently treat either reference as a color.

- [ ] **Step 6: Verify page and template accounting**

Run:

```bash
cargo test -p rofd-core --test text_content --test image_content
cargo test -p rofd-core --test templates
cargo test -p rofd-core
```

Expected: PASS, including repeated template expansion and cache-warm-order
limit cases.

- [ ] **Step 7: Commit**

```bash
git add crates/rofd-core
git commit -m "feat(core): expose text and image page objects"
```

### Task 3: Add deterministic font resolution and positioned glyph runs

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/rofd-render/Cargo.toml`
- Create: `crates/rofd-render/src/fonts.rs`
- Modify: `crates/rofd-render/src/lib.rs`
- Create: `crates/rofd-render/tests/fonts.rs`
- Add: `tests/fixtures/fonts/phase3-subset.ttf`

- [ ] **Step 1: Add a repository-controlled subset font and failing tests**

Use a small redistributable OFL-licensed TTF fixture and record its license next
to the file. Test embedded-font priority, family-name system lookup, configured
fallback, missing glyphs, `CGTransform` glyph IDs, explicit OFD offsets, absent
delta advance fallback, cache reuse, and concurrent resolution.

```rust
let run = resolver.resolve_run(&font, text, &positions, &maps, 3.0)?;
assert_eq!(run.glyphs()[0].font_source(), FontSource::Embedded(3));
assert_eq!(run.glyphs()[0].x(), 10.0);
```

- [ ] **Step 2: Confirm the font resolver is absent**

Run: `cargo test -p rofd-render --test fonts`

Expected: FAIL because `FontResolver`, `GlyphRun`, and `FontDiagnostic` do not
exist.

- [ ] **Step 3: Add backend-only font dependencies**

Enable Cairo's `freetype` feature and add `fontdb`. Keep these dependencies out
of `rofd-core`.

```toml
cairo-rs = { version = "0.20", features = ["freetype"] }
fontdb = "0.23"
```

- [ ] **Step 4: Implement the resolver and bounded caches**

Expose a trait usable by applications and a default resolver:

```rust
pub trait FontResolver: Send + Sync {
    fn resolve(&self, resource: &FontResource, character: char) -> Result<ResolvedFont>;
}

pub struct SystemFontResolver {
    database: Arc<fontdb::Database>,
    cache: Mutex<HashMap<FontCacheKey, Arc<ResolvedFontData>>>,
}
```

Load embedded bytes with FreeType memory faces. Load system matches from owned
bytes, not borrowed file handles. Cache positive resolutions; cache negative
lookups only within one render operation so newly installed fonts can be seen by
a new resolver. Never expose FreeType pointers in public types.

- [ ] **Step 5: Build exact positioned glyph runs**

Use producer glyph IDs from valid `CGTransform`; otherwise use the selected
face's character map. Explicit OFD offsets win. When offsets are absent, use
FreeType advances converted to millimetres at the requested size. Emit a visible
replacement glyph and diagnostic when no candidate covers a character.

Keep `DisplayList::from_page(&Page)` as the default-system-resolver convenience
API and add `DisplayList::from_page_with_fonts(&Page, &dyn FontResolver)` for
controlled applications and deterministic tests. `ResolvedFont` owns font bytes
and stable face-index metadata; Cairo/FreeType handles remain private to the
backend cache.

- [ ] **Step 6: Verify deterministic embedded-font behavior**

Run:

```bash
cargo test -p rofd-render --test fonts
cargo clippy -p rofd-render --all-targets -- -D warnings
cargo tree -p rofd-core | rg 'cairo|freetype|fontdb|rofd-render'
```

Expected: tests and Clippy PASS; the core isolation query prints nothing.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/rofd-render tests/fixtures/fonts
git commit -m "feat(render): resolve OFD font glyphs"
```

### Task 4: Decode bounded PNG and JPEG resources

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/rofd-render/Cargo.toml`
- Create: `crates/rofd-render/src/images.rs`
- Modify: `crates/rofd-render/src/lib.rs`
- Create: `crates/rofd-render/tests/images.rs`
- Add: `tests/fixtures/images/orientation.png`
- Add: `tests/fixtures/images/orientation.jpg`

- [ ] **Step 1: Write failing decoder and limit tests**

Test PNG/JPEG magic detection, declared-format mismatch, dimensions and RGBA
orientation, alpha, corrupt/truncated data, decompression bombs, checked stride
math, exact/one-over pixel and byte limits, cache reuse, and concurrent decode.

```rust
let image = decoder.decode(&resource, &limits)?;
assert_eq!(image.dimensions(), (3, 2));
assert_eq!(image.rgba_pixel(0, 0), [255, 0, 0, 255]);
```

- [ ] **Step 2: Confirm decoding support is absent**

Run: `cargo test -p rofd-render --test images`

Expected: FAIL because `ImageDecoder` and `DecodedImage` do not exist.

- [ ] **Step 3: Add codec-minimal image dependency**

```toml
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }
```

- [ ] **Step 4: Implement header-first bounded decoding**

Read dimensions before full decode, validate checked `width * height * 4`
against both limits, decode to immutable RGBA8, then verify decoded dimensions
and length again. Publish cache entries only after complete validation.

```rust
pub struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
}
```

- [ ] **Step 5: Verify codecs and failure behavior**

Run:

```bash
cargo test -p rofd-render --test images
cargo clippy -p rofd-render --all-targets -- -D warnings
```

Expected: PASS without enabling GIF, WebP, TIFF, or AVIF dependencies.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/rofd-render tests/fixtures/images
git commit -m "feat(render): decode bounded OFD images"
```

### Task 5: Lower text and images into the display list

**Files:**
- Modify: `crates/rofd-render/src/display_list.rs`
- Modify: `crates/rofd-render/src/lib.rs`
- Modify: `crates/rofd-render/tests/display_list.rs`

- [ ] **Step 1: Write failing lowering tests**

Assert exact command order for path/text/image mixtures, nested groups,
templates, object CTMs, boundaries, clips, alpha, source provenance, fallback
diagnostics, and missing resources. Text and image objects must no longer create
`UnsupportedObject` diagnostics.

```rust
assert!(matches!(commands[index], Command::DrawGlyphRun(_)));
assert!(matches!(commands[index + 1], Command::DrawImage(_)));
```

- [ ] **Step 2: Confirm the command variants are absent**

Run: `cargo test -p rofd-render --test display_list`

Expected: FAIL because `DrawGlyphRun` and `DrawImage` are missing.

- [ ] **Step 3: Add backend-neutral commands**

```rust
Command::DrawGlyphRun(GlyphRun),
Command::DrawImage(DecodedImage),
```

Wrap each object in balanced `Save`, clip commands, boundary/CTM transforms,
paint/alpha state, draw command, and `Restore`. Keep commands non-exhaustive and
immutable. Resolve all fallible resources before publishing a `DisplayList`.

- [ ] **Step 4: Preserve diagnostics and budgets**

Attach page/template `LayerSource` to font fallback and missing-glyph
diagnostics. Revalidate effective character/glyph counts across repeated template
expansion even when resource/font caches are warm.

- [ ] **Step 5: Verify lowering and dependency direction**

Run:

```bash
cargo test -p rofd-render --test display_list
cargo test -p rofd-core
cargo clippy -p rofd-core -p rofd-render --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/rofd-render
git commit -m "feat(render): lower text and image commands"
```

### Task 6: Render glyphs through Cairo FreeType

**Files:**
- Modify: `crates/rofd-render/src/cairo_renderer.rs`
- Modify: `crates/rofd-render/src/lib.rs`
- Create: `crates/rofd-render/tests/cairo_text.rs`

- [ ] **Step 1: Write failing embedded-font raster tests**

Render controlled Latin and CJK glyphs using the repository font. Verify
baseline positions, explicit deltas, inferred advances, fill/stroke, alpha,
arbitrary CTM, clipping, all page rotations, `CGTransform`, missing-glyph box,
and caller state/path restoration on backend failure.

- [ ] **Step 2: Confirm glyph commands are not interpreted**

Run: `cargo test -p rofd-render --test cairo_text`

Expected: FAIL because CairoRenderer does not handle `DrawGlyphRun`.

- [ ] **Step 3: Render positioned FreeType glyphs**

Create Cairo font faces through the safe `cairo-rs` FreeType API and draw
positioned glyph IDs:

```rust
let face = cairo::FontFace::create_from_ft(&resolved.face)?;
context.set_font_face(&face);
context.set_font_size(run.size_mm());
context.show_glyphs(run.cairo_glyphs())?;
```

Use glyph paths for stroked text and `show_glyphs` for fill-only text. Apply the
same deterministic antialias/tolerance policy as paths. Balance every temporary
save on all error paths and preserve the caller's original font state.

- [ ] **Step 4: Verify pixels and deterministic state**

Run:

```bash
cargo test -p rofd-render --test cairo_text
cargo test -p rofd-render --test cairo_render
cargo clippy -p rofd-render --all-targets -- -D warnings
```

Expected: PASS with repository-controlled font pixels stable across runs.

- [ ] **Step 5: Commit**

```bash
git add crates/rofd-render
git commit -m "feat(render): draw positioned OFD glyphs"
```

### Task 7: Composite decoded images through Cairo

**Files:**
- Modify: `crates/rofd-render/src/cairo_renderer.rs`
- Modify: `crates/rofd-render/src/lib.rs`
- Create: `crates/rofd-render/tests/cairo_image.rs`

- [ ] **Step 1: Write failing image raster tests**

Verify pixel orientation, object-boundary scaling, nonzero boundary origin,
object CTM, alpha, nearest/bilinear filtering, OFD clipping, optional page clip,
all page rotations, explicit `Substitution`/`ImageMask` diagnostics, and state
restoration on decode/backend errors.

- [ ] **Step 2: Confirm image commands are not interpreted**

Run: `cargo test -p rofd-render --test cairo_image`

Expected: FAIL because CairoRenderer does not handle `DrawImage`.

- [ ] **Step 3: Add deterministic interpolation options**

```rust
pub enum ImageInterpolation { Nearest, Bilinear }
```

Add `image_interpolation` to `RenderOptions`, defaulting to `Bilinear`, and map
it explicitly to Cairo filters instead of inheriting caller pattern state.

- [ ] **Step 4: Composite immutable RGBA data**

Convert RGBA to Cairo's native-endian premultiplied ARGB32 with checked stride
and allocation, map the pixel rectangle to the object boundary, apply alpha and
clip masks, then composite. Count this temporary buffer against the existing
raster budget or a separately documented image working-set budget before
allocation.

- [ ] **Step 5: Verify images and existing paths/text**

Run:

```bash
cargo test -p rofd-render --test cairo_image
cargo test -p rofd-render
cargo clippy -p rofd-render --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/rofd-render
git commit -m "feat(render): composite OFD images"
```

### Task 8: Make the repository invoice substantially complete

**Files:**
- Modify: `crates/rofd-render/tests/real_fixture.rs`
- Add: `tests/fixtures/reference/invoice-phase3.png`
- Modify: `crates/rofd-core/README.md`
- Modify: `crates/rofd-render/README.md`
- Modify: `README.md`
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: Confirm the phase-2 fixture contract is obsolete**

Run: `cargo test -p rofd-render --test real_fixture -- --nocapture`

Expected: FAIL after Tasks 1-7 because the old test still expects 48 deferred
text/image diagnostics that are now rendered.

- [ ] **Step 2: Replace it with phase-3 acceptance assertions**

For `learning/test.ofd`, require 26 path draws, 47 text objects lowered to
non-empty glyph runs, image resource 36 lowered once, and zero deferred
text/image diagnostics. Render at controlled DPI with a controlled font set and
assert stable title, amount, QR, and invoice-rule regions. Run the updated test
and expect PASS before recording a reference image.

- [ ] **Step 3: Add a reviewed reference image and semantic checks**

Generate `invoice-phase3.png` only through a dedicated ignored/update command,
visually inspect it, then commit it with provenance (fixture, DPI, font set,
renderer version). The normal test must never rewrite the reference. Compare a
hash only when embedded/controlled fonts are used; otherwise compare stable
regions and command semantics.

- [ ] **Step 4: Document support and remaining limits**

Update API examples and the support matrix for text positioning, embedded and
fallback fonts, PNG/JPEG, resource limits, diagnostics, and cache behavior.
State that composites, advanced color spaces, annotations, signatures, and text
query APIs remain deferred.

- [ ] **Step 5: Update CI and run the complete phase gate**

Install FreeType/fontconfig and a controlled test font in CI, then run:

```bash
cargo fmt --all -- --check
cargo clippy -p rofd-core -p rofd-render --all-targets -- -D warnings
cargo test -p rofd-core -p rofd-render
cargo test --workspace --all-targets
cargo test -p rofd-render --test real_fixture -- --nocapture
cargo tree -p rofd-core | rg 'cairo|freetype|fontdb|image|rofd-render'
```

Expected: all format/lint/test commands PASS; the dependency search prints
nothing and exits 1; the worktree contains no generated files.

- [ ] **Step 6: Commit**

```bash
git add README.md .github/workflows/rust.yml crates tests/fixtures/reference
git commit -m "docs: describe text and image rendering"
```
