# rofd Display List and Basic Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a GUI-independent page content model and display list, then render OFD paths, transforms, clipping, layers, and template pages through a new `rofd-render` crate backed by Cairo.

**Architecture:** `rofd-core` remains the only crate that reads ZIP/XML and exposes owned, immutable page-domain objects. `rofd-render` depends on that public model, lowers it into a backend-neutral display list, and interprets the commands with Cairo; it never reads package files or private XML. Unsupported text and image objects remain explicit in the model and produce diagnostics until phase 3 implements them.

**Tech Stack:** Rust 2021, `serde-xml-rs`, `thiserror`, `cairo-rs`, Cargo workspace, integration tests with in-memory OFD fixtures.

---

## Scope and file map

- `crates/rofd-core/src/geometry.rs`: finite points, affine transforms, and rectangles.
- `crates/rofd-core/src/paint.rs`: RGB colors and fill rules.
- `crates/rofd-core/src/path_data.rs`: panic-free `M/L/Q/B/A/C` abbreviated-path parsing.
- `crates/rofd-core/src/content.rs`: public layer, object, path, group, and clip domain model.
- `crates/rofd-core/src/raw.rs`: private Serde structures for page content and templates.
- `crates/rofd-core/src/document.rs`: lazy page-content and template loading.
- `crates/rofd-render/src/display_list.rs`: flatten content into ordered backend-neutral commands.
- `crates/rofd-render/src/cairo_renderer.rs`: Cairo command interpreter and render options.
- `crates/rofd-render/tests/`: black-box lowering and raster tests.

### Task 1: Add finite graphics value types

**Files:**
- Modify: `crates/rofd-core/src/geometry.rs`
- Create: `crates/rofd-core/src/paint.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/graphics_values.rs`

- [ ] **Step 1: Write failing value-type tests**

Create tests covering `Point::new`, `Transform::parse`, transform composition/application, `Color::parse_rgb`, alpha validation, and rejection of `NaN`, infinity, wrong arity, and channels outside `0..=255`.

```rust
use rofd_core::{Color, Point, Transform};

#[test]
fn affine_transform_maps_points() {
    let matrix = Transform::parse("2 0 0 3 4 5").unwrap();
    assert_eq!(matrix.apply(Point::new(1.0, 2.0).unwrap()), Point::new(6.0, 11.0).unwrap());
}

#[test]
fn graphics_values_reject_non_finite_input() {
    assert!(Transform::parse("1 0 0 1 NaN 0").is_err());
    assert!(Color::parse_rgb("256 0 0", None).is_err());
}
```

- [ ] **Step 2: Run the test and confirm the missing exports**

Run: `cargo test -p rofd-core --test graphics_values`

Expected: FAIL because `Color`, `Point`, and `Transform` are not exported.

- [ ] **Step 3: Implement the values without unchecked indexing**

Add `Point { x, y }`, `Transform { a, b, c, d, e, f }`, `Transform::IDENTITY`, `parse`, `then`, and `apply`. Add `Color { red, green, blue, alpha }`, `Color::BLACK`, and `parse_rgb(value, alpha)`. All constructors validate finite coordinates and alpha/channel ranges, returning `Error::InvalidValue`.

- [ ] **Step 4: Export and verify the values**

Run:

```bash
cargo test -p rofd-core --test graphics_values
cargo clippy -p rofd-core --all-targets -- -D warnings
```

Expected: both commands PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/rofd-core/src/geometry.rs crates/rofd-core/src/paint.rs crates/rofd-core/src/lib.rs crates/rofd-core/tests/graphics_values.rs
git commit -m "feat(core): add graphics value types"
```

### Task 2: Parse OFD abbreviated path data

**Files:**
- Create: `crates/rofd-core/src/path_data.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/path_data.rs`

- [ ] **Step 1: Write failing path tests**

Test all standard commands and whitespace-free producer output. The public representation is:

```rust
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo { control: Point, end: Point },
    CubicTo { control1: Point, control2: Point, end: Point },
    ArcTo { rx: f64, ry: f64, rotation: f64, large: bool, sweep: bool, end: Point },
    Close,
}

