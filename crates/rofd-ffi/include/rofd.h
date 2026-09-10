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
 * Every non-NULL input C string must point to the first byte of a valid readable
 * character array object containing UTF-8 bytes. The terminating NUL byte must
 * occur within that same object; its readable extent and byte length must be
 * representable by ptrdiff_t. The complete sequence must remain valid for the
 * entire call. Individual functions specify whether NULL or an empty string is
 * allowed. The string returned by
 * rofd_library_version() has static storage and must not be freed. A
 * diagnostic message is borrowed from its report and remains valid only
 * until that report is freed. A cairo_t passed for rendering is borrowed;
 * ownership remains with the caller.
 *
 * Semantic page queries return caller-owned opaque result handles. Borrowed
 * string data remains valid until its string or text-selection owner is freed.
 * A semantic result remains valid after the source page or document is freed.
 * Every input string and input record must remain readable for the complete
 * call and must be disjoint from every ordinary output and error output slot.
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
 * A non-NULL options pointer must be correctly aligned for its record type and
 * designate a valid object whose struct_size field is initialized and readable.
 * When struct_size declares the v1 boundary or a larger supported boundary, the
 * complete v1 prefix must be initialized and readable; every additional declared
 * supported prefix must likewise be initialized and readable for the call.
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

#define ROFD_FIND_CASE_SENSITIVE (1u << 0)
#define ROFD_FIND_WHOLE_WORDS (1u << 1)

#define ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR (1u << 0)
#define ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY (1u << 1)

#define ROFD_SELECTION_GLYPH 0u
#define ROFD_SELECTION_WORD 1u
#define ROFD_SELECTION_LINE 2u

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
typedef struct rofd_string rofd_string_t;
typedef struct rofd_text_layout rofd_text_layout_t;
typedef struct rofd_text_search rofd_text_search_t;
typedef struct rofd_text_selection rofd_text_selection_t;

typedef struct rofd_load_options {
    uint32_t struct_size;
    uint32_t strictness;
} rofd_load_options_t;

typedef struct rofd_find_options {
    uint32_t struct_size;
    uint32_t flags;
    size_t max_results;
} rofd_find_options_t;

/**
 * Renderer construction options.
 *
 * NULL options, or fallback_families == NULL and fallback_family_count == 0,
 * select built-in default families. Non-NULL fallback_families with a zero
 * count explicitly disables fallback. A count greater than zero requires a
 * pointer correctly aligned for const char * that designates the first element
 * of a single valid, initialized, readable array object containing at least
 * fallback_family_count elements. Its total extent must be representable by
 * ptrdiff_t. Every element must be non-NULL and satisfy the input C string
 * contract above.
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

typedef struct rofd_text_char {
    uint32_t struct_size;
    size_t utf8_offset;
    size_t utf8_length;
    rofd_rect_t rect_mm;
    uint32_t flags;
    uint64_t object_id;
} rofd_text_char_t;

typedef struct rofd_text_match {
    uint32_t struct_size;
    size_t utf8_offset;
    size_t utf8_length;
    rofd_rect_t rect_mm;
} rofd_text_match_t;

typedef struct rofd_render_diagnostic {
    uint32_t struct_size;
    uint32_t kind;
    uint64_t object_id;
    const char *message;
} rofd_render_diagnostic_t;

uint32_t rofd_abi_version(void);
const char *rofd_library_version(void);

void rofd_load_options_init(rofd_load_options_t *options, size_t options_size);
void rofd_find_options_init(rofd_find_options_t *options, size_t options_size);
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

/** Query canonical UTF-8 page text, including synthesized object separators.
 * The returned string is independently owned and survives freeing page and
 * document. Free it with rofd_string_free. Layout byte offsets index these
 * exact bytes. page and text are required; error is optional. Every non-NULL
 * output must be aligned, writable, mutually disjoint, and disjoint from live
 * page storage. No conflicting concurrent access is permitted. */
rofd_status_t rofd_page_get_text(const rofd_page_t *page,
                                rofd_string_t **text, rofd_error_t **error);
