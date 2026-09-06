#ifndef ROFD_H
#define ROFD_H

#include <cairo.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @file rofd.h
 * @brief Stable C ABI for the rofd OFD reader and renderer.
 *
 * Unless documented otherwise, every non-NULL handle returned through an
 * output parameter is owned by the caller and must be released with its
 * matching rofd_*_free function. All free functions are safe to call with
 * NULL. A page retains the document data it needs, so its document may be
 * freed before the page.
 *
 * Every non-NULL input C string must point to readable NUL-terminated UTF-8 bytes
 * that remain valid for the entire call. Individual functions specify whether
 * NULL or an empty string is allowed. The string returned by
 * rofd_library_version() has static storage and must not be freed. A
 * diagnostic message is borrowed from its report and remains valid only
 * until that report is freed. A cairo_t passed for rendering is borrowed;
 * ownership remains with the caller.
 *
 * Every options record begins with struct_size. Call the matching initializer
 * with the writable capacity of the record before changing fields. A NULL
 * pointer, or a capacity smaller than the oldest supported version boundary,
 * is a no-op. Otherwise the initializer clears and initializes the highest
 * complete supported version that fits, leaves later bytes unchanged, and sets
 * struct_size to that selected version boundary rather than caller capacity.
 *
 * Options records evolve only by appending fields. Every published version
 * boundary, including its tail padding, is permanent. New fields must start at
 * or after the previous boundary and must never reuse an older version's tail
 * padding. Thus a new library initializes the complete older prefix that an old
 * caller can hold, while an old library preserves a new caller's unknown tail.
 * Non-NULL options must be correctly aligned and readable through struct_size.
 * When struct_size declares a supported version boundary, the complete prefix
 * through that boundary must remain readable for the call.
 *
 * Read-only calls on live handles may run concurrently. Callers must ensure
 * that no handle is freed while another call is using it. Cairo context
 * access and synchronization remain the caller's responsibility under
 * Cairo's threading rules.
 *
 * All non-NULL output parameter locations supplied to one call must occupy
 * distinct, non-overlapping storage. Before writing any output, the library
 * checks address ranges for representable arithmetic, alignment, and overlap.
 * Detectable violations fail with ROFD_STATUS_INVALID_ARGUMENT. If ordinary
 * outputs overlap while error is a separate valid slot, ordinary outputs stay
 * untouched and error receives the failure. If error overlaps an ordinary
 * output, every output stays untouched and no error handle is published.
 * Readable input regions and live handle storage must not overlap any output
 * slot, including error. Input strings, options records, and handle storage
 * must remain valid for the entire call. They must not be concurrently
 * modified or freed.
 */

#define ROFD_ABI_VERSION 1u

typedef uint32_t rofd_status_t;

#define ROFD_STATUS_OK 0u
#define ROFD_STATUS_INVALID_ARGUMENT 1u
#define ROFD_STATUS_IO 2u
#define ROFD_STATUS_INVALID_DOCUMENT 3u
#define ROFD_STATUS_UNSUPPORTED 4u
#define ROFD_STATUS_LIMIT_EXCEEDED 5u
#define ROFD_STATUS_PAGE_OUT_OF_RANGE 6u
#define ROFD_STATUS_RENDER_ERROR 7u
#define ROFD_STATUS_OUT_OF_MEMORY 8u
#define ROFD_STATUS_INTERNAL 255u

#define ROFD_STRICTNESS_LENIENT 0u
#define ROFD_STRICTNESS_STRICT 1u

#define ROFD_IMAGE_INTERPOLATION_NEAREST 0u
#define ROFD_IMAGE_INTERPOLATION_BILINEAR 1u

#define ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT 1u
#define ROFD_DIAGNOSTIC_FONT_FALLBACK 2u
#define ROFD_DIAGNOSTIC_MISSING_GLYPH 3u
#define ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED 4u
#define ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED 5u
#define ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED 6u

typedef struct rofd_document rofd_document_t;
typedef struct rofd_page rofd_page_t;
typedef struct rofd_renderer rofd_renderer_t;
typedef struct rofd_render_report rofd_render_report_t;
typedef struct rofd_error rofd_error_t;

typedef struct rofd_load_options {
    uint32_t struct_size;
    uint32_t strictness;
} rofd_load_options_t;

/**
 * Renderer construction options.
 *
 * NULL options, or fallback_families == NULL and fallback_family_count == 0,
 * select built-in default families. Non-NULL fallback_families with a zero
 * count explicitly disables fallback. A count greater than zero requires a
 * valid array of non-NULL UTF-8 strings.
 */
typedef struct rofd_renderer_options {
    uint32_t struct_size;
    const char *const *fallback_families;
    size_t fallback_family_count;
    uint64_t max_font_bytes;
    uint64_t image_cache_bytes;
} rofd_renderer_options_t;

