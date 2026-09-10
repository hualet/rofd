#include <stddef.h>
#include <stdint.h>

#include "rofd.h"

_Static_assert(ROFD_ABI_VERSION == 1u, "unexpected ABI version");
_Static_assert(sizeof(rofd_status_t) == sizeof(uint32_t),
               "rofd_status_t must be uint32_t-sized");
_Static_assert(ROFD_IMAGE_INTERPOLATION_NEAREST == 0u,
               "unexpected nearest-neighbor interpolation value");
_Static_assert(ROFD_IMAGE_INTERPOLATION_BILINEAR == 1u,
               "unexpected bilinear interpolation value");
_Static_assert(ROFD_FIND_CASE_SENSITIVE == (1u << 0),
               "unexpected case-sensitive find flag");
_Static_assert(ROFD_FIND_WHOLE_WORDS == (1u << 1),
               "unexpected whole-words find flag");
_Static_assert(ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR == (1u << 0),
               "unexpected synthesized-separator text flag");
_Static_assert(ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY == (1u << 1),
               "unexpected conservative-geometry text flag");
_Static_assert(ROFD_SELECTION_GLYPH == 0u,
               "unexpected glyph selection value");
_Static_assert(ROFD_SELECTION_WORD == 1u,
               "unexpected word selection value");
_Static_assert(ROFD_SELECTION_LINE == 2u,
               "unexpected line selection value");
_Static_assert(offsetof(rofd_load_options_t, struct_size) == 0,
               "load options struct_size must be first");
_Static_assert(offsetof(rofd_renderer_options_t, struct_size) == 0,
               "renderer options struct_size must be first");
_Static_assert(offsetof(rofd_render_options_t, struct_size) == 0,
               "render options struct_size must be first");
_Static_assert(offsetof(rofd_find_options_t, struct_size) == 0,
               "find options struct_size must be first");
_Static_assert(offsetof(rofd_find_options_t, flags) <
                   offsetof(rofd_find_options_t, max_results),
               "find options fields must retain declaration order");
_Static_assert(offsetof(rofd_text_char_t, struct_size) == 0,
               "text character struct_size must be first");
_Static_assert(offsetof(rofd_text_char_t, utf8_offset) <
                   offsetof(rofd_text_char_t, utf8_length) &&
                   offsetof(rofd_text_char_t, utf8_length) <
                       offsetof(rofd_text_char_t, rect_mm) &&
                   offsetof(rofd_text_char_t, rect_mm) <
                       offsetof(rofd_text_char_t, flags) &&
                   offsetof(rofd_text_char_t, flags) <
                       offsetof(rofd_text_char_t, object_id),
               "text character fields must retain declaration order");
_Static_assert(offsetof(rofd_text_match_t, struct_size) == 0,
               "text match struct_size must be first");
_Static_assert(offsetof(rofd_text_match_t, utf8_offset) <
                   offsetof(rofd_text_match_t, utf8_length) &&
                   offsetof(rofd_text_match_t, utf8_length) <
                       offsetof(rofd_text_match_t, rect_mm),
               "text match fields must retain declaration order");
_Static_assert(offsetof(rofd_render_diagnostic_t, struct_size) == 0,
               "render diagnostic struct_size must be first");

static int consume_api(void) {
    rofd_load_options_t load_options;
    rofd_renderer_options_t renderer_options;
    rofd_render_options_t render_options;
    rofd_find_options_t find_options;
    rofd_string_t *string = NULL;
    rofd_text_layout_t *layout = NULL;
    rofd_text_search_t *search = NULL;
    rofd_text_selection_t *selection = NULL;

    rofd_load_options_init(&load_options, sizeof(load_options));
    rofd_renderer_options_init(&renderer_options, sizeof(renderer_options));
    rofd_render_options_init(&render_options, sizeof(render_options));
    rofd_find_options_init(&find_options, sizeof(find_options));

    if (rofd_page_get_text(NULL, NULL, NULL) != ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_page_get_text_for_area(NULL, NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_page_get_text_layout(NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_text_layout_get_count(NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_text_layout_get_char(NULL, 0u, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_page_find_text(NULL, NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_page_find_text_with_options(NULL, NULL, NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_text_search_get_count(NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_text_search_get_match(NULL, 0u, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_page_get_selected_text(NULL, ROFD_SELECTION_GLYPH, NULL, NULL,
                                    NULL) != ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_text_selection_get_region_count(NULL, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_text_selection_get_region(NULL, 0u, NULL, NULL) !=
            ROFD_STATUS_INVALID_ARGUMENT ||
        rofd_string_get_data(NULL) != NULL || rofd_string_get_length(NULL) != 0u ||
        rofd_text_selection_get_text(NULL) != NULL ||
        rofd_text_selection_get_text_length(NULL) != 0u) {
        return 1;
    }
    rofd_string_free(NULL);
    rofd_text_layout_free(NULL);
    rofd_text_search_free(NULL);
    rofd_text_selection_free(NULL);

    (void)string;
    (void)layout;
    (void)search;
    (void)selection;

    return rofd_abi_version() == ROFD_ABI_VERSION &&
           load_options.struct_size == sizeof(load_options) &&
           renderer_options.struct_size == sizeof(renderer_options) &&
           render_options.struct_size == sizeof(render_options) &&
           find_options.struct_size == sizeof(find_options) &&
           find_options.flags == 0u && find_options.max_results == 10000u
               ? 0
               : 1;
}

int main(void) {
    return consume_api();
}
