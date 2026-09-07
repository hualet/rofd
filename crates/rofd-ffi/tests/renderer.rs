use std::ffi::CString;
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;

use rofd_ffi::{
    rofd_document_free, rofd_document_get_page, rofd_document_open, rofd_document_t,
    rofd_error_free, rofd_error_t, rofd_page_free, rofd_page_t, rofd_render_options_init,
    rofd_render_options_t, rofd_renderer_free, rofd_renderer_get_pixel_size, rofd_renderer_new,
    rofd_renderer_options_init, rofd_renderer_options_t, rofd_renderer_t,
    ROFD_IMAGE_INTERPOLATION_BILINEAR, ROFD_STATUS_INVALID_ARGUMENT, ROFD_STATUS_LIMIT_EXCEEDED,
    ROFD_STATUS_OK,
};

fn fixture_path() -> CString {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    CString::new(path.to_str().expect("fixture path must be UTF-8")).unwrap()
}

unsafe fn open_fixture_page() -> (*mut rofd_document_t, *mut rofd_page_t) {
    let mut document = ptr::null_mut();
    let mut page = ptr::null_mut();
    let mut error = ptr::null_mut();
    let path = fixture_path();
    // SAFETY: Inputs and distinct outputs remain live for each call.
    assert_eq!(
        unsafe { rofd_document_open(path.as_ptr(), ptr::null(), &mut document, &mut error) },
        ROFD_STATUS_OK
    );
    // SAFETY: The document is live and both outputs are distinct.
    assert_eq!(
        unsafe { rofd_document_get_page(document, 0, &mut page, &mut error) },
        ROFD_STATUS_OK
    );
    (document, page)
}

unsafe fn free_error(error: &mut *mut rofd_error_t) {
    if !error.is_null() {
        // SAFETY: This error handle is owned and is released exactly once.
        unsafe { rofd_error_free(*error) };
        *error = ptr::null_mut();
    }
}

unsafe fn new_renderer(
    options: *const rofd_renderer_options_t,
) -> Result<*mut rofd_renderer_t, u32> {
    let mut renderer = ptr::null_mut();
    let mut error = ptr::null_mut();
    // SAFETY: The options follow their record contract and outputs are distinct.
    let status = unsafe { rofd_renderer_new(options, &mut renderer, &mut error) };
    // SAFETY: Any published error is owned by this helper.
    unsafe { free_error(&mut error) };
    if status == ROFD_STATUS_OK {
        Ok(renderer)
    } else {
        assert!(renderer.is_null());
        Err(status)
    }
}

fn initialized_renderer_options() -> rofd_renderer_options_t {
    let mut options = rofd_renderer_options_t {
        struct_size: 0,
        fallback_families: ptr::null(),
        fallback_family_count: 0,
        max_font_bytes: 0,
        image_cache_bytes: 0,
    };
    // SAFETY: `options` is writable for its full v1 size.
    unsafe { rofd_renderer_options_init(&mut options, size_of::<rofd_renderer_options_t>()) };
    options
}

fn initialized_render_options() -> rofd_render_options_t {
    let mut options = rofd_render_options_t {
        struct_size: 0,
        dpi: 0.0,
        scale: 0.0,
        rotation_degrees: 0,
        background_rgba: 0,
        has_clip: 0,
        clip_x_mm: 0.0,
        clip_y_mm: 0.0,
        clip_width_mm: 0.0,
        clip_height_mm: 0.0,
        image_interpolation: 0,
        max_raster_bytes: 0,
    };
    // SAFETY: `options` is writable for its full v1 size.
    unsafe { rofd_render_options_init(&mut options, size_of::<rofd_render_options_t>()) };
    options
}

#[test]
fn pixel_size_at_254_dpi_matches_real_fixture() {
    // SAFETY: Every handle returned below remains live through its uses and is freed once.
    unsafe {
        let (document, page) = open_fixture_page();
        let renderer = new_renderer(ptr::null()).unwrap();
        let mut options = initialized_render_options();
        options.dpi = 254.0;
        let mut width = -1;
        let mut height = -1;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                &options,
                &mut width,
                &mut height,
                &mut error,
            ),
            ROFD_STATUS_OK
        );
        assert_eq!((width, height), (2115, 1400));
        assert!(error.is_null());
        rofd_renderer_free(renderer);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[repr(C)]
