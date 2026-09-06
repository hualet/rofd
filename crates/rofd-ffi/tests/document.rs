use std::ffi::{CStr, CString};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;

use rofd_ffi::{
    rofd_document_free, rofd_document_get_page, rofd_document_get_page_count, rofd_document_open,
    rofd_document_t, rofd_error_free, rofd_error_get_message, rofd_error_get_status, rofd_error_t,
    rofd_load_options_init, rofd_load_options_t, rofd_page_free, rofd_page_get_index,
    rofd_page_get_size_mm, rofd_page_t, rofd_rect_t, ROFD_STATUS_INVALID_ARGUMENT, ROFD_STATUS_IO,
    ROFD_STATUS_OK, ROFD_STATUS_PAGE_OUT_OF_RANGE, ROFD_STRICTNESS_LENIENT, ROFD_STRICTNESS_STRICT,
};

fn fixture_path() -> CString {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    CString::new(path.to_str().expect("fixture path must be UTF-8")).unwrap()
}

unsafe fn free_error(error: &mut *mut rofd_error_t) {
    if !error.is_null() {
        // SAFETY: The pointer was returned through an error output and is freed once here.
        unsafe { rofd_error_free(*error) };
        *error = ptr::null_mut();
    }
}

unsafe fn assert_error(error: *mut rofd_error_t, expected_status: u32) {
    assert!(!error.is_null());
    // SAFETY: `error` is live for both borrowed accessor calls.
    assert_eq!(unsafe { rofd_error_get_status(error) }, expected_status);
    // SAFETY: The message is borrowed while `error` remains live.
    let message = unsafe { rofd_error_get_message(error) };
    assert!(!message.is_null());
    // SAFETY: Error messages are valid NUL-terminated strings for the handle lifetime.
    assert!(!unsafe { CStr::from_ptr(message) }.to_bytes().is_empty());
}

unsafe fn open_fixture() -> (*mut rofd_document_t, *mut rofd_error_t) {
    let mut document = ptr::null_mut();
    let mut error = ptr::null_mut();
    let path = fixture_path();
    // SAFETY: All pointers are live and the output slots are distinct.
    let status =
        unsafe { rofd_document_open(path.as_ptr(), ptr::null(), &mut document, &mut error) };
    assert_eq!(status, ROFD_STATUS_OK);
    assert!(!document.is_null());
    assert!(error.is_null());
    (document, error)
}

#[test]
fn page_remains_valid_after_document_is_freed() {
    // SAFETY: This test observes all handle ownership rules and frees each returned handle once.
    unsafe {
        let (mut document, mut error) = open_fixture();
        let mut page_count = usize::MAX;
        assert_eq!(
            rofd_document_get_page_count(document, &mut page_count, &mut error),
            ROFD_STATUS_OK
        );
        assert_eq!(page_count, 1);

        let mut page = ptr::null_mut();
        assert_eq!(
            rofd_document_get_page(document, 0, &mut page, &mut error),
            ROFD_STATUS_OK
        );
        assert!(!page.is_null());

        let mut index = usize::MAX;
        assert_eq!(
            rofd_page_get_index(page, &mut index, &mut error),
            ROFD_STATUS_OK
        );
        assert_eq!(index, 0);

        let mut size = rofd_rect_t {
            x_mm: -1.0,
            y_mm: -1.0,
            width_mm: -1.0,
            height_mm: -1.0,
        };
        assert_eq!(
            rofd_page_get_size_mm(page, &mut size, &mut error),
            ROFD_STATUS_OK
        );
        assert_eq!((size.x_mm, size.y_mm), (0.0, 0.0));
        assert_eq!((size.width_mm, size.height_mm), (211.5, 140.0));

        rofd_document_free(document);
        document = ptr::null_mut();
        assert!(document.is_null());

        index = usize::MAX;
        assert_eq!(
            rofd_page_get_index(page, &mut index, &mut error),
            ROFD_STATUS_OK
        );
        assert_eq!(index, 0);

        rofd_page_free(page);
        rofd_document_free(ptr::null_mut());
        rofd_page_free(ptr::null_mut());
    }
}

