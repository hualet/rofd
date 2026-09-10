use std::ffi::CString;
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::ptr;

use cairo::{Context, Format, ImageSurface};
use rofd_ffi::*;

struct Fixture {
    page: *mut rofd_page_t,
    renderer: *mut rofd_renderer_t,
}

impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
        let path = CString::new(path.to_str().unwrap()).unwrap();
        let mut document = ptr::null_mut();
        let mut page = ptr::null_mut();
        let mut renderer = ptr::null_mut();
        // SAFETY: Valid input path and disjoint output slots; each handle is freed once.
        unsafe {
            assert_eq!(
                rofd_document_open(path.as_ptr(), ptr::null(), &mut document, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            assert_eq!(
                rofd_document_get_page(document, 0, &mut page, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            rofd_document_free(document);
            assert_eq!(
                rofd_renderer_new(ptr::null(), &mut renderer, ptr::null_mut()),
                ROFD_STATUS_OK
            );
        }
        Self { page, renderer }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // SAFETY: These are uniquely owned live handles.
        unsafe {
            rofd_page_free(self.page);
            rofd_renderer_free(self.renderer);
        }
    }
}

fn viewport(x: i32, y: i32, width: i32, height: i32) -> rofd_pixel_rect_t {
    rofd_pixel_rect_t {
        struct_size: size_of::<rofd_pixel_rect_t>() as u32,
        x,
        y,
        width,
        height,
    }
}

#[test]
fn pixel_rectangle_layout_and_initializer_preserve_version_boundary() {
    assert_eq!(size_of::<rofd_pixel_rect_t>(), 20);
    assert_eq!(offset_of!(rofd_pixel_rect_t, height), 16);
    #[repr(C)]
    struct Extended {
        rect: rofd_pixel_rect_t,
        tail: [u8; 16],
    }
    let mut extended = Extended {
        rect: viewport(1, 2, 3, 4),
        tail: [0xa5; 16],
    };
    // SAFETY: Every non-null record has enough initialized, writable storage.
    unsafe {
        rofd_pixel_rect_init(ptr::null_mut(), usize::MAX);
        rofd_pixel_rect_init(&mut extended.rect, 19);
        assert_eq!(extended.rect.x, 1);
        rofd_pixel_rect_init(&mut extended.rect, size_of::<Extended>());
    }
    assert_eq!(extended.rect.struct_size, 20);
    assert_eq!(
        (
            extended.rect.x,
            extended.rect.y,
            extended.rect.width,
            extended.rect.height
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(extended.tail, [0xa5; 16]);
}

#[test]
fn canvas_query_is_independent_of_full_surface_and_budget() {
    let f = Fixture::new();
    let options = rofd_render_options_t {
        dpi: 5080.0,
        max_raster_bytes: 1,
        ..Default::default()
    };
    let (mut w, mut h) = (-1, -1);
    // SAFETY: Inputs and outputs remain live and disjoint.
    unsafe {
        assert_eq!(
            rofd_renderer_get_pixel_canvas_size(
                f.renderer,
                f.page,
                &options,
                &mut w,
                &mut h,
                ptr::null_mut()
            ),
            ROFD_STATUS_OK
        );
        assert_eq!((w, h), (42300, 28000));
        assert_ne!(
            rofd_renderer_get_pixel_size(
                f.renderer,
                f.page,
                &options,
                &mut w,
                &mut h,
                ptr::null_mut()
            ),
            ROFD_STATUS_OK
        );
        assert_eq!((w, h), (0, 0));
    }
}

#[test]
fn real_fixture_region_matches_full_page_and_preserves_cairo_ownership() {
    let f = Fixture::new();
    let options = rofd_render_options_t {
        dpi: 50.8,
        ..Default::default()
    };
    let mut full = ImageSurface::create(Format::ARgb32, 423, 280).unwrap();
    let rect = viewport(97, 42, 129, 71);
    let mut tile = ImageSurface::create(Format::ARgb32, rect.width, rect.height).unwrap();
    let mut report = ptr::null_mut();
    // SAFETY: Live borrowed Cairo contexts, initialized records and independent output.
    unsafe {
        let cr = Context::new(&full).unwrap();
        assert_eq!(
            rofd_renderer_render_page_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                &options,
                ptr::null_mut(),
                ptr::null_mut()
            ),
            ROFD_STATUS_OK
        );
        let cr = Context::new(&tile).unwrap();
        cr.move_to(2.0, 3.0);
        let refs = cairo::ffi::cairo_get_reference_count(cr.to_raw_none());
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                &options,
                &rect,
                &mut report,
                ptr::null_mut()
            ),
            ROFD_STATUS_OK
        );
        assert!(!report.is_null());
        assert_eq!(cr.current_point().unwrap(), (2.0, 3.0));
        assert_eq!(
            cairo::ffi::cairo_get_reference_count(cr.to_raw_none()),
            refs
        );
        rofd_render_report_free(report);
    }
    let fs = full.stride() as usize;
    let ts = tile.stride() as usize;
    let full = full.data().unwrap();
    let tile = tile.data().unwrap();
    let mut differences = Vec::new();
    for y in 0..rect.height as usize {
        let start = (y + rect.y as usize) * fs + rect.x as usize * 4;
        for x in 0..rect.width as usize * 4 {
            let delta = tile[y * ts + x].abs_diff(full[start + x]);
            if delta != 0 {
                differences.push((x / 4, y, delta));
            }
        }
    }
    // Cairo can change curve edge coverage with target extents even with an
    // identical matrix. This fixed invoice has 21 differing channels, delta <=2;
    // exact geometric/image stitching is covered in the renderer tests.
    assert!(
        differences.len() <= 32 && differences.iter().all(|v| v.2 <= 2),
        "{} differing channels, max delta {}, first {:?}",
        differences.len(),
        differences.iter().map(|v| v.2).max().unwrap_or(0),
        differences.first()
    );
}

#[test]
fn invalid_viewports_clear_reports_and_return_exact_errors() {
    let f = Fixture::new();
    let surface = ImageSurface::create(Format::ARgb32, 10, 10).unwrap();
    let cr = Context::new(&surface).unwrap();
    for rect in [
        viewport(-1, 0, 10, 10),
        viewport(0, 0, 0, 10),
        viewport(0, 0, 10, -1),
        viewport(i32::MAX, 0, 10, 10),
        viewport(0, 0, 9999, 10),
        rofd_pixel_rect_t {
            struct_size: 19,
            ..viewport(0, 0, 10, 10)
        },
    ] {
        let mut report = ptr::dangling_mut();
        let mut error = ptr::null_mut();
        // SAFETY: Invalid values are supported error cases; pointers remain live and disjoint.
        unsafe {
            assert_eq!(
                rofd_renderer_render_page_region_cairo(
                    f.renderer,
                    f.page,
                    cr.to_raw_none(),
                    ptr::null(),
                    &rect,
                    &mut report,
                    &mut error
                ),
                ROFD_STATUS_INVALID_ARGUMENT
            );
            assert!(report.is_null());
            assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
            rofd_error_free(error);
        }
    }
    let mut report = ptr::dangling_mut();
    // SAFETY: NULL required arguments are explicitly supported failures.
    unsafe {
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                ptr::null(),
                ptr::null(),
                &mut report,
                ptr::null_mut()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(report.is_null());
    }
}

#[test]
fn region_and_canvas_preflight_reject_aliases_before_writing() {
    let f = Fixture::new();
    #[repr(C, align(8))]
    struct Storage {
        rect: rofd_pixel_rect_t,
        pad: u32,
    }
    let mut storage = Storage {
        rect: viewport(8, 8, 16, 16),
        pad: 0,
    };
    let surface = ImageSurface::create(Format::ARgb32, 16, 16).unwrap();
    let cr = Context::new(&surface).unwrap();
    let rect = ptr::addr_of_mut!(storage.rect);
    // SAFETY: Raw aliases intentionally test rejection without forming aliased references.
    unsafe {
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                ptr::null(),
                rect,
                rect.cast(),
                ptr::null_mut()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!((*rect).struct_size, 20);
        assert_eq!((*rect).x, 8);
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                ptr::null(),
                rect,
                ptr::null_mut(),
                rect.cast()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!((*rect).x, 8);
        let mut options = rofd_render_options_t::default();
        let options_ptr = ptr::addr_of_mut!(options);
        let mut height = -1;
        assert_eq!(
            rofd_renderer_get_pixel_canvas_size(
                f.renderer,
                f.page,
                options_ptr,
                options_ptr.cast(),
                &mut height,
                ptr::null_mut()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            options.struct_size,
            size_of::<rofd_render_options_t>() as u32
        );
        assert_eq!(height, -1);
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                ptr::null(),
                rect,
                rect.cast(),
                &mut error
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!((*rect).x, 8);
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
        error = ptr::null_mut();
        let shared = ptr::addr_of_mut!(height);
        assert_eq!(
            rofd_renderer_get_pixel_canvas_size(
                f.renderer,
                f.page,
                ptr::null(),
                shared,
                shared,
                &mut error
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(height, -1);
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
    }
}

#[test]
fn region_accepts_extended_inputs_and_rejects_small_targets_and_null_context() {
    let f = Fixture::new();
    #[repr(C)]
    struct Extended {
        rect: rofd_pixel_rect_t,
        tail: [u8; 8],
    }
    let rect = Extended {
        rect: rofd_pixel_rect_t {
            struct_size: size_of::<Extended>() as u32,
            ..viewport(0, 0, 12, 12)
        },
        tail: [0xa5; 8],
    };
    let mut report = ptr::null_mut();
    // SAFETY: Records and outputs are live, aligned, disjoint; NULL is a supported failure.
    unsafe {
        let target = ImageSurface::create(Format::ARgb32, 12, 12).unwrap();
        let cr = Context::new(&target).unwrap();
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                ptr::null(),
                &rect.rect,
                ptr::null_mut(),
                ptr::null_mut()
            ),
            ROFD_STATUS_OK
        );
        assert_eq!(rect.tail, [0xa5; 8]);
        let small = ImageSurface::create(Format::ARgb32, 11, 12).unwrap();
        let cr = Context::new(&small).unwrap();
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                cr.to_raw_none(),
                ptr::null(),
                &rect.rect,
                &mut report,
                ptr::null_mut()
            ),
            ROFD_STATUS_RENDER_ERROR
        );
        assert!(report.is_null());
        assert_eq!(
            rofd_renderer_render_page_region_cairo(
                f.renderer,
                f.page,
                ptr::null_mut(),
                ptr::null(),
                &rect.rect,
                &mut report,
                ptr::null_mut()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(report.is_null());
    }
}