struct ExtendedRendererOptions {
    v1: rofd_renderer_options_t,
    future_tail: [u64; 2],
}

#[repr(C)]
struct ExtendedRenderOptions {
    v1: rofd_render_options_t,
    future_tail: [u64; 2],
}

#[test]
fn option_records_require_v1_and_ignore_oversized_tails() {
    let mut undersized = initialized_renderer_options();
    undersized.struct_size = (size_of::<rofd_renderer_options_t>() - 1) as u32;
    assert_eq!(
        unsafe { new_renderer(&undersized) },
        Err(ROFD_STATUS_INVALID_ARGUMENT)
    );

    let mut extended = ExtendedRendererOptions {
        v1: initialized_renderer_options(),
        future_tail: [0xa5a5_a5a5_a5a5_a5a5; 2],
    };
    extended.v1.struct_size = size_of::<ExtendedRendererOptions>() as u32;
    let renderer = unsafe { new_renderer(&extended.v1) }.unwrap();
    assert_eq!(extended.future_tail, [0xa5a5_a5a5_a5a5_a5a5; 2]);

    // SAFETY: Handles and output slots are live and disjoint.
    unsafe {
        let (document, page) = open_fixture_page();
        let mut render = ExtendedRenderOptions {
            v1: initialized_render_options(),
            future_tail: [0x5a5a_5a5a_5a5a_5a5a; 2],
        };
        render.v1.struct_size = size_of::<ExtendedRenderOptions>() as u32;
        render.v1.dpi = 254.0;
        let mut width = -1;
        let mut height = -1;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                &render.v1,
                &mut width,
                &mut height,
                &mut error,
            ),
            ROFD_STATUS_OK
        );
        assert_eq!((width, height), (2115, 1400));
        assert_eq!(render.future_tail, [0x5a5a_5a5a_5a5a_5a5a; 2]);
        rofd_renderer_free(renderer);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[test]
fn renderer_options_reject_zero_limits() {
    for mutate in [
        |options: &mut rofd_renderer_options_t| options.max_font_bytes = 0,
        |options: &mut rofd_renderer_options_t| options.image_cache_bytes = 0,
    ] {
        let mut options = initialized_renderer_options();
        mutate(&mut options);
        assert_eq!(
            unsafe { new_renderer(&options) },
            Err(ROFD_STATUS_INVALID_ARGUMENT)
        );
    }
}