#[test]
fn open_rejects_null_and_non_utf8_paths_and_nulls_handle_output() {
    for path in [ptr::null(), [0xff_u8, 0].as_ptr().cast()] {
        let mut document = ptr::NonNull::<rofd_document_t>::dangling().as_ptr();
        let mut error = ptr::null_mut();
        // SAFETY: The non-null byte sequence is NUL-terminated; outputs are valid and distinct.
        let status = unsafe { rofd_document_open(path, ptr::null(), &mut document, &mut error) };
        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(document.is_null());
        // SAFETY: Failure returned one owned error, freed once below.
        unsafe {
            assert_error(error, status);
            free_error(&mut error);
        }
    }
}

#[test]
fn open_rejects_empty_path_and_nulls_handle_output() {
    let path = CString::new("").unwrap();
    let mut document = ptr::NonNull::<rofd_document_t>::dangling().as_ptr();
    let mut error = ptr::null_mut();
    // SAFETY: The empty path is a readable NUL-terminated UTF-8 string and outputs are distinct.
    let status =
        unsafe { rofd_document_open(path.as_ptr(), ptr::null(), &mut document, &mut error) };
    assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
    assert!(document.is_null());
    // SAFETY: Failure returned one owned error, freed once below.
    unsafe {
        assert_error(error, status);
        free_error(&mut error);
    }
}

#[test]
fn missing_file_maps_to_io_and_nulls_handle_output() {
    let path = CString::new("/definitely/missing/rofd-task5.ofd").unwrap();
    let mut document = ptr::NonNull::<rofd_document_t>::dangling().as_ptr();
    let mut error = ptr::null_mut();
    // SAFETY: Inputs and distinct outputs remain valid for the call.
    let status =
        unsafe { rofd_document_open(path.as_ptr(), ptr::null(), &mut document, &mut error) };
    assert_eq!(status, ROFD_STATUS_IO);
    assert!(document.is_null());
    // SAFETY: Failure returned one owned error, freed once below.
    unsafe {
        assert_error(error, status);
        free_error(&mut error);
    }
}

#[test]
fn load_options_require_v1_size_and_known_strictness() {
    let path = fixture_path();
    for options in [
        rofd_load_options_t {
            struct_size: (size_of::<rofd_load_options_t>() - 1) as u32,
            strictness: ROFD_STRICTNESS_LENIENT,
        },
        rofd_load_options_t {
            struct_size: size_of::<rofd_load_options_t>() as u32,
            strictness: 2,
        },
    ] {
        let mut document = ptr::NonNull::<rofd_document_t>::dangling().as_ptr();
        let mut error = ptr::null_mut();
        // SAFETY: Options and distinct outputs remain valid for the call.
        let status =
            unsafe { rofd_document_open(path.as_ptr(), &options, &mut document, &mut error) };
        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(document.is_null());
        // SAFETY: Failure returned one owned error, freed once below.
        unsafe { free_error(&mut error) };
    }
}

#[repr(C)]
struct ExtendedLoadOptions {
    v1: rofd_load_options_t,
    future_tail: [u8; 16],
}

#[test]
fn load_options_accept_null_defaults_and_ignore_larger_tail() {
    let path = fixture_path();
    let mut options = ExtendedLoadOptions {
        v1: rofd_load_options_t {
            struct_size: 0,
            strictness: 99,
        },
        future_tail: [0xa5; 16],
    };
    // SAFETY: `v1` begins a writable record with the full extended capacity.
    unsafe { rofd_load_options_init(&mut options.v1, size_of::<ExtendedLoadOptions>()) };
    options.v1.struct_size = size_of::<ExtendedLoadOptions>() as u32;

    let mut document = ptr::null_mut();
    let mut error = ptr::null_mut();
    // SAFETY: Inputs and distinct outputs remain valid for the call.
    assert_eq!(
        unsafe { rofd_document_open(path.as_ptr(), &options.v1, &mut document, &mut error) },
        ROFD_STATUS_OK
    );
    assert!(!document.is_null());
    assert!(error.is_null());
    assert_eq!(options.future_tail, [0xa5; 16]);
    // SAFETY: The successful output handle is freed exactly once.
    unsafe { rofd_document_free(document) };
}