pub struct PathData {
    commands: Vec<PathCommand>,
}
```

Include `M 0 0 L200 0 L 200 150 L 0 150 C`, negative/exponent numbers, malformed arity, unknown commands, non-finite values, and arc flags other than `0` or `1`.

- [ ] **Step 2: Confirm the tests fail**

Run: `cargo test -p rofd-core --test path_data`

Expected: FAIL because `PathData` and `PathCommand` do not exist.

- [ ] **Step 3: Implement a tokenizer and command parser**

Tokenize letters separately from signed decimal/exponent numbers, then consume exact command arities: `M/L=2`, `Q=4`, `B=6`, `A=7`, `C=0`. Reject missing initial `M`, trailing operands, repeated `M`, unknown letters, invalid arc flags, and all non-finite values with `Error::InvalidValue { field: "path data", ... }`. Expose `commands(&self) -> &[PathCommand]`.

- [ ] **Step 4: Verify and fuzz the parser with representative malformed strings**

Run:

```bash
cargo test -p rofd-core --test path_data
cargo test -p rofd-core path_data
```

Expected: PASS with no panic.

- [ ] **Step 5: Commit**

```bash
git add crates/rofd-core/src/path_data.rs crates/rofd-core/src/lib.rs crates/rofd-core/tests/path_data.rs
git commit -m "feat(core): parse OFD path data"
```

### Task 3: Expose lazy page layers and content objects

**Files:**
- Create: `crates/rofd-core/src/content.rs`
- Modify: `crates/rofd-core/src/raw.rs`
- Modify: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Modify: `crates/rofd-core/tests/support/mod.rs`
- Create: `crates/rofd-core/tests/page_content.rs`

- [ ] **Step 1: Write a failing nested-content test**

Build an in-memory page containing Background, Body, and Foreground layers; a stroked/filled `PathObject`; a nested `PageBlock`; and placeholder `TextObject`/`ImageObject` nodes. Assert XML order and attributes survive as:

```rust
pub enum LayerType { Background, Body, Foreground }
pub struct Layer { /* object_id, kind, objects */ }
pub enum PageObject { Path(PathObject), Group(PageGroup), Unsupported(UnsupportedObject) }
pub struct PathObject { /* object_id, boundary, transform, path, stroke, fill, line_width, clips */ }
```

`Page::layers()` returns a slice. `UnsupportedObject` records object ID and kind so later phases are explicit rather than silently dropping content.

- [ ] **Step 2: Confirm the test fails**

Run: `cargo test -p rofd-core --test page_content`

Expected: FAIL because `Page::layers` and the content types do not exist.

- [ ] **Step 3: Add private raw XML structures**

Extend `PageRoot` with optional `Content`, whose `Layer` and `PageBlock` use `$value` vectors containing `PathObject`, `PageBlock`, `TextObject`, `ImageObject`, and `CompositeObject`. Parse `Boundary`, optional `CTM`, `LineWidth`, `Stroke` (default true), `Fill` (default false), `Rule` (default NonZero), `StrokeColor`, `FillColor`, and `AbbreviatedData`. Placeholder variants retain `ID` only.

- [ ] **Step 4: Convert raw objects to the public domain model**

During lazy `Document::page()`, validate all values through `Rect`, `Transform`, `Color`, and `PathData`; recursively convert `PageBlock`; preserve layer and object order. Reject duplicate object IDs within the page as `InvalidStructure`. Store the immutable layers in `PageData` and expose them through `Page::layers()`.

- [ ] **Step 5: Verify compatibility with empty and real pages**

Run:

```bash
cargo test -p rofd-core --test page_content
cargo test -p rofd-core --test real_fixture
cargo test -p rofd-core
```

Expected: all PASS; the real fixture contains explicit unsupported text/image nodes without preventing page metadata access.

- [ ] **Step 6: Commit**

```bash
git add crates/rofd-core/src crates/rofd-core/tests
git commit -m "feat(core): expose page content model"
```

### Task 4: Build a backend-neutral display list

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/rofd-render/Cargo.toml`
- Create: `crates/rofd-render/src/lib.rs`
- Create: `crates/rofd-render/src/display_list.rs`
- Create: `crates/rofd-render/tests/display_list.rs`

- [ ] **Step 1: Write a failing lowering test**

Create a synthetic OFD page through `rofd-core` and assert the exact command sequence:

```rust
pub enum Command {
    Save,
    ConcatTransform(Transform),
    ClipPath { path: PathData, rule: FillRule },
    SetStroke(Option<Color>),
    SetFill(Option<Color>),
    SetLineWidth(f64),
    DrawPath(PathData),
    Restore,
}

pub struct DisplayList { commands: Vec<Command>, diagnostics: Vec<RenderDiagnostic> }
```

