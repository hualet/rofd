#include <cstdint>
#include <type_traits>

#include "rofd.h"

static_assert(ROFD_ABI_VERSION == 1u, "unexpected ABI version");
static_assert(std::is_standard_layout_v<rofd_rect_t>,
              "rofd_rect_t must have standard layout");
static_assert(std::is_standard_layout_v<rofd_render_options_t>,
              "rofd_render_options_t must have standard layout");

int main() {
    rofd_load_options_t load_options;
    rofd_renderer_options_t renderer_options;
    rofd_render_options_t render_options;
    rofd_document_t *document = nullptr;
    rofd_page_t *page = nullptr;
    rofd_renderer_t *renderer = nullptr;
    rofd_render_report_t *report = nullptr;
    rofd_error_t *error = nullptr;

    rofd_load_options_init(&load_options, sizeof(load_options));
    rofd_renderer_options_init(&renderer_options, sizeof(renderer_options));
    rofd_render_options_init(&render_options, sizeof(render_options));

    (void)document;
    (void)page;
    (void)renderer;
    (void)report;
    (void)error;
    return rofd_abi_version() == ROFD_ABI_VERSION &&
                   load_options.struct_size == sizeof(load_options) &&
                   renderer_options.struct_size == sizeof(renderer_options) &&
                   render_options.struct_size == sizeof(render_options)
               ? 0
               : 1;
}
