#include <cairo.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

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
    cairo_surface_t *surface = NULL;
    cairo_t *cairo = NULL;
    rofd_render_options_t render_options;
    rofd_render_diagnostic_t diagnostic = {0};
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

    CHECK(rofd_render_report_get_count(report, &diagnostic_count, NULL) ==
          ROFD_STATUS_OK);
    CHECK(diagnostic_count > 0u);
    diagnostic.struct_size = sizeof(diagnostic);
    CHECK(rofd_render_report_get_diagnostic(report, 0u, &diagnostic, &error) ==
          ROFD_STATUS_OK);
    CHECK(diagnostic.message != NULL);
    CHECK(diagnostic.message[0] != '\0');
    result = 0;

cleanup:
    if (error != NULL) {
        const char *message = rofd_error_get_message(error);
        fprintf(stderr, "rofd error (%u): %s\n", rofd_error_get_status(error),
                message != NULL ? message : "<no message>");
    }
    rofd_error_free(error);
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
