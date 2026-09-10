use std::ffi::c_char;
use std::mem::{align_of, offset_of, size_of};
use std::panic::catch_unwind;
use std::ptr;

/// Status value returned by fallible rofd C ABI functions.
pub type rofd_status_t = u32;

/// The operation completed successfully.
pub const ROFD_STATUS_OK: rofd_status_t = 0;
/// An argument was invalid.
pub const ROFD_STATUS_INVALID_ARGUMENT: rofd_status_t = 1;
/// An input/output operation failed.
pub const ROFD_STATUS_IO: rofd_status_t = 2;
/// The input was not a valid OFD document.
pub const ROFD_STATUS_INVALID_DOCUMENT: rofd_status_t = 3;
/// The requested operation or document feature is unsupported.
pub const ROFD_STATUS_UNSUPPORTED: rofd_status_t = 4;
/// A configured resource limit was exceeded.
pub const ROFD_STATUS_LIMIT_EXCEEDED: rofd_status_t = 5;
/// The requested page index was out of range.
pub const ROFD_STATUS_PAGE_OUT_OF_RANGE: rofd_status_t = 6;
/// Rendering failed.
pub const ROFD_STATUS_RENDER_ERROR: rofd_status_t = 7;
/// Memory allocation failed.
pub const ROFD_STATUS_OUT_OF_MEMORY: rofd_status_t = 8;
/// An internal library error occurred.
pub const ROFD_STATUS_INTERNAL: rofd_status_t = 255;

/// Lenient document parsing mode.
pub const ROFD_STRICTNESS_LENIENT: u32 = 0;
/// Strict document parsing mode.
pub const ROFD_STRICTNESS_STRICT: u32 = 1;

/// A parse warning category unknown to this ABI version.
pub const ROFD_WARNING_UNKNOWN: u32 = 0;
/// A page inherited the document page area.
pub const ROFD_WARNING_PAGE_AREA_FALLBACK: u32 = 1;
/// The document omitted its default page area.
pub const ROFD_WARNING_DOCUMENT_PAGE_AREA_MISSING: u32 = 2;
/// An invalid signature or stamp annotation was skipped.
pub const ROFD_WARNING_SIGNATURE_SKIPPED: u32 = 3;
/// An unknown graphic unit was skipped.
pub const ROFD_WARNING_UNKNOWN_GRAPHIC_UNIT_SKIPPED: u32 = 4;
/// An invalid page annotation was skipped.
pub const ROFD_WARNING_ANNOTATION_SKIPPED: u32 = 5;
/// Historical document bodies were skipped.
pub const ROFD_WARNING_HISTORICAL_DOC_BODY_SKIPPED: u32 = 6;
/// A navigation entry or target was invalid or unresolved.
pub const ROFD_WARNING_NAVIGATION_INVALID: u32 = 7;
/// A navigation action, event or destination type was retained but is unsupported.
pub const ROFD_WARNING_NAVIGATION_UNSUPPORTED: u32 = 8;
/// A producer compatibility representation was used for navigation data.
pub const ROFD_WARNING_NAVIGATION_COMPATIBILITY: u32 = 9;

