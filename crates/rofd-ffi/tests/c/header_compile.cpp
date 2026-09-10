#include <cstdint>
#include <type_traits>

#include "rofd.h"

static_assert(ROFD_ABI_VERSION == 1u, "unexpected ABI version");
static_assert(std::is_standard_layout_v<rofd_rect_t>,
              "rofd_rect_t must have standard layout");
static_assert(std::is_standard_layout_v<rofd_render_options_t>,
              "rofd_render_options_t must have standard layout");
static_assert(std::is_standard_layout_v<rofd_find_options_t>,
              "rofd_find_options_t must have standard layout");
static_assert(std::is_standard_layout_v<rofd_text_char_t>,
              "rofd_text_char_t must have standard layout");
static_assert(std::is_standard_layout_v<rofd_text_match_t>,
              "rofd_text_match_t must have standard layout");
static_assert(ROFD_FIND_CASE_SENSITIVE == (1u << 0),
              "unexpected case-sensitive find flag");
static_assert(ROFD_FIND_WHOLE_WORDS == (1u << 1),
              "unexpected whole-words find flag");
static_assert(ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR == (1u << 0),
              "unexpected synthesized-separator text flag");
static_assert(ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY == (1u << 1),
              "unexpected conservative-geometry text flag");
static_assert(ROFD_SELECTION_GLYPH == 0u, "unexpected glyph selection value");
static_assert(ROFD_SELECTION_WORD == 1u, "unexpected word selection value");
static_assert(ROFD_SELECTION_LINE == 2u, "unexpected line selection value");
static_assert(offsetof(rofd_find_options_t, struct_size) == 0,
              "find options struct_size must be first");
static_assert(offsetof(rofd_text_char_t, struct_size) == 0,
              "text character struct_size must be first");
static_assert(offsetof(rofd_text_match_t, struct_size) == 0,
              "text match struct_size must be first");

using page_find_text_fn = rofd_status_t (*)(const rofd_page_t *, const char *,
                                            rofd_text_search_t **,
                                            rofd_error_t **);
using page_find_text_with_options_fn = rofd_status_t (*)(
    const rofd_page_t *, const char *, const rofd_find_options_t *,
    rofd_text_search_t **, rofd_error_t **);
using page_get_selected_text_fn = rofd_status_t (*)(
    const rofd_page_t *, std::uint32_t, const rofd_rect_t *,
    rofd_text_selection_t **, rofd_error_t **);
static_assert(std::is_same_v<decltype(&rofd_page_find_text), page_find_text_fn>,
              "unexpected default search declaration");
static_assert(std::is_same_v<decltype(&rofd_page_find_text_with_options),
                             page_find_text_with_options_fn>,
              "unexpected option search declaration");
static_assert(std::is_same_v<decltype(&rofd_page_get_selected_text),
                             page_get_selected_text_fn>,
              "unexpected selection declaration");

int main() {
    rofd_load_options_t load_options;
    rofd_renderer_options_t renderer_options;
    rofd_render_options_t render_options;
    rofd_find_options_t find_options;
    rofd_document_t *document = nullptr;
    rofd_page_t *page = nullptr;
    rofd_renderer_t *renderer = nullptr;
    rofd_render_report_t *report = nullptr;
    rofd_error_t *error = nullptr;
    rofd_string_t *string = nullptr;
    rofd_text_layout_t *layout = nullptr;
    rofd_text_search_t *search = nullptr;
    rofd_text_selection_t *selection = nullptr;

    rofd_load_options_init(&load_options, sizeof(load_options));
    rofd_renderer_options_init(&renderer_options, sizeof(renderer_options));
    rofd_render_options_init(&render_options, sizeof(render_options));
    rofd_find_options_init(&find_options, sizeof(find_options));

    (void)&rofd_page_get_text;
    (void)&rofd_page_get_text_for_area;
    (void)&rofd_page_get_text_layout;
    (void)&rofd_text_layout_get_count;
    (void)&rofd_text_layout_get_char;
    (void)&rofd_string_get_data;
    (void)&rofd_string_get_length;
    (void)&rofd_string_free;
    (void)&rofd_page_find_text;
    (void)&rofd_page_find_text_with_options;
    (void)&rofd_text_search_get_count;
    (void)&rofd_text_search_get_match;
    (void)&rofd_text_search_free;
    (void)&rofd_page_get_selected_text;
    (void)&rofd_text_selection_get_text;
    (void)&rofd_text_selection_get_text_length;
    (void)&rofd_text_selection_get_region_count;
    (void)&rofd_text_selection_get_region;
    (void)&rofd_text_selection_free;

    (void)document;
    (void)page;
    (void)renderer;
    (void)report;
    (void)error;
    (void)string;
    (void)layout;
    (void)search;
    (void)selection;
    return rofd_abi_version() == ROFD_ABI_VERSION &&
                   load_options.struct_size == sizeof(load_options) &&
                   renderer_options.struct_size == sizeof(renderer_options) &&
                   render_options.struct_size == sizeof(render_options) &&
                   find_options.struct_size == sizeof(find_options) &&
                   find_options.flags == 0u &&
                   find_options.max_results == 10000u
               ? 0
               : 1;
}
