#include <cairo.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "rofd.h"

#define CHECK(condition)                                                       \
    do {                                                                       \
        if (!(condition)) {                                                    \
            fprintf(stderr, "check failed at %s:%d: %s\n", __FILE__, __LINE__, \
                    #condition);                                               \
            goto cleanup;                                                      \
        }                                                                      \
    } while (0)

typedef int (*pixel_predicate_t)(uint32_t red, uint32_t green, uint32_t blue);

static size_t matching_pixels(const uint32_t *pixels,
                              size_t stride_pixels,
                              int x_min,
                              int y_min,
                              int x_max,
                              int y_max,
                              pixel_predicate_t predicate) {
    size_t count = 0u;
    int x;
    int y;

    for (y = y_min; y <= y_max; ++y) {
        for (x = x_min; x <= x_max; ++x) {
            const uint32_t pixel = pixels[(size_t)y * stride_pixels + (size_t)x];
            const uint32_t red = (pixel >> 16u) & 0xffu;
            const uint32_t green = (pixel >> 8u) & 0xffu;
            const uint32_t blue = pixel & 0xffu;
            count += (size_t)predicate(red, green, blue);
        }
    }
    return count;
}

static int is_qr_black(uint32_t red, uint32_t green, uint32_t blue) {
    return red < 40u && green < 40u && blue < 40u;
}

static int is_title_dark_red(uint32_t red, uint32_t green, uint32_t blue) {
    return red > 80u && green < 45u && blue < 45u;
}

static int is_amount_black(uint32_t red, uint32_t green, uint32_t blue) {
    return red < 100u && green < 100u && blue < 100u;
}

static int is_rule_dark_red(uint32_t red, uint32_t green, uint32_t blue) {
    return red > 70u && green < 50u && blue < 50u;
}

int main(int argc, char **argv) {
    rofd_document_t *document = NULL;
    rofd_page_t *page = NULL;
    rofd_renderer_t *renderer = NULL;
    rofd_render_report_t *report = NULL;
    rofd_error_t *error = NULL;
    rofd_string_t *text = NULL;
    rofd_text_layout_t *layout = NULL;
    rofd_text_search_t *search = NULL;
    rofd_text_selection_t *selection = NULL;
    cairo_surface_t *surface = NULL;
    cairo_surface_t *tile_surface = NULL;
    cairo_t *tile_cairo = NULL;
    cairo_t *cairo = NULL;
    rofd_render_options_t render_options;
    rofd_pixel_rect_t viewport;
    rofd_render_diagnostic_t diagnostic = {0};
    rofd_find_options_t find_options;
    rofd_text_match_t match = {0};
    rofd_rect_t page_rect = {0.0, 0.0, 0.0, 0.0};
    size_t page_count = 0u;
    size_t diagnostic_count = 0u;
    size_t page_index = SIZE_MAX;
    int32_t width = 0;
    int32_t height = 0;
    int stride = 0;
    unsigned char *data = NULL;
    int result = 1;

    CHECK(argc == 2);
    CHECK(rofd_abi_version() == ROFD_ABI_VERSION);
    CHECK(rofd_library_version() != NULL);
    CHECK(rofd_document_open(argv[1], NULL, &document, &error) ==
          ROFD_STATUS_OK);
    CHECK(document != NULL);
    CHECK(error == NULL);
    CHECK(rofd_document_get_page_count(document, &page_count, &error) ==
          ROFD_STATUS_OK);
    CHECK(page_count == 1u);
    CHECK(rofd_document_get_page(document, 0u, &page, &error) ==
          ROFD_STATUS_OK);
    CHECK(page != NULL);
    CHECK(rofd_page_get_size_mm(page, &page_rect, &error) == ROFD_STATUS_OK);
    CHECK(page_rect.width_mm == 211.5);
    CHECK(page_rect.height_mm == 140.0);

    CHECK(rofd_page_get_text(page, &text, &error) == ROFD_STATUS_OK);
    CHECK(rofd_string_get_data(text) != NULL);
    CHECK(strstr(rofd_string_get_data(text), "电子发票（普通发票）") != NULL);
    CHECK(rofd_page_get_text_layout(page, &layout, &error) == ROFD_STATUS_OK);
    CHECK(rofd_text_layout_get_count(layout, &page_count, &error) ==
          ROFD_STATUS_OK);
    CHECK(page_count > 0u);

    rofd_find_options_init(&find_options, sizeof(find_options));
    find_options.flags = ROFD_FIND_WHOLE_WORDS;
    CHECK(rofd_page_find_text_with_options(page, "发票号码", &find_options,
                                           &search, &error) == ROFD_STATUS_OK);
    CHECK(rofd_text_search_get_count(search, &page_count, &error) ==
          ROFD_STATUS_OK);
    CHECK(page_count == 1u);
    match.struct_size = sizeof(match);
    CHECK(rofd_text_search_get_match(search, 0u, &match, &error) ==
          ROFD_STATUS_OK);
    CHECK(match.utf8_length == strlen("发票号码"));
    CHECK(match.rect_mm.width_mm > 0.0 && match.rect_mm.height_mm > 0.0);
    CHECK(rofd_page_get_selected_text(page, ROFD_SELECTION_WORD,
                                      &match.rect_mm, &selection, &error) ==
          ROFD_STATUS_OK);
    CHECK(rofd_text_selection_get_region_count(selection, &page_count, &error) ==
          ROFD_STATUS_OK);
    CHECK(page_count > 0u);

    rofd_document_free(document);
    document = NULL;
    CHECK(rofd_page_get_index(page, &page_index, &error) == ROFD_STATUS_OK);
    CHECK(page_index == 0u);

    CHECK(rofd_renderer_new(NULL, &renderer, &error) == ROFD_STATUS_OK);
    CHECK(renderer != NULL);
    rofd_render_options_init(&render_options, sizeof(render_options));
    CHECK(render_options.struct_size == sizeof(render_options));
    render_options.dpi = 254.0;
    CHECK(rofd_renderer_get_pixel_size(renderer, page, &render_options, &width,
                                       &height, &error) == ROFD_STATUS_OK);
    CHECK(width == 2115);
    CHECK(height == 1400);

    surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, width, height);
    CHECK(cairo_surface_status(surface) == CAIRO_STATUS_SUCCESS);
    cairo = cairo_create(surface);
    CHECK(cairo_status(cairo) == CAIRO_STATUS_SUCCESS);
    CHECK(cairo_get_reference_count(cairo) == 1u);
    CHECK(rofd_renderer_render_page_cairo(renderer, page, cairo,
                                          &render_options, &report, &error) ==
          ROFD_STATUS_OK);
    CHECK(report != NULL);
    CHECK(error == NULL);
    CHECK(cairo_get_reference_count(cairo) == 1u);

    /* The borrowed context remains caller-owned and usable after rendering. */
    cairo_move_to(cairo, 7.0, 9.0);
    CHECK(cairo_status(cairo) == CAIRO_STATUS_SUCCESS);
    CHECK(cairo_has_current_point(cairo));

    cairo_surface_flush(surface);
    CHECK(cairo_surface_status(surface) == CAIRO_STATUS_SUCCESS);
    data = cairo_image_surface_get_data(surface);
    stride = cairo_image_surface_get_stride(surface);
    CHECK(data != NULL);
    CHECK(stride > 0 && stride % (int)sizeof(uint32_t) == 0);
    CHECK(((uintptr_t)(const void *)data % _Alignof(uint32_t)) == 0u);
    CHECK(matching_pixels((const uint32_t *)(const void *)data,
                          (size_t)stride / sizeof(uint32_t), 50, 30, 290, 260,
                          is_qr_black) > 8000u);
    CHECK(matching_pixels((const uint32_t *)(const void *)data,
                          (size_t)stride / sizeof(uint32_t), 600, 50, 1330,
                          180, is_title_dark_red) > 4000u);
    CHECK(matching_pixels((const uint32_t *)(const void *)data,
                          (size_t)stride / sizeof(uint32_t), 1300, 820, 1650,
                          980, is_amount_black) > 100u);
    CHECK(matching_pixels((const uint32_t *)(const void *)data,
                          (size_t)stride / sizeof(uint32_t), 25, 260, 2090,
                          1170, is_rule_dark_red) > 15000u);

    CHECK(rofd_renderer_get_pixel_canvas_size(renderer, page, &render_options,
                                               &width, &height, &error) == ROFD_STATUS_OK);
    CHECK(width == 2115 && height == 1400);
    rofd_pixel_rect_init(&viewport, sizeof(viewport));
    viewport.x = 83;
    viewport.y = 47;
    viewport.width = 257;
    viewport.height = 129;
    tile_surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, viewport.width,
                                              viewport.height);
    CHECK(cairo_surface_status(tile_surface) == CAIRO_STATUS_SUCCESS);
    tile_cairo = cairo_create(tile_surface);
    CHECK(rofd_renderer_render_page_region_cairo(renderer, page, tile_cairo,
            &render_options, &viewport, NULL, &error) == ROFD_STATUS_OK);
    CHECK(cairo_get_reference_count(tile_cairo) == 1u);
    cairo_surface_flush(tile_surface);
    size_t different_channels = 0;
    unsigned int maximum_delta = 0;
    for (int row = 0; row < viewport.height; ++row) {
        const unsigned char *tile_row = cairo_image_surface_get_data(tile_surface) +
            (size_t)row * (size_t)cairo_image_surface_get_stride(tile_surface);
        const unsigned char *full_row = data +
            (size_t)(row + viewport.y) * (size_t)stride + (size_t)viewport.x * 4u;
        for (size_t column = 0; column < (size_t)viewport.width * 4u; ++column) {
            unsigned int delta = tile_row[column] > full_row[column] ?
                (unsigned int)(tile_row[column] - full_row[column]) :
                (unsigned int)(full_row[column] - tile_row[column]);
            if (delta != 0) ++different_channels;
            if (delta > maximum_delta) maximum_delta = delta;
        }
    }
    /* Cairo curve coverage depends slightly on target extents. This invoice
       has 8 changed channels by one level; bound both magnitude and count. */
    CHECK(different_channels <= 16 && maximum_delta <= 1);

    CHECK(rofd_render_report_get_count(report, &diagnostic_count, NULL) ==
          ROFD_STATUS_OK);
    CHECK(diagnostic_count > 0u);
    diagnostic.struct_size = sizeof(diagnostic);
    CHECK(rofd_render_report_get_diagnostic(report, 0u, &diagnostic, &error) ==
          ROFD_STATUS_OK);
    CHECK(diagnostic.message != NULL);
    CHECK(diagnostic.message[0] != '\0');

    rofd_page_free(page);
    page = NULL;
    match.struct_size = sizeof(match);
    CHECK(rofd_text_search_get_match(search, 0u, &match, &error) ==
          ROFD_STATUS_OK);
    CHECK(match.utf8_length == strlen("发票号码"));
    CHECK(rofd_text_selection_get_text(selection) != NULL);
    CHECK(rofd_text_selection_get_text_length(selection) >=
          strlen("发票号码"));
    CHECK(strstr(rofd_text_selection_get_text(selection), "发票号码") != NULL);
    result = 0;

cleanup:
    if (tile_cairo != NULL) cairo_destroy(tile_cairo);
    if (tile_surface != NULL) cairo_surface_destroy(tile_surface);
    if (error != NULL) {
        const char *message = rofd_error_get_message(error);
        fprintf(stderr, "rofd error (%u): %s\n", rofd_error_get_status(error),
                message != NULL ? message : "<no message>");
    }
    rofd_error_free(error);
    rofd_text_selection_free(selection);
    rofd_text_search_free(search);
    rofd_text_layout_free(layout);
    rofd_string_free(text);
    rofd_render_report_free(report);
    if (cairo != NULL) {
        cairo_destroy(cairo);
    }
    if (surface != NULL) {
        cairo_surface_destroy(surface);
    }
    rofd_renderer_free(renderer);
    rofd_page_free(page);
    rofd_document_free(document);
    return result;
}
