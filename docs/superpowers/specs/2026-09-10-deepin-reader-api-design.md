# deepin-reader integration APIs

Approved scope: implement and commit provider-side rofd changes in priority order.
deepin-reader is a read-only consumer reference. Build outputs and worktrees stay
inside the repository; no Debian package build is required.

## P0: pixel viewport rendering

Keep the published full-page API and `clip_mm` semantics. Add a versioned
`rofd_pixel_rect_t { uint32_t struct_size; int32_t x, y, width, height; }` and
`rofd_renderer_render_page_region_cairo(renderer, page, cairo, options,
viewport, report, error)`. Coordinates describe the final rotated/scaled full
pixel canvas; the viewport origin maps to target (0, 0). Reject nonpositive,
negative, overflowing, or out-of-canvas viewports. Do not silently clamp.

Add `rofd_renderer_get_pixel_canvas_size` with the same arguments as
`rofd_renderer_get_pixel_size`. It validates geometry within positive i32
dimensions but does not require a full-page Cairo allocation or full-page raster
budget. Existing size/render calls retain their validation behavior. All
region working surfaces and clip masks use viewport dimensions. The optional
millimetre clip remains in absolute physical-page coordinates. The target is at
least viewport-sized; writes are restricted to the viewport. Restore caller
graphics state and path, preserve diagnostic reporting and image/font services.
Decoded source images remain subject to the existing bounded decoder/cache
contracts; region rendering does not promise partial source-image decoding.

Acceptance: exact full-page vs stitched-tile pixels at four rotations,
fractional DPI/scale, nonzero page origins, clips, transparency and images;
large canvas with small budget successfully renders a small tile; invalid
viewport/target and FFI overlap tests. Rendering traverses the page display list,
but never creates a full-page raster before cropping.

## P1: metadata and warnings

Parse Keywords in core and expose metadata as an independently owned immutable
snapshot with borrowed UTF-8 accessors: DocID, title, author, subject, abstract,
creator/version, creation/modification strings, and ordered keywords. Preserve
missing values rather than fabricate dates or an ID. Do not hash a file as DocID.
Warnings are independently owned snapshots of warnings known at query time;
lazy page operations may add warnings, requiring a fresh snapshot. Stable codes,
package paths and messages are accessible after the document is released.

## P1: outline

Parse nested OutlineElem, title, Expanded (default true), and ordered Actions.
Ignore advisory Count in favor of real topology. Resolve PageID and named
bookmarks to document page indices. Model destinations with optional left, top,
right, bottom and zoom plus destination mode. Expose an owned immutable preorder
list with parent/first-child/next-sibling indices, expansion and action queries.
Use an explicit no-index sentinel. Bound nesting and counts using core limits.

## P2: page links

Expose a page-level owned snapshot with one or more page-space millimetre
rectangles and ordered actions per link. Collect actions on supported page and
graphic units (including annotation appearance), not merely Type=Link labels.
Explicit Action Region shapes produce conservative rectangular hit regions;
otherwise use the containing object's transformed boundary or physical page.
Preserve event type, internal destinations, URI/base, attachment identity,
file URI, and unknown action names. Queries never execute an action or open a
file/network destination. Unknown actions remain inspectable with warnings.
Reuse the outline action model. Preserve ordering for deterministic hit testing.

## ABI and delivery

All additions preserve v1 symbols/records/SONAME, struct_size rules, independent
handle lifetimes, transactional outputs, overlap preflight, panic containment,
bounded untrusted input and exact status mapping. Add C11/C++17 dynamic consumer
tests and symbol allowlist entries. Each numbered priority item gets a focused
Conventional Commit after build, format, relevant/full tests, Clippy and C ABI
checks. Do not push or tag as part of this feature task.

## Source references

- Existing `crates/rofd-render/src/cairo_renderer.rs` and FFI safety helpers.
- Existing semantic API design: `2026-09-09-poppler-style-c-semantic-api-design.md`.
- ofdrw official core models: `basicStructure/outlines/CT_OutlineElem.java`,
  `action/CT_Action.java`, `action/actionType/actionGoto/CT_Dest.java`,
  `action/actionType/GotoA.java`, `action/actionType/URI.java`, under
  https://github.com/ofdrw/ofdrw/tree/master/ofdrw-core/src/main/java/org/ofdrw/core.
