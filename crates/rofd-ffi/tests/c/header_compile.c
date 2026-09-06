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
_Static_assert(offsetof(rofd_load_options_t, struct_size) == 0,
               "load options struct_size must be first");
_Static_assert(offsetof(rofd_renderer_options_t, struct_size) == 0,
               "renderer options struct_size must be first");
_Static_assert(offsetof(rofd_render_options_t, struct_size) == 0,
               "render options struct_size must be first");
_Static_assert(offsetof(rofd_render_diagnostic_t, struct_size) == 0,
               "render diagnostic struct_size must be first");

static void initialize_options(void) {
    rofd_load_options_t load_options;
    rofd_renderer_options_t renderer_options;
    rofd_render_options_t render_options;

    rofd_load_options_init(&load_options);
    rofd_renderer_options_init(&renderer_options);
    rofd_render_options_init(&render_options);
}

int main(void) {
    initialize_options();
    return 0;
}