/// Sentinel for an absent node relation or unresolved page index.
pub const ROFD_NO_INDEX: usize = usize::MAX;
/// An unsupported action, whose source name remains available.
pub const ROFD_ACTION_UNKNOWN: u32 = 0;
/// A jump inside the current document.
pub const ROFD_ACTION_GOTO: u32 = 1;
/// A URI action; this library never opens it.
pub const ROFD_ACTION_URI: u32 = 2;
/// An action referring to a document attachment.
pub const ROFD_ACTION_ATTACHMENT: u32 = 3;
/// An unknown event, whose source name remains available.
pub const ROFD_ACTION_EVENT_UNKNOWN: u32 = 0;
/// The source action declares the document-open event (DO).
pub const ROFD_ACTION_EVENT_DOCUMENT_OPEN: u32 = 1;
/// The source action declares the page-open event (PO).
pub const ROFD_ACTION_EVENT_PAGE_OPEN: u32 = 2;
/// The source action declares the click event (CLICK).
pub const ROFD_ACTION_EVENT_CLICK: u32 = 3;
/// An attachment action requests a new window; absent source values default true.
pub const ROFD_ACTION_NEW_WINDOW: u32 = 1 << 0;
/// An unknown destination mode, whose source name remains available.
pub const ROFD_DESTINATION_UNKNOWN: u32 = 0;
/// Position at optional left/top coordinates and zoom.
pub const ROFD_DESTINATION_XYZ: u32 = 1;
/// Fit the destination page.
pub const ROFD_DESTINATION_FIT: u32 = 2;
/// Fit horizontally, with optional top position.
pub const ROFD_DESTINATION_FIT_H: u32 = 3;
/// Fit vertically, with optional left position.
pub const ROFD_DESTINATION_FIT_V: u32 = 4;
/// Fit the rectangle given by destination coordinates.
pub const ROFD_DESTINATION_FIT_R: u32 = 5;
/// The destination has a resolved zero-based page index.
pub const ROFD_DESTINATION_HAS_PAGE_INDEX: u32 = 1 << 0;
/// The destination retains an OFD page identifier, even if unresolved.
pub const ROFD_DESTINATION_HAS_PAGE_ID: u32 = 1 << 1;
/// The source explicitly specified a left coordinate.
pub const ROFD_DESTINATION_HAS_LEFT: u32 = 1 << 2;
/// The source explicitly specified a top coordinate.
pub const ROFD_DESTINATION_HAS_TOP: u32 = 1 << 3;
/// The source explicitly specified a right coordinate.
pub const ROFD_DESTINATION_HAS_RIGHT: u32 = 1 << 4;
/// The source explicitly specified a bottom coordinate.
pub const ROFD_DESTINATION_HAS_BOTTOM: u32 = 1 << 5;
/// The source explicitly specified zoom; zero means retain current zoom.
pub const ROFD_DESTINATION_HAS_ZOOM: u32 = 1 << 6;

/// Nearest-neighbor image interpolation.
pub const ROFD_IMAGE_INTERPOLATION_NEAREST: u32 = 0;
/// Bilinear image interpolation.
pub const ROFD_IMAGE_INTERPOLATION_BILINEAR: u32 = 1;

/// Enables case-sensitive literal text search.
pub const ROFD_FIND_CASE_SENSITIVE: u32 = 1 << 0;
/// Restricts text search results to whole words.
pub const ROFD_FIND_WHOLE_WORDS: u32 = 1 << 1;

/// Marks a text character inserted while flattening source text objects.
pub const ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR: u32 = 1 << 0;
/// Marks a text character whose geometry is conservative rather than exact.
pub const ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY: u32 = 1 << 1;

/// Selects only glyphs intersecting a selection rectangle.
pub const ROFD_SELECTION_GLYPH: u32 = 0;
/// Expands a selection rectangle to complete words.
pub const ROFD_SELECTION_WORD: u32 = 1;
/// Expands a selection rectangle to complete logical lines.
pub const ROFD_SELECTION_LINE: u32 = 2;

/// Diagnostic indicating an unsupported object.
pub const ROFD_DIAGNOSTIC_UNSUPPORTED_OBJECT: u32 = 1;
/// Diagnostic indicating that a fallback font was used.
pub const ROFD_DIAGNOSTIC_FONT_FALLBACK: u32 = 2;
/// Diagnostic indicating a missing glyph.
pub const ROFD_DIAGNOSTIC_MISSING_GLYPH: u32 = 3;
/// Diagnostic indicating unsupported image substitution.
pub const ROFD_DIAGNOSTIC_IMAGE_SUBSTITUTION_UNSUPPORTED: u32 = 4;
/// Diagnostic indicating an unsupported image mask.
pub const ROFD_DIAGNOSTIC_IMAGE_MASK_UNSUPPORTED: u32 = 5;
/// Diagnostic indicating an unsupported image border.
pub const ROFD_DIAGNOSTIC_IMAGE_BORDER_UNSUPPORTED: u32 = 6;

/// Current version of the rofd C ABI.
pub const ROFD_ABI_VERSION: u32 = 1;