/** Return owned source glyphs with positive-area intersection, in canonical
 * order, omitting synthesized separators. area_mm is required and must point
 * to a complete initialized, readable, aligned rofd_rect_t in physical-page
 * millimetres. Nonfinite coordinates or edges and negative dimensions return
 * ROFD_STATUS_INVALID_ARGUMENT. Negative positions are valid; empty areas
 * produce empty strings. The entire rectangle and live page storage must be
 * disjoint from text and error. Ownership and output rules match get_text. */
rofd_status_t rofd_page_get_text_for_area(const rofd_page_t *page,
                                         const rofd_rect_t *area_mm,
                                         rofd_string_t **text, rofd_error_t **error);
/** Borrow immutable NUL-terminated UTF-8 bytes until rofd_string_free; NULL
 * returns NULL. A non-NULL string handle must remain live during every read. */
const char *rofd_string_get_data(const rofd_string_t *text);
/** UTF-8 byte length excluding the terminating NUL; NULL returns zero. */
size_t rofd_string_get_length(const rofd_string_t *text);
/** Free once after all reads finish; NULL is a no-op. */
void rofd_string_free(rofd_string_t *text);

/** Return an independently owned immutable snapshot, one entry per Unicode
 * scalar in canonical page text, including synthesized separators. Free with
 * rofd_text_layout_free. The snapshot survives page and document destruction.
 * page and layout are required; error is optional. All outputs must be aligned,
 * writable, mutually disjoint and disjoint from live page storage. */
rofd_status_t rofd_page_get_text_layout(const rofd_page_t *page,
                                       rofd_text_layout_t **layout, rofd_error_t **error);
/** Return the scalar count. layout and count are required. Live layout storage
 * must be disjoint from aligned writable count and optional error outputs;
 * outputs must be mutually disjoint. Failure initializes valid count to zero. */
rofd_status_t rofd_text_layout_get_count(const rofd_text_layout_t *layout,
                                        size_t *count, rofd_error_t **error);
/** Copy one character by index; out-of-range returns ROFD_STATUS_PAGE_OUT_OF_RANGE.
 * Before calling, set character->struct_size to sizeof(rofd_text_char_t) or a
 * larger caller record size. A non-NULL record must have an initialized readable
 * size field and, for supported sizes, an aligned writable complete v1 prefix.
 * The transaction clears the permanent v1 prefix including padding, preserves
 * the caller-declared struct_size and unknown tail, and publishes fields only
 * on success. NULL and undersized records remain untouched; a separate error
 * receives ROFD_STATUS_INVALID_ARGUMENT. Offsets and lengths are UTF-8 bytes in
 * rofd_page_get_text's canonical string, not character indices or area-text
 * offsets. Rectangles use physical-page millimetres; missing geometry is a zero
 * rectangle. Synthesized separators have object_id zero and SYNTHESIZED_SEPARATOR;
 * approximate geometry sets CONSERVATIVE_GEOMETRY (also set on current separators).
 * Live layout storage and every output, including optional error, must be
 * mutually disjoint and have no conflicting concurrent access during the call. */
rofd_status_t rofd_text_layout_get_char(const rofd_text_layout_t *layout,
                                       size_t index, rofd_text_char_t *character,
                                       rofd_error_t **error);
/** Free once after all reads finish; NULL is a no-op. Copied records remain valid. */
void rofd_text_layout_free(rofd_text_layout_t *layout);

/** Find canonical page text with case-insensitive substring defaults. query
 * must be non-NULL, valid nonempty UTF-8. The independently owned result
 * survives page/document destruction and is freed with rofd_text_search_free.
 * Input and output storage follow the global disjointness contract. */
rofd_status_t rofd_page_find_text(const rofd_page_t *page, const char *query,
                                  rofd_text_search_t **search,
                                  rofd_error_t **error);
/** Find canonical page text with versioned options. NULL options select
 * defaults. Unknown flags and zero max_results are invalid; larger record tails
 * are ignored and result count is bounded by the page resource policy. */
rofd_status_t rofd_page_find_text_with_options(
    const rofd_page_t *page, const char *query,
    const rofd_find_options_t *options, rofd_text_search_t **search,
    rofd_error_t **error);
/** Return the number of owned matches. search and count are required; failure
 * initializes a valid count output to zero. */
