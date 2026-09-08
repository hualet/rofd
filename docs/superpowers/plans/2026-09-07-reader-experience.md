# Reader Experience Implementation Plan

> Execute in the existing isolated `hualet/rofd-qt-reader` worktree using subagent-driven-development; user approved the design and implementation on 2026-09-07.

**Goal:** Deliver continuous reading, zoom, navigation, thumbnails and text search in a polished Qt desktop reader.

**Architecture:** Rust worker owns document parsing, Cairo rendering and bounded session cache. QObject bridge exposes JSON metadata and queued results; QML manages viewport, fit modes and selection. Core remains GUI independent.

**Tech Stack:** Rust, qmetaobject, Cairo, Qt 6 Quick Controls, serde_json, base64.

## Tasks

- [x] Add pure search and raster sizing tests in `src/bin/rofd/reader_tests.rs`, observe failures, implement `reader.rs`: source-ordered recursive text traversal, Unicode search across runs, object-boundary results, capped matches, raster size limits and cache eviction tests.
- [x] Replace `viewer.rs` with a GUI-thread bridge and `worker.rs` background session. Test failed replacement preserves document, cancellation tokens discard stale open/search/render, retry behavior, and owned raster lifetimes. `poll()` drains worker messages; no Qt objects cross threads.
- [x] Build QML components in `src/bin/rofd/ui/`: theme, toolbar, sidebar, continuous viewport and reusable page card. Register resources. Wire shortcuts, fit modes, horizontal/vertical scrolling, active page synchronization, search highlights and asynchronous image replies.
- [x] Integrate and run `cargo test -p rofd --features qt-reader`, `cargo test -p rofd-core`, `cargo fmt --all -- --check`, `cargo clippy -p rofd-core --all-targets -- -D warnings`, `cargo clippy -p rofd --features qt-reader --all-targets -- -D warnings` and `cargo build -p rofd --features qt-reader`.
- [x] Run actual app with repository fixture and generated multi-page OFD; exercise navigation, resize, zoom, search, failure recovery. Capture screenshots and fix observed issues. Review design compliance and code quality before delivery. Leave changes uncommitted unless requested.

## QML / bridge contract

Read-only properties: `pages_json` (array of `{width,height,error}`, logical px at 96 DPI), `document_title`, `page_count`, `busy`, `error_message`, `search_json` (array of `{page,snippet,x,y,width,height}`, normalized page coordinates), `search_status`, `search_busy`. `current_page` is read/write zero-based, clamped by backend. `generation` increments only on successful document replacement.

Methods: `open_file(QString)`, `poll()`, `request_page(i32, f64, bool)` (page, device render scale, thumbnail), `search(QString)`, `clear_error()`. `request_page` returns a render ticket; `cancel_render(i32)` cancels a superseded/offscreen request. Signal: `render_ready(QString)` JSON `{page,scale,thumbnail,source,error,reduced}`. QML must discard old-scale render replies; bridge discards old-document replies. `poll()` timer runs every 30ms. Render scale is rounded to hundredths for cache keys. Request only visible pages and immediate neighbors; release offscreen image sources. Retry requests are allowed after failure.

QML uses `generationChanged` to reset viewport and query; `pages_json` changes with successful open. Search matching is case insensitive Unicode lowercase, source-order, across runs within an object. Results loop locally and use normalized object boundaries for highlighter/scroll target. UI labels explicitly say text-region highlight where useful.

## Verification outcome

Core: 177 tests pass. Affected root package: 17 tests pass (15 reader + 2 legacy). QML: 17 pass. Formatting, diff whitespace, and strict reusable-crate Clippy pass. Reader Clippy adds no warnings; the unchanged legacy root library has 17 baseline warnings in `src/render.rs` and `src/types.rs`, so the root `-D warnings` command remains blocked by that pre-existing baseline. Screenshots and interactive checks documented in `docs/reader.md`.