/// Options used when loading an OFD document.
#[repr(C)]
pub struct rofd_load_options_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Document parsing strictness.
    pub strictness: u32,
}

impl Default for rofd_load_options_t {
    fn default() -> Self {
        Self {
            struct_size: record_size::<Self>(),
            strictness: ROFD_STRICTNESS_LENIENT,
        }
    }
}

/// Options used when searching canonical page text.
#[repr(C)]
pub struct rofd_find_options_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Text search behavior represented by `ROFD_FIND_*` flags.
    pub flags: u32,
    /// Maximum number of matches returned by a search.
    pub max_results: usize,
}

/// Options used when constructing a renderer.
#[repr(C)]
pub struct rofd_renderer_options_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Array of pointers to NUL-terminated UTF-8 fallback family names.
    ///
    /// A null pointer paired with a zero count selects built-in defaults.
    pub fallback_families: *const *const c_char,
    /// Number of pointers in `fallback_families`.
    pub fallback_family_count: usize,
    /// Maximum number of font bytes retained by the renderer.
    pub max_font_bytes: u64,
    /// Maximum number of decoded image bytes retained by the renderer.
    pub image_cache_bytes: u64,
}

impl Default for rofd_renderer_options_t {
    fn default() -> Self {
        Self {
            struct_size: record_size::<Self>(),
            fallback_families: ptr::null(),
            fallback_family_count: 0,
            max_font_bytes: 64 * 1024 * 1024,
            image_cache_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Options used when rendering an OFD page.
#[repr(C)]
pub struct rofd_render_options_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Target resolution in dots per inch.
    pub dpi: f64,
    /// Additional output scaling factor.
    pub scale: f64,
    /// Clockwise rotation in degrees.
    pub rotation_degrees: u32,
    /// Background color encoded as RGBA bytes.
    pub background_rgba: u32,
    /// Whether the clipping rectangle is enabled.
    pub has_clip: u32,
    /// Clipping rectangle X coordinate in millimetres.
    pub clip_x_mm: f64,
    /// Clipping rectangle Y coordinate in millimetres.
    pub clip_y_mm: f64,
    /// Clipping rectangle width in millimetres.
    pub clip_width_mm: f64,
    /// Clipping rectangle height in millimetres.
    pub clip_height_mm: f64,
    /// Image interpolation mode.
    pub image_interpolation: u32,
    /// Maximum number of bytes available for raster output.
    pub max_raster_bytes: u64,
}

impl Default for rofd_render_options_t {
    fn default() -> Self {
        Self {
            struct_size: record_size::<Self>(),
            dpi: 96.0,
            scale: 1.0,
            rotation_degrees: 0,
            background_rgba: 0xffff_ffff,
            has_clip: 0,
            clip_x_mm: 0.0,
            clip_y_mm: 0.0,
            clip_width_mm: 0.0,
            clip_height_mm: 0.0,
            image_interpolation: ROFD_IMAGE_INTERPOLATION_BILINEAR,
            max_raster_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Viewport in the final rotated and scaled full-page pixel canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct rofd_pixel_rect_t {
    /// Size of this initialized record in bytes.
    pub struct_size: u32,
    /// Nonnegative horizontal pixel origin.
    pub x: i32,
    /// Nonnegative vertical pixel origin.
    pub y: i32,
    /// Positive number of columns.
    pub width: i32,
    /// Positive number of rows.
    pub height: i32,
}

/// Rectangle expressed in millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct rofd_rect_t {
    /// X coordinate in millimetres.
    pub x_mm: f64,
    /// Y coordinate in millimetres.
    pub y_mm: f64,
    /// Width in millimetres.
    pub width_mm: f64,
    /// Height in millimetres.
    pub height_mm: f64,
}

/// One Unicode scalar and its source metadata in canonical page text.
#[repr(C)]
pub struct rofd_text_char_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Byte offset of the scalar in canonical UTF-8 page text.
    pub utf8_offset: usize,
    /// Byte length of the scalar in canonical UTF-8 page text.
    pub utf8_length: usize,
    /// Character geometry in page millimetres.
    pub rect_mm: rofd_rect_t,
    /// Character properties represented by `ROFD_TEXT_CHAR_*` flags.
    pub flags: u32,
    /// Identifier of the source text object, or zero for a synthesized character.
    pub object_id: u64,
}

/// One literal text search match in canonical page text.
#[repr(C)]
pub struct rofd_text_match_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Byte offset of the match in canonical UTF-8 page text.
    pub utf8_offset: usize,
    /// Byte length of the match in canonical UTF-8 page text.
    pub utf8_length: usize,
    /// Union of visible matched character boxes in page millimetres.
    pub rect_mm: rofd_rect_t,
}

/// One rendering diagnostic exposed through the C ABI.
#[repr(C)]
pub struct rofd_render_diagnostic_t {
    /// Size of this record in bytes.
    pub struct_size: u32,
    /// Diagnostic kind, represented by a `ROFD_DIAGNOSTIC_*` constant.
    pub kind: u32,
    /// Identifier of the object associated with the diagnostic.
    pub object_id: u64,
    /// Borrowed NUL-terminated UTF-8 diagnostic message.
    pub message: *const c_char,
}

/// One parse warning borrowed from an independently owned warning snapshot.
#[repr(C)]
pub struct rofd_warning_t {
    /// Caller-provided record size; must cover the complete v1 prefix.
    pub struct_size: u32,
    /// Stable category represented by a `ROFD_WARNING_*` constant.
    pub code: u32,
    /// Borrowed NUL-terminated UTF-8 package path, valid until the snapshot is freed.
    pub path: *const c_char,
    /// Borrowed NUL-terminated UTF-8 explanation, valid until the snapshot is freed.
    pub message: *const c_char,
}

/// One node borrowed from an independently owned preorder outline snapshot.
#[repr(C)]
pub struct rofd_outline_node_t {
    /// Caller-provided size covering the complete v1 prefix.
    pub struct_size: u32,
    /// One for expanded, zero for collapsed; omitted source values default expanded.
    pub expanded: u32,
    /// Borrowed NUL-terminated UTF-8 title, valid until the outline is freed.
    pub title: *const c_char,
    /// Parent preorder index, or [`ROFD_NO_INDEX`] for a root.
    pub parent: usize,
    /// First child preorder index, or [`ROFD_NO_INDEX`].
    pub first_child: usize,
    /// Next sibling preorder index, or [`ROFD_NO_INDEX`].
    pub next_sibling: usize,
    /// Number of ordered actions on this node.
    pub action_count: usize,
}

/// One inert action borrowed from its outline or page-link snapshot.
#[repr(C)]
pub struct rofd_action_t {
    /// Caller-provided size covering the complete v1 prefix.
    pub struct_size: u32,
    /// Action category represented by `ROFD_ACTION_*`.
    pub kind: u32,
    /// Event category represented by `ROFD_ACTION_EVENT_*`.
    pub event: u32,
    /// Optional behavior flags, currently [`ROFD_ACTION_NEW_WINDOW`].
    pub flags: u32,
    /// Borrowed source action name, including unsupported names.
    pub type_name: *const c_char,
    /// Borrowed source event name, including unsupported names.
    pub event_name: *const c_char,
    /// Borrowed URI for a URI action, otherwise null; no URI is executed.
    pub uri: *const c_char,
    /// Borrowed optional URI base, otherwise null; resolution belongs to the caller.
    pub uri_base: *const c_char,
    /// Borrowed attachment identifier for GotoA, otherwise null.
    pub attachment_id: *const c_char,
    /// Borrowed named-bookmark reference for Goto, otherwise null.
    pub bookmark: *const c_char,
}

/// Optional destination information borrowed from an action's owning snapshot.
#[repr(C)]
pub struct rofd_destination_t {
    /// Caller-provided size covering the complete v1 prefix.
    pub struct_size: u32,
    /// Destination mode represented by `ROFD_DESTINATION_*`.
    pub kind: u32,
    /// Presence bits represented by `ROFD_DESTINATION_HAS_*`.
    pub flags: u32,
    /// Resolved zero-based page index, or [`ROFD_NO_INDEX`] on successful unresolved queries.
    pub page_index: usize,
    /// Original OFD page identifier when HAS_PAGE_ID is set, otherwise zero.
    pub page_id: u64,
    /// Borrowed destination mode name, or null when no destination is available.
    pub mode_name: *const c_char,
    /// Optional source left coordinate in physical-page millimetres.
    pub left_mm: f64,
    /// Optional source top coordinate in physical-page millimetres.
    pub top_mm: f64,
    /// Optional source right coordinate in physical-page millimetres.
    pub right_mm: f64,
    /// Optional source bottom coordinate in physical-page millimetres.
    pub bottom_mm: f64,
    /// Optional source zoom; zero means retain the current zoom.
    pub zoom: f64,
}

// Each published size boundary includes trailing padding and must never change when fields are
// appended. In particular, future fields must not reuse padding before one of these boundaries.
pub(crate) const ROFD_LOAD_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_load_options_t, strictness) + size_of::<u32>(),
    align_of::<u32>(),
);
pub(crate) const ROFD_RENDERER_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_renderer_options_t, image_cache_bytes) + size_of::<u64>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<*const *const c_char>(),
        align_of::<usize>(),
        align_of::<u64>(),
    ]),
);
pub(crate) const ROFD_RENDER_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_render_options_t, max_raster_bytes) + size_of::<u64>(),
    max_alignment(&[align_of::<u32>(), align_of::<f64>(), align_of::<u64>()]),
);
pub(crate) const ROFD_FIND_OPTIONS_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_find_options_t, max_results) + size_of::<usize>(),
    max_alignment(&[align_of::<u32>(), align_of::<usize>()]),
);
#[allow(dead_code)] // Consumed by semantic output accessors added in the next task.
pub(crate) const ROFD_TEXT_CHAR_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_text_char_t, object_id) + size_of::<u64>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<usize>(),
        align_of::<rofd_rect_t>(),
        align_of::<u64>(),
    ]),
);
pub(crate) const ROFD_TEXT_MATCH_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_text_match_t, rect_mm) + size_of::<rofd_rect_t>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<usize>(),
        align_of::<rofd_rect_t>(),
    ]),
);
pub(crate) const ROFD_RENDER_DIAGNOSTIC_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_render_diagnostic_t, message) + size_of::<*const c_char>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<u64>(),
        align_of::<*const c_char>(),
    ]),
);
pub(crate) const ROFD_WARNING_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_warning_t, message) + size_of::<*const c_char>(),
    max_alignment(&[align_of::<u32>(), align_of::<*const c_char>()]),
);
pub(crate) const ROFD_OUTLINE_NODE_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_outline_node_t, action_count) + size_of::<usize>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<usize>(),
        align_of::<*const c_char>(),
    ]),
);
pub(crate) const ROFD_ACTION_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_action_t, bookmark) + size_of::<*const c_char>(),
    max_alignment(&[align_of::<u32>(), align_of::<*const c_char>()]),
);
pub(crate) const ROFD_DESTINATION_V1_SIZE: usize = c_record_size(
    offset_of!(rofd_destination_t, zoom) + size_of::<f64>(),
    max_alignment(&[
        align_of::<u32>(),
        align_of::<usize>(),
        align_of::<u64>(),
        align_of::<*const c_char>(),
        align_of::<f64>(),
    ]),
);

