# Cairo Positioned Glyph Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render validated backend-neutral positioned glyph runs with Cairo and keep image drawing deferred.

**Architecture:** Create Cairo font faces through a crate-private `ResolvedFont` bridge and cache them only for one interpreter run, keyed by live font allocation identity and collection face. Validate and batch a complete glyph run before painting, then render font-backed segments or deterministic missing-glyph boxes through the same clipped intermediate pipeline used by paths.

**Tech Stack:** Rust 2021, cairo-rs safe FreeType integration, freetype-rs, controlled TTF fixtures, pixel integration tests.

---

### Task 1: Establish positioned-text pixel behavior

**Files:**
- Create: `crates/rofd-render/tests/cairo_text.rs`
- Modify: `crates/rofd-render/tests/support/mod.rs` only if shared fixture helpers are needed

- [ ] Add package builders and pixel-bound helpers using `phase3-subset.ttf`.
- [ ] Add failing tests for embedded Latin/CJK baselines, multiple runs, explicit/inferred deltas, and fill/stroke/alpha/style.
- [ ] Run `cargo test -p rofd-render --test cairo_text` and record the expected `UnsupportedDisplayCommand::GlyphRun` failures.

### Task 2: Add safe render-local font faces and glyph batching

**Files:**
- Modify: `crates/rofd-render/src/fonts.rs`
- Modify: `crates/rofd-render/src/cairo_renderer.rs`
- Modify: `crates/rofd-render/src/lib.rs` only for structured errors

- [ ] Add crate-private constant-size font allocation identity and safe Cairo `FontFace` construction backed by owned FreeType face data.
- [ ] Permit text in page/command preflight while continuing to reject images before display-list construction.
- [ ] Validate the entire run against the page glyph limit and finite numeric requirements before painting.
- [ ] Batch consecutive glyphs by font allocation/face and compatible transform; build Cairo glyph vectors with checked index conversion.
- [ ] Render fill-only batches with `show_glyphs`; render stroked batches with `glyph_path` followed by path-equivalent fill/stroke sequencing.
- [ ] Render missing glyphs as deterministic baseline-relative boxes.
- [ ] Run the focused tests until green without weakening assertions.

### Task 3: Preserve transforms, clips, rotations, diagnostics, and state

**Files:**
- Modify: `crates/rofd-render/tests/cairo_text.rs`
- Modify: `crates/rofd-render/src/cairo_renderer.rs`

- [ ] Add failing tests for noncommuting object transforms, object clips, all page rotations, CG glyph IDs, controlled fallback, and synthetic missing boxes.
- [ ] Refactor clipped drawing into a generic operation so glyphs and boxes use the same A8 union/intersection mask as paths.
- [ ] Add a failing backend/invalid-font test that checks caller matrix, line width, font face/matrix/options, current path, operator, antialias, and tolerance restoration.
- [ ] Implement balanced cleanup and structured safe-identity errors; rerun focused and regression tests.

### Task 4: Documentation, verification, and delivery

**Files:**
- Modify: `crates/rofd-render/README.md`

- [ ] Document positioned glyph rendering, mixed fallback, synthetic boxes, and deferred images.
- [ ] Run formatting, focused text/render/display/fixture tests, core/render package tests, strict Clippy, workspace all-target tests, dependency isolation, and `git diff --check`.
- [ ] Self-review the diff for unsafe code, host-path leakage, per-glyph face creation, unbounded caches, image behavior changes, and missing required coverage.
- [ ] Commit the focused result as `feat(render): draw positioned OFD glyphs`.
