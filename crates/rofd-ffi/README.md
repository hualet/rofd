# rofd C ABI v1

`rofd-ffi` exposes the reusable OFD reader and Cairo renderer through the
versioned header [`include/rofd.h`](include/rofd.h). Include that header and
link with `rofd_ffi` plus Cairo. Check `rofd_abi_version()` against
`ROFD_ABI_VERSION` before using an ABI whose version is not already known by
the application.

## Page link mappings

`rofd_page_get_links` returns an independently owned immutable snapshot, including
an empty non-null list for pages without links. Query its count, then each link's
region/action counts and indexed values. One source Action currently produces
one link entry with one action and one or more region rectangles. Same-area
actions remain separate entries in deterministic source/effective content order.
Root actions precede content, objects precede children and visible annotations
follow page content; actions are not restricted to Link annotation labels.

Regions use absolute physical-page millimetres, with applicable object/ancestor
transforms already applied. Explicit Region Areas become conservative bounding
rectangles, not exact hit-test paths; rendering DPI/rotation are not applied.
Fallback object Boundary is transformed only by the parent, not by the object's
CTM twice. Invalid explicit regions fail in strict mode or skip that link with a
warning in lenient mode; they never silently become whole-page links.

Action/destination queries reuse `rofd_action_t` and `rofd_destination_t`, with
the same presence flags, versioning and transactional output rules as outlines.
Any invalid link/region/action index returns `PAGE_OUT_OF_RANGE`; ordinary count
and rectangle failures zero the output. All snapshot strings/regions survive
page/document free, but strings expire when `rofd_link_list_free` releases it.
First query may add warnings. No action is executed, including `file:` URIs and
attachments; consumers should filter CLICK events for mouse hit testing.

## Outline and navigation actions

`rofd_document_get_outline` returns an independently owned preorder snapshot.
An absent outline produces an empty snapshot. Node records expose a UTF-8 title,
expanded state (default true), action count and parent/first-child/next-sibling
indices; `ROFD_NO_INDEX` means no relation. Source `Count` values do not determine
the actual tree. All indices are zero-based.

Query each node's ordered actions with `rofd_outline_get_action`. The shared
`rofd_action_t` distinguishes internal Goto, URI and attachment GotoA, preserving
unknown action/event names. URI bases and attachment identifiers remain raw
strings; queries never execute actions, fetch a URI or open an attachment.

For Goto, `rofd_outline_get_action_destination` returns a versioned destination
record. It resolves OFD PageID or a named bookmark into a page index. Check
`ROFD_DESTINATION_HAS_PAGE_INDEX` before navigating: broken references use
`ROFD_NO_INDEX`, not page zero. An unresolved bookmark may have no destination
fields. Asking a non-Goto action for a destination returns `UNSUPPORTED`.
Other `HAS_*` bits distinguish absent coordinates/zoom from explicit zero.
Coordinates are absolute physical-page millimetres, independent of rendering
DPI, scale and rotation; the caller applies destination-mode defaults. Explicit
zero zoom means retain the current zoom.

Initialize every node/action/destination output's `struct_size` with `sizeof`.
Queries clear the known record prefix (including padding) on ordinary failure,
preserve that declared size and leave unknown tails untouched. Strings are
borrowed until `rofd_outline_free`, even if the source document is already freed.
The usual input/output overlap, error and thread-safety contracts apply.

Navigation semantics are parsed lazily and use the document's resource limits.
Strict malformed navigation fails the outline query; lenient recovery reports
warnings while retaining usable entries. Unknown action/event/mode names are
inert and inspectable. Normative Dest attributes take precedence; ofdrw-style
numeric child elements are accepted with a compatibility warning. Global XML
well-formedness and depth checks still occur when opening the document.
The exact historical namespace `http://www.ofdspec.org` is also accepted with a
compatibility warning; unrelated extension namespaces cannot impersonate OFD.
Retained XML strings and expanded navigation/diagnostic strings have independent
byte budgets, including repeated bookmark targets and namespace names. Exceeding
a budget fails the query in both modes without publishing a partial snapshot.