const ROFD_LOAD_OPTIONS_VERSION_SIZES: &[usize] = &[ROFD_LOAD_OPTIONS_V1_SIZE];
const ROFD_RENDERER_OPTIONS_VERSION_SIZES: &[usize] = &[ROFD_RENDERER_OPTIONS_V1_SIZE];
const ROFD_RENDER_OPTIONS_VERSION_SIZES: &[usize] = &[ROFD_RENDER_OPTIONS_V1_SIZE];
const ROFD_FIND_OPTIONS_VERSION_SIZES: &[usize] = &[ROFD_FIND_OPTIONS_V1_SIZE];
pub(crate) const ROFD_PIXEL_RECT_V1_SIZE: usize = 20;
const ROFD_PIXEL_RECT_VERSION_SIZES: &[usize] = &[ROFD_PIXEL_RECT_V1_SIZE];

static LIBRARY_VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

fn record_size<T>() -> u32 {
    u32::try_from(size_of::<T>()).expect("ABI record size must fit in u32")
}

const fn c_record_size(last_field_end: usize, alignment: usize) -> usize {
    last_field_end.div_ceil(alignment) * alignment
}

const fn max_alignment(alignments: &[usize]) -> usize {
    let mut maximum = 1;
    let mut index = 0;
    while index < alignments.len() {
        if alignments[index] > maximum {
            maximum = alignments[index];
        }
        index += 1;
    }
    maximum
}