#[test]
fn fallback_family_four_states_are_validated_and_copied() {
    let defaults = initialized_renderer_options();
    for options in [ptr::null(), &defaults] {
        let renderer = unsafe { new_renderer(options) }.unwrap();
        // SAFETY: Each returned renderer is owned and released exactly once.
        unsafe { rofd_renderer_free(renderer) };
    }

    let empty_marker = CString::new("unused").unwrap();
    let empty_array = [empty_marker.as_ptr()];
    let mut disabled = initialized_renderer_options();
    disabled.fallback_families = empty_array.as_ptr();
    disabled.fallback_family_count = 0;
    let renderer = unsafe { new_renderer(&disabled) }.unwrap();
    // SAFETY: The renderer is owned and released exactly once.
    unsafe { rofd_renderer_free(renderer) };

    let family = CString::new("DejaVu Sans").unwrap();
    let families = [family.as_ptr()];
    let mut explicit = initialized_renderer_options();
    explicit.fallback_families = families.as_ptr();
    explicit.fallback_family_count = families.len();
    let renderer = unsafe { new_renderer(&explicit) }.unwrap();
    drop(family);
    // SAFETY: Construction copied the family; the renderer is independently owned.
    unsafe { rofd_renderer_free(renderer) };

    let mut missing_array = initialized_renderer_options();
    missing_array.fallback_family_count = 1;
    assert_eq!(
        unsafe { new_renderer(&missing_array) },
        Err(ROFD_STATUS_INVALID_ARGUMENT)
    );

    let null_families = [ptr::null()];
    let mut null_element = initialized_renderer_options();
    null_element.fallback_families = null_families.as_ptr();
    null_element.fallback_family_count = 1;
    assert_eq!(
        unsafe { new_renderer(&null_element) },
        Err(ROFD_STATUS_INVALID_ARGUMENT)
    );

    let invalid_utf8 = [0xff_u8, 0];
    let invalid_families = [invalid_utf8.as_ptr().cast()];
    let mut invalid_element = initialized_renderer_options();
    invalid_element.fallback_families = invalid_families.as_ptr();
    invalid_element.fallback_family_count = 1;
    assert_eq!(
        unsafe { new_renderer(&invalid_element) },
        Err(ROFD_STATUS_INVALID_ARGUMENT)
    );

    let boundary_family = CString::new("DejaVu Sans").unwrap();
    let boundary_families = vec![boundary_family.as_ptr(); 1024];
    let mut boundary = initialized_renderer_options();
    boundary.fallback_families = boundary_families.as_ptr();
    boundary.fallback_family_count = boundary_families.len();
    let renderer = unsafe { new_renderer(&boundary) }.unwrap();
    // SAFETY: The boundary-size renderer is owned and released exactly once.
    unsafe { rofd_renderer_free(renderer) };

    let excessive_families = vec![boundary_family.as_ptr(); 1025];
    let mut excessive = initialized_renderer_options();
    excessive.fallback_families = excessive_families.as_ptr();
    excessive.fallback_family_count = excessive_families.len();
    assert_eq!(
        unsafe { new_renderer(&excessive) },
        Err(ROFD_STATUS_INVALID_ARGUMENT)
    );
}

#[test]
fn render_options_validate_values_and_clear_all_outputs_on_failure() {
    // SAFETY: Handles remain live and outputs are distinct throughout the test.
    unsafe {
        let (document, page) = open_fixture_page();
        let renderer = new_renderer(ptr::null()).unwrap();
        let base = initialized_render_options();
        let cases = [
            (
                rofd_render_options_t {
                    image_interpolation: ROFD_IMAGE_INTERPOLATION_BILINEAR + 1,
                    ..base
                },
                ROFD_STATUS_INVALID_ARGUMENT,
            ),
            (
                rofd_render_options_t {
                    has_clip: 2,
                    ..base
                },
                ROFD_STATUS_INVALID_ARGUMENT,
            ),
            (
                rofd_render_options_t { dpi: 0.0, ..base },
                ROFD_STATUS_INVALID_ARGUMENT,
            ),
            (
                rofd_render_options_t { scale: 0.0, ..base },
                ROFD_STATUS_INVALID_ARGUMENT,
            ),
            (
                rofd_render_options_t {
                    rotation_degrees: 45,
                    ..base
                },
                ROFD_STATUS_INVALID_ARGUMENT,
            ),
            (
                rofd_render_options_t {
                    has_clip: 1,
                    clip_width_mm: 0.0,
                    clip_height_mm: 1.0,
                    ..base
                },
                ROFD_STATUS_INVALID_ARGUMENT,
            ),
            (
                rofd_render_options_t {
                    max_raster_bytes: 1,
                    ..base
                },
                ROFD_STATUS_LIMIT_EXCEEDED,
            ),
        ];
        for (options, expected_status) in cases {
            let mut width = -1;
            let mut height = -1;
            let mut error = ptr::null_mut();
            let status = rofd_renderer_get_pixel_size(
                renderer,
                page,
                &options,
                &mut width,
                &mut height,
                &mut error,
            );
            assert_eq!(status, expected_status);
            assert_eq!((width, height), (0, 0));
            free_error(&mut error);
        }

        let mut undersized = initialized_render_options();
        undersized.struct_size = (size_of::<rofd_render_options_t>() - 1) as u32;
        let mut width = -1;
        let mut height = -1;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                &undersized,
                &mut width,
                &mut height,
                &mut error,
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!((width, height), (0, 0));
        free_error(&mut error);

        rofd_renderer_free(renderer);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[test]
fn null_render_options_use_defaults_and_rotation_clip_and_scale_are_converted() {
    // SAFETY: Handles remain live and outputs are distinct throughout the test.
    unsafe {
        let (document, page) = open_fixture_page();
        let renderer = new_renderer(ptr::null()).unwrap();
        let mut width = -1;
        let mut height = -1;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                ptr::null(),
                &mut width,
                &mut height,
                &mut error,
            ),
            ROFD_STATUS_OK
        );
        assert_eq!((width, height), (800, 530));

        let mut options = initialized_render_options();
        options.dpi = 254.0;
        options.scale = 0.5;
        options.rotation_degrees = 90;
        options.background_rgba = 0x1234_5678;
        options.has_clip = 1;
        options.clip_x_mm = 1.0;
        options.clip_y_mm = 2.0;
        options.clip_width_mm = 3.0;
        options.clip_height_mm = 4.0;
        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                &options,
                &mut width,
                &mut height,
                &mut error,
            ),
            ROFD_STATUS_OK
        );
        assert_eq!((width, height), (700, 1058));
        rofd_renderer_free(renderer);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[test]
