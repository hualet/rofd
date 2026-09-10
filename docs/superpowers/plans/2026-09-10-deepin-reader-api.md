# deepin-reader API Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans or superpowers:subagent-driven-development task-by-task, with specification and quality reviews.

**Goal:** Supply the four provider-side APIs required by deepin-reader.

**Architecture:** Add core semantic models and immutable owned C snapshots. Split
pixel canvas geometry from viewport allocation in the renderer, preserving v1.

**Tech Stack:** Rust 2021, Cairo, bounded XML/ZIP parsing, C11/C++17.

## Task 1: P0 region rendering

Files: `crates/rofd-render/src/cairo_renderer.rs`, `src/lib.rs`,
`tests/{cairo_render,cairo_stamp,cairo_text}.rs`,
`crates/rofd-ffi/src/{abi,region,renderer,lib}.rs`, `include/rofd.h`,
`tests/region.rs`, `tests/c/*`, crate READMEs.

- [x] Add failing Rust tests using `PixelRect { x: 17, y: 9, width: 31, height: 23 }`,
  `CairoRenderer::pixel_canvas_size` and `render_page_region`; compare every RGBA
  byte with the corresponding full-page pixel, including four rotations, clips,
  nonzero origins, image/text/clipping fixtures and small budgets on huge pages.
- [x] Run targeted renderer/FFI tests; confirm missing API and high-zoom stamp failure.
- [x] Implement positive i32 canvas geometry separately from Cairo-limited target
  geometry. Construct target matrix by subtracting integer viewport x/y from its
  translation. Use viewport dimensions for all masks/intermediates and budgets.
- [x] Add versioned FFI viewport initializer, input-range preflight and two entry
  points: canvas size and region rendering (including report output). Use existing
  HandleOutput/ScalarOutput and boundary_with_inputs conventions.
- [x] Run region/FFI tests. Add C/C++ record/function probes and real-fixture tile
  comparison; update the exported-symbol allowlist and public contracts.
- [x] Run format, workspace tests, strict core/render/ffi Clippy, release FFI/Qt
  build and `crates/rofd-ffi/tests/run_c_tests.sh`; review and commit
  `feat(render): add pixel viewport rendering`.

P0 verification: workspace/all-targets 488 passed, 0 failed, 2 intentional
ignores; strict Clippy, format, release FFI/Qt build and C11/C++17 dynamic ABI
runner passed. Tests are grouped in the existing cairo_render/cairo_stamp/
cairo_text files rather than a separate cairo_region module, reusing fixtures.
Review regressions cover 20M/2B canvas coordinates, bounded page/path/image
rectangles, and explicit rejection of unsafe arbitrary Cairo paths.

## Task 2: P1 metadata and warnings

Files: core `src/{raw,document}.rs`, core metadata tests; FFI
`src/{metadata,handles,abi,lib}.rs`, header, metadata tests, C consumers, READMEs.

- [ ] Add a package with all DocInfo fields and `<Keywords><Keyword>one</Keyword>
  <Keyword>二</Keyword></Keywords>`; assert `document.metadata().keywords` preserves
  order. Test missing fields and lazy warning snapshot growth.
- [ ] Run core tests to confirm missing keywords behavior; implement raw and
  public metadata field using existing bounded XML parsing.
- [ ] Add owned metadata/warning list handles with complete NULL/empty/lifetime,
  out-of-range, record-tail and output-overlap tests before implementation.
- [ ] Implement explicit field accessors, list counts and indexed queries using
  existing FFI lifetime/error helpers; document missing versus empty values.
- [ ] Verify build/tests/Clippy/C consumers; review and commit
  `feat(ffi): expose document metadata and warnings`.

## Task 3: P1 outlines and shared actions

Files: core `src/{navigation,raw,document,lib}.rs`, navigation tests; FFI
`src/{navigation,handles,abi,lib}.rs`, header, tests and C consumers.

- [ ] Add nested outline fixtures with Expanded=false, advisory Count mismatch,
  direct Dest and named Bookmark, absent/broken destinations and unknown actions.
  Assert actual topology, page-index resolution, optional coordinates and limits.
- [ ] Run failing tests; implement bounded shared action/destination parsing and
  outline flattening. Keep unsupported action type names and warning evidence.
- [ ] Add C snapshot queries and tests for ownership after document free,
  sentinel relations, destination validity, output transactions and record tails.
- [ ] Verify build/tests/Clippy/C consumers; review and commit
  `feat(core): expose document outlines and destinations`.

## Task 4: P2 page link mappings

Files: core `src/{navigation,raw,document,page}.rs` as existing module layout
requires; FFI navigation module/header; fixture, Rust and C consumer tests.

- [ ] Add tests for page/object/annotation actions, explicit multi-area Region,
  translated/transformed fallback boundary, internal/URI/attachment/unknown
  actions, action order and invalid coordinates. Assert non-execution.
- [ ] Run failing tests; implement page-space rectangle mapping and reuse shared
  action resolution. Snapshot owns all regions, strings and destinations.
- [ ] Add C count/index/region/action accessors, lifetime, invalid-index, NULL,
  overlap and consumer tests. Document conservative rectangle geometry.
- [ ] Verify build/tests/Clippy/C consumers; review and commit
  `feat(core): expose page link mappings`.

## Execution environment

Worktree: `.worktrees/deepin-reader-api`, branch `feat/deepin-reader-api`.
Use `CARGO_TARGET_DIR=/home/hualet/projects/hualet/rofd/target` and
`TMPDIR=/home/hualet/projects/hualet/rofd/target/tmp` for Cargo. The C runner uses
its repository-local `target/rofd-ffi-c-tests`. Do not build Debian packages.