fn highest_supported_version_size(options_size: usize, version_sizes: &[usize]) -> Option<usize> {
    version_sizes
        .iter()
        .copied()
        .filter(|version_size| *version_size <= options_size)
        .max()
}

/// Returns the version of the rofd C ABI.
#[no_mangle]
pub extern "C" fn rofd_abi_version() -> u32 {
    catch_unwind(|| ROFD_ABI_VERSION).unwrap_or(0)
}

/// Returns the library crate version as a static NUL-terminated string.
#[no_mangle]
pub extern "C" fn rofd_library_version() -> *const c_char {
    catch_unwind(|| LIBRARY_VERSION.as_ptr().cast()).unwrap_or(ptr::null())
}

/// Initializes the known document load options prefix to its defaults.
///
/// A null pointer or a capacity smaller than the oldest supported version is a
/// no-op. Otherwise the highest complete supported version that fits is
/// initialized, and bytes beyond that permanent version boundary are unchanged.
/// The selected boundary, not caller capacity, is written to `struct_size`.
///
/// # Safety
///
/// A null `options` is always accepted and is a no-op, regardless of
/// `options_size`. If `options` is non-null and `options_size` can hold a
/// supported version, it must be properly aligned and point to valid, writable
/// storage for at least the selected version prefix. A non-null pointer is not
/// dereferenced when no supported version fits.
#[no_mangle]
pub unsafe extern "C" fn rofd_load_options_init(
    options: *mut rofd_load_options_t,
    options_size: usize,
) {
    let _ = catch_unwind(|| {
        if options.is_null() {
            return;
        }
        let Some(initialized_size) =
            highest_supported_version_size(options_size, ROFD_LOAD_OPTIONS_VERSION_SIZES)
        else {
            return;
        };

        // SAFETY: Version selection and the caller contract guarantee a valid, writable prefix.
        // Clearing the prefix first initializes every padding byte; field writes do not touch
        // any unknown trailing bytes in a larger caller allocation.
        unsafe {
            options.cast::<u8>().write_bytes(0, initialized_size);
            ptr::addr_of_mut!((*options).struct_size)
                .write(u32::try_from(initialized_size).expect("ABI version size must fit in u32"));
            // Every selectable version currently includes the complete v1 field set.
            ptr::addr_of_mut!((*options).strictness).write(ROFD_STRICTNESS_LENIENT);
        }
    });
}