fn null_required_pixel_outputs_are_rejected_after_clearing_other_outputs() {
    // SAFETY: Handles remain live and non-null outputs are distinct.
    unsafe {
        let (document, page) = open_fixture_page();
        let renderer = new_renderer(ptr::null()).unwrap();
        let mut width = -1;
        let mut height = -1;
        let mut error = ptr::null_mut();

        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                ptr::null(),
                ptr::null_mut(),
                &mut height,
                &mut error,
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(height, 0);
        free_error(&mut error);

        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                ptr::null(),
                &mut width,
                ptr::null_mut(),
                &mut error,
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(width, 0);
        free_error(&mut error);

        rofd_renderer_free(renderer);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[test]
fn invalid_handles_and_overlapping_outputs_fail_without_partial_results() {
    // SAFETY: Live handles stay valid; raw output calls intentionally exercise the documented
    // overlap rejection without creating aliased Rust references.
    unsafe {
        let (document, page) = open_fixture_page();
        let renderer = new_renderer(ptr::null()).unwrap();
        for (renderer_input, page_input) in [
            (ptr::null(), page as *const rofd_page_t),
            (renderer as *const rofd_renderer_t, ptr::null()),
        ] {
            let mut width = -1;
            let mut height = -1;
            let mut error = ptr::null_mut();
            assert_eq!(
                rofd_renderer_get_pixel_size(
                    renderer_input,
                    page_input,
                    ptr::null(),
                    &mut width,
                    &mut height,
                    &mut error,
                ),
                ROFD_STATUS_INVALID_ARGUMENT
            );
            assert_eq!((width, height), (0, 0));
            free_error(&mut error);
        }

        let mut shared = -1;
        let mut error = ptr::null_mut();
        let shared_pointer = ptr::addr_of_mut!(shared);
        assert_eq!(
            rofd_renderer_get_pixel_size(
                renderer,
                page,
                ptr::null(),
                shared_pointer,
                shared_pointer,
                &mut error,
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(shared, -1);
        free_error(&mut error);

        rofd_renderer_free(renderer);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[test]
fn renderer_output_is_nulled_on_invalid_options_and_free_accepts_null() {
    let mut options = initialized_renderer_options();
    options.max_font_bytes = 0;
    let mut renderer = ptr::NonNull::<rofd_renderer_t>::dangling().as_ptr();
    let mut error = ptr::null_mut();
    // SAFETY: Options and distinct outputs remain valid for the call.
    assert_eq!(
        unsafe { rofd_renderer_new(&options, &mut renderer, &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    assert!(renderer.is_null());
    // SAFETY: The failure error is owned here; null renderer free is defined as a no-op.
    unsafe {
        free_error(&mut error);
        rofd_renderer_free(ptr::null_mut());
    }
}