#[test]
fn load_options_accept_strict_mode() {
    let path = fixture_path();
    let options = rofd_load_options_t {
        struct_size: size_of::<rofd_load_options_t>() as u32,
        strictness: ROFD_STRICTNESS_STRICT,
    };
    let mut document = ptr::null_mut();
    let mut error = ptr::null_mut();
    // SAFETY: Inputs and distinct outputs remain valid for the call.
    assert_eq!(
        unsafe { rofd_document_open(path.as_ptr(), &options, &mut document, &mut error) },
        ROFD_STATUS_OK
    );
    assert!(!document.is_null());
    assert!(error.is_null());
    // SAFETY: The successful output handle is freed exactly once.
    unsafe { rofd_document_free(document) };
}

#[test]
fn null_required_outputs_fail_and_other_outputs_are_zeroed() {
    let path = fixture_path();
    let mut error = ptr::null_mut();
    // SAFETY: A null required output is defined; the separate error slot is writable.
    assert_eq!(
        unsafe { rofd_document_open(path.as_ptr(), ptr::null(), ptr::null_mut(), &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    // SAFETY: Failure returned one owned error, freed once below.
    unsafe { free_error(&mut error) };

    // SAFETY: Helper returns one live document handle.
    let (document, mut error) = unsafe { open_fixture() };
    // SAFETY: Null required outputs are defined and do not consume the borrowed document.
    assert_eq!(
        unsafe { rofd_document_get_page_count(document, ptr::null_mut(), &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    unsafe { free_error(&mut error) };
    assert_eq!(
        unsafe { rofd_document_get_page(document, 0, ptr::null_mut(), &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    unsafe { free_error(&mut error) };

    let mut page = ptr::null_mut();
    assert_eq!(
        unsafe { rofd_document_get_page(document, 0, &mut page, &mut error) },
        ROFD_STATUS_OK
    );
    assert_eq!(
        unsafe { rofd_page_get_index(page, ptr::null_mut(), &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    unsafe { free_error(&mut error) };
    assert_eq!(
        unsafe { rofd_page_get_size_mm(page, ptr::null_mut(), &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    unsafe { free_error(&mut error) };

    unsafe {
        rofd_page_free(page);
        rofd_document_free(document);
    }
}

#[test]
fn null_handles_and_out_of_range_pages_leave_outputs_at_entry_defaults() {
    let mut error = ptr::null_mut();
    let mut count = usize::MAX;
    // SAFETY: Output is writable; null input handle is defined as invalid.
    assert_eq!(
        unsafe { rofd_document_get_page_count(ptr::null(), &mut count, &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    assert_eq!(count, 0);
    unsafe { free_error(&mut error) };

    // SAFETY: Helper returns one live document handle.
    let (document, mut error) = unsafe { open_fixture() };
    let mut page = ptr::NonNull::<rofd_page_t>::dangling().as_ptr();
    // SAFETY: Document and outputs remain valid for the call.
    assert_eq!(
        unsafe { rofd_document_get_page(document, usize::MAX, &mut page, &mut error) },
        ROFD_STATUS_PAGE_OUT_OF_RANGE
    );
    assert!(page.is_null());
    unsafe { free_error(&mut error) };

    let mut index = usize::MAX;
    assert_eq!(
        unsafe { rofd_page_get_index(ptr::null(), &mut index, &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    assert_eq!(index, 0);
    unsafe { free_error(&mut error) };

    let mut size = rofd_rect_t {
        x_mm: 1.0,
        y_mm: 2.0,
        width_mm: 3.0,
        height_mm: 4.0,
    };
    assert_eq!(
        unsafe { rofd_page_get_size_mm(ptr::null(), &mut size, &mut error) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    assert_eq!(
        (size.x_mm, size.y_mm, size.width_mm, size.height_mm),
        (0.0, 0.0, 0.0, 0.0)
    );
    unsafe {
        free_error(&mut error);
        rofd_document_free(document);
    }
}
