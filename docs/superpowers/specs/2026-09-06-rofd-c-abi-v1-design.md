# rofd C ABI v1 Design

## Goal and Scope

This phase adds a small, stable C interface over the already usable
`rofd-core` and `rofd-render` crates. It exists to support the Qt/QML reader
without exposing Rust layout, ownership, or panic behavior. Version 1 opens a
UTF-8 file path, queries pages and page geometry, computes raster dimensions,
renders into a caller-owned Cairo context, and reports fatal errors and
non-fatal rendering diagnostics.

Text search, metadata, annotations, signatures, byte-buffer input, a GLib or
GObject wrapper, and the Qt application remain outside this phase. They may be
added later with new functions or fields appended to versioned option
structures; published v1 values and layouts must not be changed.

## Chosen Approach

Add a focused `crates/rofd-ffi/` package depending only on `rofd-core`,
`rofd-render`, and Cairo. It builds `rlib`, `cdylib`, and `staticlib` artifacts;
the initial link name is `rofd_ffi` to avoid colliding with the legacy root
`rofd` crate. `crates/rofd-ffi/include/rofd.h` is the canonical, hand-maintained
C header. Generated-header approaches are rejected because ordinary Rust type
changes must not silently redefine the public ABI. A full GLib/GObject layer is
deferred because it would enlarge both the dependency surface and the API.

The dependency direction remains:

```text
Qt reader -> rofd.h / librofd_ffi -> rofd-render -> rofd-core
```

Only `rofd-ffi` may contain the unsafe code required to validate raw pointers,
read C strings, and borrow `cairo_t`. It enables `unsafe_op_in_unsafe_fn`, keeps
unsafe blocks small, and documents every safety invariant. Core and renderer
remain free of FFI concerns.

## ABI Types and Compatibility

The header is valid C11 and C++. It uses `extern "C"`, fixed-width integers,
`size_t`, opaque handle declarations, and `ROFD_ABI_VERSION 1`. Rust-facing ABI
records use `#[repr(C)]`; Rust enums, `bool`, `String`, `Vec`, references, and
platform-dependent Rust layouts never cross the boundary.

The opaque handles are:

- `rofd_document_t`, owning one cloned `rofd_core::Document`;
- `rofd_page_t`, owning one `rofd_core::Page` and therefore its document data;
- `rofd_renderer_t`, owning a system-font snapshot, ordered fallback families,
  and one bounded image cache;
- `rofd_render_report_t`, owning stable copies of diagnostics from one render;
  and
- `rofd_error_t`, owning a status and a NUL-terminated UTF-8 message.

Every options record starts with `uint32_t struct_size`. Each init function also
receives the caller's writable capacity. Each published record version has a
permanent size boundary: the end of its last field rounded up to that version's
maximum field alignment. This includes tail padding, including target-specific
tail padding on 32-bit layouts. Supported boundaries are kept as an ordered
version list.

A null pointer or capacity below the oldest supported boundary is a no-op and
no byte is touched. Otherwise the initializer selects the highest complete
supported version that fits, zeros and fills exactly that prefix, leaves later
caller bytes unchanged, and writes the selected boundary—not caller capacity or
the newest record size—to `struct_size`. A new library must therefore initialize
the v1 prefix for a v1-sized old caller even after fields have been appended; an
old library preserves fields in a new caller's unknown tail.

Records evolve only by appending fields. No field may be inserted, reordered,
removed, or reinterpreted. A new field must start at or after the preceding
version boundary and may not consume that version's tail padding; use explicit
padding or an equivalent layout constraint when an ABI requires it.

Every non-NULL input C string must point to the first byte of a valid readable
character array object containing UTF-8 bytes. The terminating NUL byte must
occur within that same object; its readable extent and byte length must be
representable by `ptrdiff_t`. The complete sequence must remain valid for the
entire call. Individual functions define whether NULL or empty input is allowed.
For `rofd_document_open`, a NULL path is a
defined input that returns `ROFD_STATUS_INVALID_ARGUMENT` under the normal
output transaction. An empty path also returns `ROFD_STATUS_INVALID_ARGUMENT`.
A non-NULL path must satisfy the general string validity, lifetime, and
non-overlap contract.
A non-NULL options pointer must be correctly aligned for its record type and
designate a valid object whose `struct_size` field is initialized and readable.
When `struct_size` declares the v1 boundary or a larger supported boundary, the
complete v1 prefix must be initialized and readable; every additional declared
supported prefix must likewise be initialized and readable for the call.

The initial records are:

```c
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
```