The C ABI runner also generates a small controlled OFD under its own target
directory and dynamically checks a nonempty tree, destinations, action payloads,
record tails and ownership. It does not need an external ZIP command or Python.

## Metadata and parse warnings

`rofd_document_get_metadata` returns an independently owned immutable snapshot.
The nine string getters borrow UTF-8 values from that snapshot: a missing field
returns `NULL`, while an explicitly empty field returns a non-NULL empty string.
Dates retain their producer representation. Keywords preserve declaration
order, repeated values and empty entries; access them with
`rofd_metadata_get_keyword_count` and `rofd_metadata_get_keyword`.

`rofd_document_get_warnings` copies only warnings collected so far. It does not
load pages or force lazy parsing. A snapshot taken before the first page query
can therefore be empty even when a later snapshot contains a page-area fallback
warning. Existing snapshots remain unchanged. These are parsing warnings;
rendering diagnostics remain available through the rendering report API.

```c
rofd_metadata_t *metadata = NULL;
rofd_warning_list_t *warnings = NULL;
if (rofd_document_get_metadata(document, &metadata, NULL) == ROFD_STATUS_OK) {
    const char *title = rofd_metadata_get_title(metadata);
    printf("title: %s\n", title ? title : "(missing)");
}
if (rofd_document_get_warnings(document, &warnings, NULL) == ROFD_STATUS_OK) {
    size_t count = 0;
    if (rofd_warning_list_get_count(warnings, &count, NULL) == ROFD_STATUS_OK) {
        for (size_t i = 0; i < count; ++i) {
            rofd_warning_t warning = {0};
            warning.struct_size = sizeof(warning);
            if (rofd_warning_list_get_warning(warnings, i, &warning, NULL)
                    == ROFD_STATUS_OK)
                fprintf(stderr, "%u %s: %s\n", warning.code,
                        warning.path, warning.message);
        }
    }
}
rofd_metadata_free(metadata);
rofd_warning_list_free(warnings);
```

Both snapshot types remain usable after document and page handles are freed.
Borrowed strings must not be modified or separately freed, and remain valid
until their snapshot is freed. Embedded NULs are displayed as `\0` so C string
consumers retain the full value. Warning codes are stable `ROFD_WARNING_*`
constants; future unknown categories map to `ROFD_WARNING_UNKNOWN` (0).

Warning output records require an initialized `struct_size`. Queries clear the
known prefix including padding and restore that size, preserving unknown tail
bytes. Invalid indices use `ROFD_STATUS_PAGE_OUT_OF_RANGE`. Required NULL
arguments fail with `ROFD_STATUS_INVALID_ARGUMENT`; ordinary failures null
handle/string outputs or zero counts and valid record prefixes. Malformed
address layouts preserve every output. Overlaps preserve ordinary outputs and
may publish an error only to a separate valid error slot. These APIs keep the
existing ABI version and layouts; no GUI types enter the core metadata API.

## Text semantic C example

Page text, layout, and search use independently owned result handles. This
example opens page zero, prints its canonical UTF-8 text, and reports the first
whole-word match for a query supplied on the command line:

```c
#include <rofd.h>
#include <stddef.h>
#include <stdio.h>

int main(int argc, char **argv) {
    rofd_document_t *document = NULL;
    rofd_page_t *page = NULL;
    rofd_string_t *text = NULL;
    rofd_text_layout_t *layout = NULL;
    rofd_text_search_t *search = NULL;
    rofd_error_t *error = NULL;
    rofd_find_options_t options;
    rofd_text_match_t match = {0};
    size_t character_count = 0;
    size_t match_count = 0;
    int result = 1;

#define ROFD_TRY(call) do {                                                \
    if ((call) != ROFD_STATUS_OK) {                                        \
        fprintf(stderr, "rofd: %s\n", error ? rofd_error_get_message(error) \
                                             : "unknown error");           \
        goto cleanup;                                                      \
    }                                                                      \
} while (0)

    if (argc != 3 || rofd_abi_version() != ROFD_ABI_VERSION)
        goto cleanup;
    ROFD_TRY(rofd_document_open(argv[1], NULL, &document, &error));
    ROFD_TRY(rofd_document_get_page(document, 0, &page, &error));
    ROFD_TRY(rofd_page_get_text(page, &text, &error));
    ROFD_TRY(rofd_page_get_text_layout(page, &layout, &error));
    ROFD_TRY(rofd_text_layout_get_count(layout, &character_count, &error));

    rofd_find_options_init(&options, sizeof(options));
    options.flags = ROFD_FIND_WHOLE_WORDS;
    ROFD_TRY(rofd_page_find_text_with_options(page, argv[2], &options,
                                               &search, &error));
    ROFD_TRY(rofd_text_search_get_count(search, &match_count, &error));
    if (match_count != 0) {
        match.struct_size = sizeof(match);
        ROFD_TRY(rofd_text_search_get_match(search, 0, &match, &error));
        printf("first match: byte %zu, length %zu\n", match.utf8_offset,
               match.utf8_length);
    }
    fwrite(rofd_string_get_data(text), 1, rofd_string_get_length(text), stdout);
    printf("\n%zu characters, %zu matches\n", character_count, match_count);
    result = 0;

cleanup:
    rofd_error_free(error);
    rofd_text_search_free(search);
    rofd_text_layout_free(layout);
    rofd_string_free(text);
    rofd_page_free(page);
    rofd_document_free(document);
    return result;
}
```

Rust consumers should call `rofd_core::Page::text()` and use the idiomatic
`PageText`, `FindOptions`, `TextMatch`, and `TextSelection` values directly;
the opaque handle model exists only to provide stable ownership across the C
ABI.

## Complete C example

```c
#include <rofd.h>
#include <cairo.h>
#include <stdint.h>
#include <stdio.h>

int main(int argc, char **argv) {
    rofd_document_t *document = NULL;
    rofd_page_t *page = NULL;
    rofd_renderer_t *renderer = NULL;
    rofd_render_report_t *report = NULL;
    rofd_error_t *error = NULL;
    cairo_surface_t *surface = NULL;
    cairo_t *cr = NULL;
    rofd_load_options_t load_options;
    rofd_renderer_options_t renderer_options;
    rofd_render_options_t render_options;
    int32_t width = 0, height = 0;
    int result = 1;

    if (argc != 2 || rofd_abi_version() != ROFD_ABI_VERSION)
        goto cleanup;

    rofd_load_options_init(&load_options, sizeof(load_options));
    rofd_renderer_options_init(&renderer_options, sizeof(renderer_options));
    rofd_render_options_init(&render_options, sizeof(render_options));

#define ROFD_TRY(call) do {                                                \
    if ((call) != ROFD_STATUS_OK) {                                        \
        fprintf(stderr, "rofd: %s\n", error ? rofd_error_get_message(error) \
                                             : "unknown error");           \
        goto cleanup;                                                      \
    }                                                                      \
} while (0)

    ROFD_TRY(rofd_document_open(argv[1], &load_options, &document, &error));
    ROFD_TRY(rofd_document_get_page(document, 0, &page, &error));
    rofd_document_free(document);
    document = NULL; /* page remains valid */

    ROFD_TRY(rofd_renderer_new(&renderer_options, &renderer, &error));
    ROFD_TRY(rofd_renderer_get_pixel_size(renderer, page, &render_options,
                                           &width, &height, &error));
    surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, width, height);
    cr = cairo_create(surface);
    if (cairo_status(cr) != CAIRO_STATUS_SUCCESS)
        goto cleanup;
    ROFD_TRY(rofd_renderer_render_page_cairo(renderer, page, cr,
                                              &render_options, &report, &error));

    size_t count = 0;
    ROFD_TRY(rofd_render_report_get_count(report, &count, &error));
    for (size_t i = 0; i < count; ++i) {
        rofd_render_diagnostic_t diagnostic = {0};
        diagnostic.struct_size = sizeof(diagnostic);
        ROFD_TRY(rofd_render_report_get_diagnostic(report, i, &diagnostic,
                                                    &error));
        fprintf(stderr, "diagnostic %u: %s\n", diagnostic.kind,
                diagnostic.message); /* borrowed from report */
    }
    result = 0;

cleanup:
    rofd_error_free(error);
    rofd_render_report_free(report);
    if (cr) cairo_destroy(cr);
    if (surface) cairo_surface_destroy(surface);
    rofd_renderer_free(renderer);
    rofd_page_free(page);
    rofd_document_free(document);
    return result;
}
```