/// Initializes the known text search options prefix to its defaults.
///
/// A null pointer or a capacity smaller than the oldest supported version is a
/// no-op. Otherwise the highest complete supported version that fits is
/// initialized, and bytes beyond that permanent version boundary are unchanged.
/// The selected boundary, not caller capacity, is written to `struct_size`.
///
/// # Safety
///
/// A null `options` is always accepted and is a no-op, regardless of
/// `options_size`. If `options` is non-null and `options_size` can hold a
/// supported version, it must be properly aligned and point to valid, writable
/// storage for at least the selected version prefix. A non-null pointer is not
/// dereferenced when no supported version fits.
#[no_mangle]
pub unsafe extern "C" fn rofd_find_options_init(
    options: *mut rofd_find_options_t,
    options_size: usize,
) {
    let _ = catch_unwind(|| {
        if options.is_null() {
            return;
        }
        let Some(initialized_size) =
            highest_supported_version_size(options_size, ROFD_FIND_OPTIONS_VERSION_SIZES)
        else {
            return;
        };

        // SAFETY: Version selection and the caller contract guarantee a valid, writable prefix.
        // Clearing the prefix first initializes every padding byte; field writes do not touch
        // any unknown trailing bytes in a larger caller allocation.
        unsafe {
            options.cast::<u8>().write_bytes(0, initialized_size);
            ptr::addr_of_mut!((*options).struct_size)
                .write(u32::try_from(initialized_size).expect("ABI version size must fit in u32"));
            // Every selectable version currently includes the complete v1 field set.
            ptr::addr_of_mut!((*options).flags).write(0);
            ptr::addr_of_mut!((*options).max_results).write(10_000);
        }
    });
}