`background_rgba` is `0xRRGGBBAA`. Strictness, interpolation, and diagnostic
kinds are stable integer constants, not C enum layouts. Default renderer
construction scans system fonts once, uses 64 MiB font and image-cache limits,
and selects the ordered families `Noto Sans CJK SC`, `Noto Sans`, and
`DejaVu Sans`. Null options, or `fallback_families == NULL` and
`fallback_family_count == 0`, select these built-in default families;
`fallback_families != NULL` and `fallback_family_count == 0` explicitly
disable fallback; `fallback_family_count > 0` requires a valid array of
non-null UTF-8 strings. Specifically, the array pointer must be correctly
aligned for `const char *` and designate the first element of a single valid,
initialized, readable array object containing at least
`fallback_family_count` elements. Its total extent must be representable by
`ptrdiff_t`; every element must satisfy the general input C string contract.
Explicit zero byte limits are invalid.

## Functions and Ownership

All exported names begin with `rofd_`:

```c
uint32_t rofd_abi_version(void);
const char *rofd_library_version(void);

void rofd_load_options_init(rofd_load_options_t *options, size_t options_size);
void rofd_renderer_options_init(rofd_renderer_options_t *options,
                                size_t options_size);
void rofd_render_options_init(rofd_render_options_t *options,
                              size_t options_size);

rofd_status_t rofd_document_open(
    const char *path,
    const rofd_load_options_t *options,
    rofd_document_t **document,
    rofd_error_t **error);
rofd_status_t rofd_document_get_page_count(
    const rofd_document_t *document,
    size_t *page_count,
    rofd_error_t **error);
rofd_status_t rofd_document_get_page(
    const rofd_document_t *document,
    size_t index,
    rofd_page_t **page,
    rofd_error_t **error);
void rofd_document_free(rofd_document_t *document);

rofd_status_t rofd_page_get_index(
    const rofd_page_t *page,
    size_t *index,
    rofd_error_t **error);
rofd_status_t rofd_page_get_size_mm(
    const rofd_page_t *page,
    rofd_rect_t *size,
    rofd_error_t **error);
void rofd_page_free(rofd_page_t *page);

rofd_status_t rofd_renderer_new(
    const rofd_renderer_options_t *options,
    rofd_renderer_t **renderer,
    rofd_error_t **error);
rofd_status_t rofd_renderer_get_pixel_size(
    const rofd_renderer_t *renderer,
    const rofd_page_t *page,
    const rofd_render_options_t *options,
    int32_t *width,
    int32_t *height,
    rofd_error_t **error);
rofd_status_t rofd_renderer_render_page_cairo(
    const rofd_renderer_t *renderer,
    const rofd_page_t *page,
    cairo_t *context,
    const rofd_render_options_t *options,
    rofd_render_report_t **report,
    rofd_error_t **error);
void rofd_renderer_free(rofd_renderer_t *renderer);

rofd_status_t rofd_render_report_get_count(
    const rofd_render_report_t *report,
    size_t *count,
    rofd_error_t **error);
rofd_status_t rofd_render_report_get_diagnostic(
    const rofd_render_report_t *report,
    size_t index,
    rofd_render_diagnostic_t *diagnostic,
    rofd_error_t **error);
void rofd_render_report_free(rofd_render_report_t *report);

rofd_status_t rofd_error_get_status(const rofd_error_t *error);
const char *rofd_error_get_message(const rofd_error_t *error);
void rofd_error_free(rofd_error_t *error);
```

Every output handle is owned by the caller and has exactly one matching free
function. Free functions accept null. A page remains valid after its document
handle is freed. The returned library-version string is static and must not be
freed. A diagnostic message is borrowed from its report and remains valid until
that report is freed. `cairo_t *` is borrowed only for the duration of the call;
the FFI takes a temporary Cairo reference and never consumes the caller's
reference. The public header includes Cairo's public header for the `cairo_t`
declaration.

`report` may be null when diagnostics are not wanted. When non-null, successful
rendering returns a report even when its count is zero. Failure leaves it null.
`rofd_render_diagnostic_t` starts with `struct_size` and returns a stable kind,
object ID, and borrowed message. Initial kinds are unsupported object, font
fallback, missing glyph, unsupported image substitution, unsupported image
mask, and unsupported image border.

## Errors and Boundary Rules

`rofd_status_t` is `uint32_t`, with permanent numeric values:

| Value | Name | Meaning |
| ---: | --- | --- |
| 0 | `ROFD_STATUS_OK` | Operation completed |
| 1 | `ROFD_STATUS_INVALID_ARGUMENT` | Null, invalid UTF-8, option, or record size |
| 2 | `ROFD_STATUS_IO` | Host file access failed |
| 3 | `ROFD_STATUS_INVALID_DOCUMENT` | ZIP, XML, structure, value, or resource is invalid |
| 4 | `ROFD_STATUS_UNSUPPORTED` | Correct processing requires unsupported behavior |
| 5 | `ROFD_STATUS_LIMIT_EXCEEDED` | A configured safety or raster limit was exceeded |
| 6 | `ROFD_STATUS_PAGE_OUT_OF_RANGE` | Page or diagnostic index is outside the collection |
| 7 | `ROFD_STATUS_RENDER_ERROR` | Font, image, display-list, or Cairo rendering failed |
| 8 | `ROFD_STATUS_OUT_OF_MEMORY` | An explicitly reported allocation failed |
| 255 | `ROFD_STATUS_INTERNAL` | Panic, poisoned internal state, or unmapped invariant failure |

The Rust global allocator may abort the process and cannot be promised as a
recoverable status. The out-of-memory code applies only where the implementation
receives an allocation failure as a normal Rust error.

On entry, fallible functions visit every registered output: every non-null
handle output is set to null and every non-null scalar output is set to zero,
even when another required output location is null. Any null required location
then fails the call before its operation runs. Operations return typed staged
values that retain Rust ownership. Only a successful operation is committed to
caller slots; errors and panics leave entry defaults in place and drop all
staged ownership normally. The boundary owns commit, whose sealed output-slot
implementations perform only non-panicking raw writes and one-way ownership
transfers.

All non-null output locations in one call must occupy distinct, non-overlapping
storage. Before any output write, the boundary uses checked address arithmetic
to validate alignment and ranges and rejects detectable overlap with
`ROFD_STATUS_INVALID_ARGUMENT`. If only ordinary outputs overlap and `error` is
a separate valid slot, the ordinary outputs remain untouched and the failure is
published through `error`. If `error` overlaps any ordinary output, every output
remains untouched and no error handle is published. These fail-closed cases are
the exception to normal entry initialization.

Readable input regions and live handle storage must not overlap any output slot,
including the error slot. Input strings, options records, and handle storage
must remain valid for the entire call and must not be concurrently modified or
freed. Each document/page query borrows its live handle without consuming it;
the handle storage must be disjoint from that call's ordinary and error output
slots.

`error` is optional. If supplied, it is set to null on entry and receives one
newly owned error on failure. Callers must free an older error before reusing
its variable. Messages are descriptive but not parsed as an API; status and
diagnostic kind are the machine-readable contract.

The three options init functions and every free function are null-safe no-ops.
`rofd_error_get_status(NULL)` returns `ROFD_STATUS_INVALID_ARGUMENT`, while
`rofd_error_get_message(NULL)` returns null. `rofd_library_version()` returns
the static `rofd-ffi` package version.

Every exported entry point uses `catch_unwind`. No Rust panic or unwind may
cross into C. Void init/free functions also contain panics, but cannot report
them. Handles are never reconstructed with `Box::from_raw` except in their
matching free function. Const handle access borrows rather than consumes.

## Concurrency

Document, page, and renderer read operations may be called concurrently while
their handles remain live. Font and image caches retain their existing bounded,
thread-safe behavior. The caller must not race any operation with the matching
free function and must obey Cairo's own rules for a shared context or target
surface. There is no global last-error or mutable process-wide renderer state.

## Verification and Acceptance

Rust boundary tests cover null inputs, invalid UTF-8, unknown integer constants,
undersized and oversized option records, zeroed option padding, output
initialization, error mapping, diagnostic bounds, caught test panics, and every
permitted handle-release order. Portable layout tests cover stable prefixes and
alignment properties; Linux x86_64 tests assert the exact size, alignment, and
field offsets mirrored by the header.

A committed C program is compiled and linked against `librofd_ffi`. It opens
`learning/test.ofd`, observes one page and its `211.5 x 140 mm` box, obtains a
page, frees the document first, creates a Cairo image surface, queries the
`2115 x 1400` size at 254 DPI, renders, inspects diagnostics, and verifies stable
QR, title, and invoice-rule pixel regions. A C++ translation unit includes the
same header and exercises the declarations used later by Qt.

CI runs formatting, strict Clippy for all three reusable crates, Rust tests,
the C and C++ consumers, the real invoice render, and a defined-symbol allowlist
generated with `nm -D --defined-only`. The allowlist contains only approved
`rofd_*` entries. `cargo tree -p rofd-core` must still contain no Cairo,
font, image-codec, Qt, or FFI dependency.

Completion requires a clean worktree, no compiler warnings, no unwind crossing
the ABI, successful pure-C rendering of the real fixture, and documentation of
all ownership and compatibility rules in the installed header and crate README.