For example, after building the shared library, compile with
`cc reader.c -Icrates/rofd-ffi/include -Ltarget/debug -lrofd_ffi $(pkg-config --cflags --libs cairo)`.

## Pixel viewport rendering

For tiles, call `rofd_renderer_get_pixel_canvas_size` to get the final rotated,
scaled full-page pixel dimensions. It is independent of the allocation needed
for an entire page; the original `get_pixel_size` retains its full-target limits.
Intersect a requested slice with that canvas in the application, then initialize
`rofd_pixel_rect_t` with `rofd_pixel_rect_init` and set x/y/width/height.

Allocate only a viewport-sized Cairo surface and call:

```c
rofd_renderer_render_page_region_cairo(renderer, page, cr, &render_options,
                                        &viewport, &report, &error);
```

The viewport's full-page pixel origin maps to target (0, 0). Invalid or
out-of-canvas viewports are rejected, not clamped. `clip_mm` remains an absolute
page-space content clip; it does not control tile positioning. Optional render
reports and caller Cairo ownership follow the full-page API.

Region rendering uses tile-sized target, mask and intermediate surfaces. Page
display-list traversal and bounded source-image decoding still occur; this API
does not promise partial image decoding or spatially indexed object traversal.
Integer pixel origins preserve the full-page sample grid. Cairo may compute
slightly different antialiased curve-edge coverage on differently sized
targets, so universal byte equality with a full-page crop is not guaranteed.
Tests assert exact geometric/image stitching and separately bound curve-edge
and real-invoice differences in both magnitude and count.

A valid canvas size does not guarantee every source geometry is representable
by Cairo. Region rendering bounds page clips and axis-aligned filled/image
rectangles to the tile; unsafe extreme path coordinates return
`ROFD_STATUS_RENDER_ERROR` instead of reporting success with incorrect pixels.

## Consumer contract

- Returned handles are caller-owned and use their matching `rofd_*_free`;
  every free function accepts `NULL`. Pages remain usable after their document
  is freed. A Cairo context is borrowed for a render call and is never consumed.
- Input strings are NUL-terminated UTF-8 and stay readable for the call. The
  library version string has static storage. Error messages and diagnostic
  messages are borrowed until their owning error or report is freed.
- Initialize every options record with `rofd_*_options_init(&value,
  sizeof(value))`, then change fields. `struct_size` is the initialized version
  boundary, not arbitrary capacity; appended fields preserve old boundaries.
  An undersized initializer is a no-op, and unsupported boundaries are rejected.
- `NULL` load/render options select defaults. For renderer fallback, `NULL`
  options and `{ NULL, 0 }` select built-ins; `{ non-NULL, 0 }` disables
  fallback; `{ non-NULL, count > 0 }` supplies UTF-8 family names. `{ NULL,
  count > 0 }` is invalid.
- Non-`NULL` input and output regions for one call must not alias. Read-only
  calls may share live handles across threads, but no handle may be freed while
  in use. Cairo access and synchronization remain the caller's responsibility.

The v1 surface intentionally defers image mappings, richer annotation
and signature queries, progressive rendering, callbacks,
custom font providers, non-Cairo backends, and advanced composite or color-space
controls.