Each path emits `Save`, boundary translation, optional CTM, clips, paint state, path, and `Restore`. Nested groups flatten in source order. Unsupported objects add a diagnostic with object ID and kind.

- [ ] **Step 2: Confirm the package is absent**

Run: `cargo test -p rofd-render --test display_list`

Expected: FAIL because `rofd-render` is not a workspace package.

- [ ] **Step 3: Create the crate and lowering implementation**

Add `crates/rofd-render` to workspace members and default members. Depend only on `rofd-core`, `thiserror`, and `cairo-rs`. Add `DisplayList::from_page(&Page) -> Result<Self>`; reject non-positive line widths and unbalanced internal state rather than emitting invalid commands.

- [ ] **Step 4: Prove the dependency direction and command order**

Run:

```bash
cargo test -p rofd-render --test display_list
cargo tree -p rofd-core | rg 'cairo|qmetaobject|rofd-render'
```

Expected: display-list tests PASS; the dependency search has no output and exits 1.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/rofd-render
git commit -m "feat(render): add page display lists"
```

### Task 5: Parse and apply path clipping

**Files:**
- Modify: `crates/rofd-core/src/content.rs`
- Modify: `crates/rofd-core/src/raw.rs`
- Modify: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/tests/page_content.rs`
- Modify: `crates/rofd-render/src/display_list.rs`
- Modify: `crates/rofd-render/tests/display_list.rs`

- [ ] **Step 1: Write a failing clip test**

Use `<Clips><Clip><Area CTM="1 0 0 1 2 3"><Path Boundary="0 0 10 10" Fill="true"><AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</AbbreviatedData></Path></Area></Clip></Clips>`. Assert the domain model retains the area CTM and path boundary, and the display list emits their transforms before `ClipPath`.

- [ ] **Step 2: Confirm the clip is not yet represented**

Run: `cargo test -p rofd-render --test display_list clip`

Expected: FAIL because the expected clip commands are absent.

- [ ] **Step 3: Implement path-only clips**

Parse `Clips/Clip/Area/Path` into `Clip { transform, paths }`; each clip path has its own boundary, transform, data, and fill rule. Flatten each area using nested `Save` and transform commands. If a clip area contains text, return `UnsupportedFeature("text clip areas are not supported in phase 2")` instead of silently widening the visible area.

- [ ] **Step 4: Verify clipping and malformed input**

Run:

```bash
cargo test -p rofd-core --test page_content clip
cargo test -p rofd-render --test display_list clip
```

Expected: PASS; malformed clip paths return errors without panic.

- [ ] **Step 5: Commit**

```bash
git add crates/rofd-core crates/rofd-render
git commit -m "feat(render): support path clipping"
```

### Task 6: Interpret display lists with Cairo

**Files:**
- Create: `crates/rofd-render/src/cairo_renderer.rs`
- Modify: `crates/rofd-render/src/lib.rs`
- Create: `crates/rofd-render/tests/cairo_render.rs`

- [ ] **Step 1: Write failing raster tests**

Render a 10 mm square with a red fill and black stroke to an ARgb32 image surface. Assert the center pixel is red, an outside pixel remains the configured white background, and clipping excludes pixels outside the clip. Add option tests for DPI, scale, rotation, background, and an optional page-space clip rectangle.

- [ ] **Step 2: Confirm the renderer is absent**

Run: `cargo test -p rofd-render --test cairo_render`

Expected: FAIL because `CairoRenderer` and `RenderOptions` do not exist.

- [ ] **Step 3: Implement render options and the Cairo interpreter**

Expose:

```rust
pub struct RenderOptions {
    pub dpi: f64,
    pub scale: f64,
    pub rotation_degrees: u16,
    pub background: Color,
    pub clip: Option<Rect>,
}

pub struct CairoRenderer;

impl CairoRenderer {
    pub fn render_page(
        &self,
        page: &rofd_core::Page,
        context: &cairo::Context,
        options: &RenderOptions,
    ) -> Result<RenderReport>;
}
```

Validate positive finite DPI/scale and rotations in `{0, 90, 180, 270}`. Paint the background, convert millimetres with `dpi / 25.4`, apply rotation and optional clip, then interpret every display command. Implement quadratic curves by converting them to cubic curves and implement elliptical arcs with a saved/transformed Cairo context. Map every Cairo failure to `Error::Backend`.

