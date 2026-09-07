# rofd C ABI v1

`rofd-ffi` exposes the reusable OFD reader and Cairo renderer through the
versioned header [`include/rofd.h`](include/rofd.h). Include that header and
link with `rofd_ffi` plus Cairo. Check `rofd_abi_version()` against
`ROFD_ABI_VERSION` before using an ABI whose version is not already known by
the application.

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

The v1 surface intentionally defers annotations, signatures, text search and
selection, outline/metadata APIs, progressive rendering, callbacks, custom font
providers, non-Cairo backends, and advanced composite or color-space controls.