/// Initializes the known renderer construction options prefix to its defaults.
///
/// A null pointer or a capacity smaller than the oldest supported version is a
/// no-op. Otherwise the highest complete supported version that fits is
/// initialized, and bytes beyond that permanent version boundary are unchanged.
/// The selected boundary, not caller capacity, is written to `struct_size`.
///
/// # Safety
///
/// A null `options` is always accepted and is a no-op, regardless of
/// `options_size`. If `options` is non-null and `options_size` can hold a
/// supported version, it must be properly aligned and point to valid, writable
/// storage for at least the selected version prefix. A non-null pointer is not
/// dereferenced when no supported version fits.
#[no_mangle]
pub unsafe extern "C" fn rofd_renderer_options_init(
    options: *mut rofd_renderer_options_t,
    options_size: usize,
) {
    let _ = catch_unwind(|| {
        if options.is_null() {
            return;
        }
        let Some(initialized_size) =
            highest_supported_version_size(options_size, ROFD_RENDERER_OPTIONS_VERSION_SIZES)
        else {
            return;
        };

        // SAFETY: Version selection and the caller contract guarantee a valid, writable prefix.
        // Clearing the prefix first initializes every padding byte; field writes do not touch
        // any unknown trailing bytes in a larger caller allocation.
        unsafe {
            options.cast::<u8>().write_bytes(0, initialized_size);
            ptr::addr_of_mut!((*options).struct_size)
                .write(u32::try_from(initialized_size).expect("ABI version size must fit in u32"));
            // Every selectable version currently includes the complete v1 field set.
            ptr::addr_of_mut!((*options).fallback_families).write(ptr::null());
            ptr::addr_of_mut!((*options).fallback_family_count).write(0);
            ptr::addr_of_mut!((*options).max_font_bytes).write(64 * 1024 * 1024);
            ptr::addr_of_mut!((*options).image_cache_bytes).write(64 * 1024 * 1024);
        }
    });
}

/// Initializes the known page rendering options prefix to its defaults.
///
/// A null pointer or a capacity smaller than the oldest supported version is a
/// no-op. Otherwise the highest complete supported version that fits is
/// initialized, and bytes beyond that permanent version boundary are unchanged.
/// The selected boundary, not caller capacity, is written to `struct_size`.
///
/// # Safety
///
/// A null `options` is always accepted and is a no-op, regardless of
/// `options_size`. If `options` is non-null and `options_size` can hold a
/// supported version, it must be properly aligned and point to valid, writable
/// storage for at least the selected version prefix. A non-null pointer is not
/// dereferenced when no supported version fits.
#[no_mangle]
pub unsafe extern "C" fn rofd_render_options_init(
    options: *mut rofd_render_options_t,
    options_size: usize,
) {
    let _ = catch_unwind(|| {
        if options.is_null() {
            return;
        }
        let Some(initialized_size) =
            highest_supported_version_size(options_size, ROFD_RENDER_OPTIONS_VERSION_SIZES)
        else {
            return;
        };

        // SAFETY: Version selection and the caller contract guarantee a valid, writable prefix.
        // Clearing the prefix first initializes every padding byte; field writes do not touch
        // any unknown trailing bytes in a larger caller allocation.
        unsafe {
            options.cast::<u8>().write_bytes(0, initialized_size);
            ptr::addr_of_mut!((*options).struct_size)
                .write(u32::try_from(initialized_size).expect("ABI version size must fit in u32"));
            // Every selectable version currently includes the complete v1 field set.
            ptr::addr_of_mut!((*options).dpi).write(96.0);
            ptr::addr_of_mut!((*options).scale).write(1.0);
            ptr::addr_of_mut!((*options).rotation_degrees).write(0);
            ptr::addr_of_mut!((*options).background_rgba).write(0xffff_ffff);
            ptr::addr_of_mut!((*options).has_clip).write(0);
            ptr::addr_of_mut!((*options).clip_x_mm).write(0.0);
            ptr::addr_of_mut!((*options).clip_y_mm).write(0.0);
            ptr::addr_of_mut!((*options).clip_width_mm).write(0.0);
            ptr::addr_of_mut!((*options).clip_height_mm).write(0.0);
            ptr::addr_of_mut!((*options).image_interpolation)
                .write(ROFD_IMAGE_INTERPOLATION_BILINEAR);
            ptr::addr_of_mut!((*options).max_raster_bytes).write(256 * 1024 * 1024);
        }
    });
}

