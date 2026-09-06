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
 * All input strings are UTF-8. The string returned by
 * rofd_library_version() has static storage and must not be freed. A
 * diagnostic message is borrowed from its report and remains valid only
 * until that report is freed. A cairo_t passed for rendering is borrowed;
 * ownership remains with the caller.
 *
 * Every options record begins with struct_size. Call the matching initializer
 * before changing fields. The library uses struct_size to determine which
 * fields are present, allowing trailing fields to be added compatibly in
 * later ABI versions.
 *
 * Read-only calls on live handles may run concurrently. Callers must ensure
 * that no handle is freed while another call is using it. Cairo context
 * access and synchronization remain the caller's responsibility under
 * Cairo's threading rules.
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

void rofd_load_options_init(rofd_load_options_t *options);
void rofd_renderer_options_init(rofd_renderer_options_t *options);
void rofd_render_options_init(rofd_render_options_t *options);

rofd_status_t rofd_document_open(const char *path,
                                 const rofd_load_options_t *options,
                                 rofd_document_t **document,
                                 rofd_error_t **error);
rofd_status_t rofd_document_get_page_count(const rofd_document_t *document,
                                           size_t *page_count,
                                           rofd_error_t **error);
rofd_status_t rofd_document_get_page(const rofd_document_t *document,
                                     size_t page_index,
                                     rofd_page_t **page,
                                     rofd_error_t **error);
void rofd_document_free(rofd_document_t *document);

rofd_status_t rofd_page_get_index(const rofd_page_t *page,
                                  size_t *page_index,
                                  rofd_error_t **error);
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