typedef struct rofd_render_options {
    uint32_t struct_size;
    double dpi;
    double scale;
    uint32_t rotation_degrees;
    uint32_t background_rgba;
    uint32_t has_clip;
    double clip_x_mm;
    double clip_y_mm;
    double clip_width_mm;
    double clip_height_mm;
    uint32_t image_interpolation;
    uint64_t max_raster_bytes;
} rofd_render_options_t;

typedef struct rofd_rect {
    double x_mm;
    double y_mm;
    double width_mm;
    double height_mm;
} rofd_rect_t;

typedef struct rofd_render_diagnostic {
    uint32_t struct_size;
    uint32_t kind;
    uint64_t object_id;
    const char *message;
} rofd_render_diagnostic_t;

uint32_t rofd_abi_version(void);
const char *rofd_library_version(void);

void rofd_load_options_init(rofd_load_options_t *options, size_t options_size);
void rofd_renderer_options_init(rofd_renderer_options_t *options,
                                size_t options_size);
void rofd_render_options_init(rofd_render_options_t *options,
                              size_t options_size);

/**
 * Open an OFD document. A NULL path is a defined input that returns
 * ROFD_STATUS_INVALID_ARGUMENT under the normal output transaction. An empty
 * path also returns ROFD_STATUS_INVALID_ARGUMENT. A non-NULL path must satisfy
 * the input-string contract above; options may be NULL or must satisfy the
 * options-record contract. Both readable inputs must be disjoint from document
 * and error output storage.
 */
rofd_status_t rofd_document_open(const char *path,
                                 const rofd_load_options_t *options,
                                 rofd_document_t **document,
                                 rofd_error_t **error);
/** Borrow a live document without consuming it; its storage must be disjoint
 * from page_count and error and must not be freed during the call. */
rofd_status_t rofd_document_get_page_count(const rofd_document_t *document,
                                           size_t *page_count,
                                           rofd_error_t **error);
/** Borrow a live document and return an independently owned page. The document
 * storage must be disjoint from page and error and remain live for the call. */
rofd_status_t rofd_document_get_page(const rofd_document_t *document,
                                     size_t page_index,
                                     rofd_page_t **page,
                                     rofd_error_t **error);
void rofd_document_free(rofd_document_t *document);

/** Borrow a live page without consuming it; its storage must be disjoint from
 * page_index and error and must not be freed during the call. */
rofd_status_t rofd_page_get_index(const rofd_page_t *page,
                                  size_t *page_index,
                                  rofd_error_t **error);
/** Borrow a live page without consuming it; its storage must be disjoint from
 * page_size and error and must not be freed during the call. */
rofd_status_t rofd_page_get_size_mm(const rofd_page_t *page,
                                    rofd_rect_t *page_size,
                                    rofd_error_t **error);
void rofd_page_free(rofd_page_t *page);

rofd_status_t rofd_renderer_new(const rofd_renderer_options_t *options,
                                rofd_renderer_t **renderer,
                                rofd_error_t **error);
rofd_status_t rofd_renderer_get_pixel_size(
    const rofd_renderer_t *renderer,
    const rofd_page_t *page,
    const rofd_render_options_t *options,
    int32_t *pixel_width,
    int32_t *pixel_height,
    rofd_error_t **error);
/**
 * Render a page into a borrowed Cairo context.
 *
 * The report output may be NULL when the caller wants to discard diagnostics.
 */
rofd_status_t rofd_renderer_render_page_cairo(
    const rofd_renderer_t *renderer,
    const rofd_page_t *page,
    cairo_t *cairo,
    const rofd_render_options_t *options,
    rofd_render_report_t **report,
    rofd_error_t **error);
void rofd_renderer_free(rofd_renderer_t *renderer);

rofd_status_t rofd_render_report_get_count(const rofd_render_report_t *report,
                                           size_t *diagnostic_count,
                                           rofd_error_t **error);
/**
 * Get one diagnostic from a render report.
 *
 * Before calling, set diagnostic->struct_size to
 * sizeof(rofd_render_diagnostic_t).
 */
rofd_status_t rofd_render_report_get_diagnostic(
    const rofd_render_report_t *report,
    size_t diagnostic_index,
    rofd_render_diagnostic_t *diagnostic,
    rofd_error_t **error);
void rofd_render_report_free(rofd_render_report_t *report);

rofd_status_t rofd_error_get_status(const rofd_error_t *error);
/**
 * Return a borrowed UTF-8 error message.
 *
 * The returned pointer must not be freed and remains valid only until error is
 * freed with rofd_error_free().
 */
const char *rofd_error_get_message(const rofd_error_t *error);
void rofd_error_free(rofd_error_t *error);

#ifdef __cplusplus
}
#endif

#endif