/// Initializes a pixel viewport to an empty rectangle.
///
/// Set positive width/height before rendering. Null or undersized storage is
/// untouched. Only the highest supported prefix is initialized; unknown tails
/// are preserved and `struct_size` receives the selected version boundary.
///
/// # Safety
/// A non-null pointer whose capacity holds a supported version must be aligned
/// and writable for that complete prefix, with no conflicting concurrent access.
#[no_mangle]
pub unsafe extern "C" fn rofd_pixel_rect_init(rect: *mut rofd_pixel_rect_t, capacity: usize) {
    let _ = catch_unwind(|| {
        if rect.is_null() {
            return;
        }
        let Some(size) = highest_supported_version_size(capacity, ROFD_PIXEL_RECT_VERSION_SIZES)
        else {
            return;
        };
        // SAFETY: The caller guarantees writable aligned storage for the selected prefix.
        unsafe {
            rect.cast::<u8>().write_bytes(0, size);
            ptr::addr_of_mut!((*rect).struct_size).write(size as u32);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_records_match_their_permanent_v1_boundaries() {
        assert_eq!(ROFD_OUTLINE_NODE_V1_SIZE, size_of::<rofd_outline_node_t>());
        assert_eq!(ROFD_ACTION_V1_SIZE, size_of::<rofd_action_t>());
        assert_eq!(ROFD_DESTINATION_V1_SIZE, size_of::<rofd_destination_t>());
        assert_eq!(ROFD_WARNING_V1_SIZE, size_of::<rofd_warning_t>());
        assert_eq!(ROFD_PIXEL_RECT_V1_SIZE, size_of::<rofd_pixel_rect_t>());
        assert_eq!(ROFD_LOAD_OPTIONS_V1_SIZE, size_of::<rofd_load_options_t>());
        assert_eq!(
            ROFD_RENDERER_OPTIONS_V1_SIZE,
            size_of::<rofd_renderer_options_t>()
        );
        assert_eq!(
            ROFD_RENDER_OPTIONS_V1_SIZE,
            size_of::<rofd_render_options_t>()
        );
        assert_eq!(ROFD_FIND_OPTIONS_V1_SIZE, size_of::<rofd_find_options_t>());
        assert_eq!(ROFD_TEXT_CHAR_V1_SIZE, size_of::<rofd_text_char_t>());
        assert_eq!(ROFD_TEXT_MATCH_V1_SIZE, size_of::<rofd_text_match_t>());
    }

    #[test]
    fn version_selection_chooses_the_highest_complete_known_version() {
        const V1_SIZE: usize = 24;
        const V2_SIZE: usize = 40;
        let versions = [V1_SIZE, V2_SIZE];

        assert_eq!(highest_supported_version_size(V1_SIZE - 1, &versions), None);
        assert_eq!(
            highest_supported_version_size(V1_SIZE, &versions),
            Some(V1_SIZE)
        );
        assert_eq!(
            highest_supported_version_size(V2_SIZE - 1, &versions),
            Some(V1_SIZE)
        );
        assert_eq!(
            highest_supported_version_size(V2_SIZE, &versions),
            Some(V2_SIZE)
        );
        assert_eq!(
            highest_supported_version_size(V2_SIZE + 16, &versions),
            Some(V2_SIZE)
        );
    }
}
