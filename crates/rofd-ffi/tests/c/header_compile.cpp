#include <cstdint>
#include <type_traits>

#include "rofd.h"

static_assert(ROFD_ABI_VERSION == 1u, "unexpected ABI version");
static_assert(std::is_standard_layout_v<rofd_rect_t>,
              "rofd_rect_t must have standard layout");
static_assert(std::is_standard_layout_v<rofd_render_options_t>,
              "rofd_render_options_t must have standard layout");

int main() {
    rofd_document_t *document = nullptr;
    rofd_page_t *page = nullptr;
    rofd_renderer_t *renderer = nullptr;
    rofd_render_report_t *report = nullptr;
    rofd_error_t *error = nullptr;

    (void)document;
    (void)page;
    (void)renderer;
    (void)report;
    (void)error;
    return 0;
}