- [ ] **Step 4: Verify pixels and state restoration**

Run:

```bash
cargo test -p rofd-render --test cairo_render
cargo clippy -p rofd-render --all-targets -- -D warnings
```

Expected: PASS, including a test that a renderer failure cannot leave an unmatched save on the caller's context.

- [ ] **Step 5: Commit**

```bash
git add crates/rofd-render
git commit -m "feat(render): render paths with Cairo"
```

### Task 7: Merge template pages by layer order

**Files:**
- Modify: `crates/rofd-core/src/raw.rs`
- Modify: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/src/content.rs`
- Create: `crates/rofd-core/tests/templates.rs`
- Modify: `crates/rofd-render/src/display_list.rs`
- Modify: `crates/rofd-render/tests/display_list.rs`

- [ ] **Step 1: Write failing template-order tests**

Construct `Document.xml` with two `CommonData/TemplatePage` entries (`ID`, `BaseLoc`, default `ZOrder`) and a page with `Template` references. Assert effective content order is background templates, page Background/Body/Foreground layers, then foreground templates. Explicit page-reference `ZOrder` overrides the template default.

- [ ] **Step 2: Confirm templates are ignored**

Run: `cargo test -p rofd-core --test templates`

Expected: FAIL because `Page::layers()` contains only the page's direct layers.

- [ ] **Step 3: Index and lazily load templates**

Resolve every template `BaseLoc` relative to `Document.xml`, reject duplicate template IDs, detect reference cycles with an active-ID stack, and cache parsed template pages. Represent source as `LayerSource::{Template(u64), Page}` while preserving each source's XML layer order.

- [ ] **Step 4: Merge effective display order**

Have `Page::layers()` return the effective immutable sequence. Unknown template IDs and cycles are `InvalidStructure`; invalid `ZOrder` is `InvalidValue`; templates default to Background when neither declaration nor reference specifies `ZOrder`.

- [ ] **Step 5: Verify ordering and safety**

Run:

```bash
cargo test -p rofd-core --test templates
cargo test -p rofd-render --test display_list template
cargo test -p rofd-core
```

Expected: all PASS, including missing-template and cycle cases.

- [ ] **Step 6: Commit**

```bash
git add crates/rofd-core crates/rofd-render
git commit -m "feat(core): resolve template page layers"
```

### Task 8: Add fixture regression, docs, and CI gates

**Files:**
- Create: `crates/rofd-render/tests/real_fixture.rs`
- Modify: `crates/rofd-render/README.md`
- Modify: `README.md`
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: Add a real-fixture display-list regression**

Open `learning/test.ofd`, build page 0's display list, assert the expected page size and a stable nonzero count of `DrawPath` commands, and assert diagnostics explicitly report the text/image objects deferred to phase 3. Render the partial page to an image surface and assert at least one known invoice rule pixel is non-background.

- [ ] **Step 2: Run the regression and record the exact stable counts**

Run: `cargo test -p rofd-render --test real_fixture -- --nocapture`

Expected: PASS after replacing broad `> 0` assertions with observed exact command/diagnostic counts that are semantically stable.

- [ ] **Step 3: Document the new boundary and known limits**

Document `DisplayList::from_page`, `CairoRenderer::render_page`, supported path/clip/template features, and the explicit phase-3 limitation for text and images. Update the root status from core foundation to partial rendering migration.

- [ ] **Step 4: Extend CI without pulling GUI dependencies**

Run these jobs in `.github/workflows/rust.yml`:

```bash
cargo fmt --all -- --check
cargo clippy -p rofd-core -p rofd-render --all-targets -- -D warnings
cargo test -p rofd-core -p rofd-render
```

- [ ] **Step 5: Run the complete phase quality gate**

Run:

```bash
cargo fmt --all -- --check
cargo clippy -p rofd-core -p rofd-render --all-targets -- -D warnings
cargo test -p rofd-core -p rofd-render
cargo tree -p rofd-core | rg 'cairo|qmetaobject|rofd-render'
```

Expected: format, Clippy, and tests PASS; the dependency search prints nothing and exits 1.

- [ ] **Step 6: Commit**

```bash
git add README.md .github/workflows/rust.yml crates/rofd-render
git commit -m "docs: describe basic rendering support"
```