rofd_status_t rofd_text_search_get_count(const rofd_text_search_t *search,
                                         size_t *count,
                                         rofd_error_t **error);
/** Copy one match by index. Set match->struct_size to sizeof(rofd_text_match_t)
 * or a larger caller size. The permanent v1 prefix is cleared transactionally,
 * struct_size and unknown tail are preserved, and an absent match rectangle is
 * returned as zeros. Out-of-range returns ROFD_STATUS_PAGE_OUT_OF_RANGE. */
rofd_status_t rofd_text_search_get_match(const rofd_text_search_t *search,
                                         size_t index,
                                         rofd_text_match_t *match,
                                         rofd_error_t **error);
/** Free an owned search result once after all reads; NULL is a no-op. */
void rofd_text_search_free(rofd_text_search_t *search);

/** Select text intersecting selection_mm using a ROFD_SELECTION_* style. The
 * rectangle is in physical-page millimetres and follows the same finite,
 * non-negative-dimension rules as area extraction. The independently owned
 * result survives page/document destruction and must be freed with
 * rofd_text_selection_free. page, selection_mm and selection are required. */
rofd_status_t rofd_page_get_selected_text(
    const rofd_page_t *page, uint32_t style,
    const rofd_rect_t *selection_mm, rofd_text_selection_t **selection,
    rofd_error_t **error);
/** Borrow immutable NUL-terminated selected UTF-8 text until selection free;
 * NULL returns NULL. */
const char *rofd_text_selection_get_text(
    const rofd_text_selection_t *selection);
/** Selected UTF-8 byte length excluding the terminating NUL; NULL is zero. */
size_t rofd_text_selection_get_text_length(
    const rofd_text_selection_t *selection);
/** Return the number of ordered selected page regions. selection and count are
 * required; failure initializes a valid count output to zero. */
rofd_status_t rofd_text_selection_get_region_count(
    const rofd_text_selection_t *selection, size_t *count,
    rofd_error_t **error);
/** Copy one region in physical-page millimetres. Out-of-range returns
 * ROFD_STATUS_PAGE_OUT_OF_RANGE and initializes a valid output to zeros. */
rofd_status_t rofd_text_selection_get_region(
    const rofd_text_selection_t *selection, size_t index,
    rofd_rect_t *region_mm, rofd_error_t **error);
/** Free an owned text selection after all borrowed text reads; NULL is a no-op. */
void rofd_text_selection_free(rofd_text_selection_t *selection);

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
 * Otherwise it is set to NULL on entry and receives one owned report on
 * success, even when the report is empty. cairo must be a non-NULL live
 * context whose target and referenced objects remain valid for the call. The
 * caller must obey Cairo's context/thread synchronization rules and prevent
 * concurrent context mutation. rofd takes and drops one temporary reference;
 * it does not consume the caller's reference. Renderer/page handle storage,
 * the readable options prefix, and the detectable Cairo address must not
 * overlap report or error output storage.
 */
rofd_status_t rofd_renderer_render_page_cairo(
    const rofd_renderer_t *renderer,
    const rofd_page_t *page,
    cairo_t *cairo,
    const rofd_render_options_t *options,
    rofd_render_report_t **report,
    rofd_error_t **error);
void rofd_renderer_free(rofd_renderer_t *renderer);

/** Borrow a live report and return its diagnostic count. report and
 * diagnostic_count are required; diagnostic_count must be writable and
 * aligned. error is optional. When error is non-NULL, it and diagnostic_count
 * must satisfy the global writable, aligned, and mutually disjoint output
 * contract. Report storage must be disjoint from every non-NULL output. */
rofd_status_t rofd_render_report_get_count(const rofd_render_report_t *report,
                                           size_t *diagnostic_count,
                                           rofd_error_t **error);
/**
 * Get one diagnostic from a render report.
 *
 * Before calling, set diagnostic->struct_size to
 * sizeof(rofd_render_diagnostic_t) or any larger caller record size.
 * A valid output has its complete v1 prefix, including padding, cleared before
 * lookup while preserving the caller's input struct_size and unknown tail.
 * Thus failure after record validation leaves kind/object_id zero and message
 * NULL. The returned message is borrowed until the report is freed. The report
 * handle storage must not overlap diagnostic or error output storage.
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
