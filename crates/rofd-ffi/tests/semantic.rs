#[path = "../../rofd-core/tests/support/mod.rs"]
mod support;

use rofd_ffi::*;
use std::ffi::{CStr, CString};
use std::mem::{size_of, MaybeUninit};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

fn page(two_objects: bool) -> *mut rofd_page_t {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let tail = if two_objects {
        r#"<ofd:TextObject ID="3" Boundary="10 35 30 8" Font="10" Size="4"><ofd:TextCode X="1" Y="5">尾</ofd:TextCode></ofd:TextObject>"#
    } else {
        ""
    };
    let xml = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="1"><ofd:TextObject ID="2" Boundary="10 20 30 8" Font="10" Size="4"><ofd:TextCode X="1" Y="5" DeltaX="3 4">A中B</ofd:TextCode></ofd:TextObject>{tail}</ofd:Layer></ofd:Content></ofd:Page>"#
    );
    let document = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea><ofd:PublicRes>Res.xml</ofd:PublicRes></ofd:CommonData><ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
    let font = br#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Fixture"/></ofd:Fonts></ofd:Res>"#;
    let bytes =
        support::ofd_with_document_page_and_entries(document, &xml, &[("Doc_0/Res.xml", font)]);
    let path = std::env::temp_dir().join(format!(
        "rofd-ffi-semantic-{}-{}.ofd",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes).unwrap();
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    // SAFETY: All input and output storage is valid and distinct; document is freed once.
    unsafe {
        let mut document = ptr::null_mut();
        let mut page = ptr::null_mut();
        assert_eq!(
            rofd_document_open(cpath.as_ptr(), ptr::null(), &mut document, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            rofd_document_get_page(document, 0, &mut page, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        rofd_document_free(document);
        page
    }
}

unsafe fn character(layout: *const rofd_text_layout_t, index: usize) -> rofd_text_char_t {
    let mut value = MaybeUninit::<rofd_text_char_t>::zeroed();
    // SAFETY: A complete initialized output is live and distinct from the borrowed layout.
    unsafe {
        (*value.as_mut_ptr()).struct_size = size_of::<rofd_text_char_t>() as u32;
        assert_eq!(
            rofd_text_layout_get_char(layout, index, value.as_mut_ptr(), ptr::null_mut()),
            ROFD_STATUS_OK
        );
        value.assume_init()
    }
}

unsafe fn bytes(string: *const rofd_string_t) -> Vec<u8> {
    // SAFETY: The returned data is borrowed only while its owning string is live.
    unsafe {
        CStr::from_ptr(rofd_string_get_data(string))
            .to_bytes()
            .to_vec()
    }
}

#[test]
fn owned_text_and_layout_keep_exact_utf8_after_page_and_document_free() {
    // SAFETY: Each handle is live for its accesses and freed once after all borrows.
    unsafe {
        let page = page(false);
        let mut text = ptr::null_mut();
        let mut layout = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text(page, &mut text, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            rofd_page_get_text_layout(page, &mut layout, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        rofd_page_free(page);
        assert_eq!(bytes(text), "A中B".as_bytes());
        assert_eq!(rofd_string_get_length(text), 5);
        let mut count = 0;
        assert_eq!(
            rofd_text_layout_get_count(layout, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 3);
        let c = character(layout, 1);
        assert_eq!((c.utf8_offset, c.utf8_length, c.object_id), (1, 3, 2));
        assert_eq!(c.flags, ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY);
        assert!(c.rect_mm.x_mm.is_finite() && c.rect_mm.y_mm.is_finite());
        assert!(c.rect_mm.width_mm > 0.0 && c.rect_mm.width_mm.is_finite());
        assert!(c.rect_mm.height_mm > 0.0 && c.rect_mm.height_mm.is_finite());
        rofd_string_free(text);
        rofd_text_layout_free(layout);
    }
}

#[test]
fn area_uses_glyph_intersections_and_separator_has_no_geometry() {
    // SAFETY: All live handles and records are disjoint and freed exactly once.
    unsafe {
        let page = page(true);
        let mut layout = ptr::null_mut();
        let mut text = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text_layout(page, &mut layout, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        let separator = character(layout, 3);
        assert_eq!(
            (
                separator.utf8_offset,
                separator.utf8_length,
                separator.object_id
            ),
            (5, 1, 0)
        );
        assert_eq!(
            separator.flags,
            ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR | ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY
        );
        assert_eq!(
            (
                separator.rect_mm.x_mm,
                separator.rect_mm.y_mm,
                separator.rect_mm.width_mm,
                separator.rect_mm.height_mm
            ),
            (0.0, 0.0, 0.0, 0.0)
        );
        let c = character(layout, 1);
        let area = rofd_rect_t {
            x_mm: c.rect_mm.x_mm + c.rect_mm.width_mm - 0.02,
            y_mm: c.rect_mm.y_mm + 0.01,
            width_mm: 0.01,
            height_mm: 0.01,
        };
        assert_eq!(
            rofd_page_get_text_for_area(page, &area, &mut text, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(bytes(text), "中".as_bytes());
        rofd_string_free(text);
        assert_eq!(
            rofd_page_get_text(page, &mut text, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(bytes(text), "A中B\n尾".as_bytes());
        rofd_string_free(text);
        rofd_text_layout_free(layout);
        rofd_page_free(page);
    }
}

#[test]
fn null_and_invalid_arguments_publish_transactional_defaults() {
    // SAFETY: Invalid handles are null; sentinel outputs are never dereferenced.
    unsafe {
        assert!(rofd_string_get_data(ptr::null()).is_null());
        assert_eq!(rofd_string_get_length(ptr::null()), 0);
        rofd_string_free(ptr::null_mut());
        rofd_text_layout_free(ptr::null_mut());
        let mut text = ptr::dangling_mut();
        let mut layout = ptr::dangling_mut();
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text(ptr::null(), &mut text, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(text.is_null());
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
        assert_eq!(
            rofd_page_get_text_layout(ptr::null(), &mut layout, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(layout.is_null());
        let mut count = 99;
        assert_eq!(
            rofd_text_layout_get_count(ptr::null(), &mut count, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0);
        let page = page(false);
        assert_eq!(
            rofd_page_get_text(page, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_page_get_text_layout(page, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_page_get_text_for_area(page, ptr::null(), &mut text, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(text.is_null());
        for area in [
            rofd_rect_t {
                x_mm: f64::NAN,
                y_mm: 0.0,
                width_mm: 1.0,
                height_mm: 1.0,
            },
            rofd_rect_t {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: -1.0,
                height_mm: 1.0,
            },
        ] {
            assert_eq!(
                rofd_page_get_text_for_area(page, &area, &mut text, ptr::null_mut()),
                ROFD_STATUS_INVALID_ARGUMENT
            );
            assert!(text.is_null());
        }
        assert_eq!(
            rofd_page_get_text_layout(page, &mut layout, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            rofd_text_layout_get_count(layout, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_text_layout_get_char(layout, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        let mut c = character(layout, 1);
        assert_eq!(
            rofd_text_layout_get_char(layout, 99, &mut c, ptr::null_mut()),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert_eq!(
            (c.utf8_offset, c.utf8_length, c.object_id, c.flags),
            (0, 0, 0, 0)
        );
        assert_eq!(c.struct_size as usize, size_of::<rofd_text_char_t>());
        rofd_text_layout_free(layout);
        rofd_page_free(page);
    }
}

#[test]
fn record_versioning_preserves_small_record_and_unknown_tail() {
    #[repr(C)]
    struct Extended {
        record: rofd_text_char_t,
        tail: [u8; 16],
    }
    // SAFETY: Byte-initialized records are readable and writable at their declared boundary.
    unsafe {
        let page = page(false);
        let mut layout = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text_layout(page, &mut layout, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        let mut extended = MaybeUninit::<Extended>::uninit();
        ptr::write_bytes(
            extended.as_mut_ptr().cast::<u8>(),
            0xa5,
            size_of::<Extended>(),
        );
        (*extended.as_mut_ptr()).record.struct_size = size_of::<Extended>() as u32;
        let mut extended = extended.assume_init();
        assert_eq!(
            rofd_text_layout_get_char(layout, 1, &mut extended.record, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(extended.tail, [0xa5; 16]);
        assert_eq!(extended.record.struct_size as usize, size_of::<Extended>());
        let mut small = 4_u32;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_text_layout_get_char(layout, 0, (&mut small as *mut u32).cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(small, 4);
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
        rofd_text_layout_free(layout);
        rofd_page_free(page);
    }
}

#[test]
fn preflight_rejects_output_aliases_and_input_overlap_without_corruption() {
    // SAFETY: Aliased slots are valid allocations, deliberately rejected before any conflicting write.
    unsafe {
        let page = page(false);
        let mut layout = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text_layout(page, &mut layout, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        let mut slot: *mut rofd_string_t = ptr::dangling_mut();
        let sentinel = slot;
        let raw = &mut slot as *mut *mut rofd_string_t;
        assert_eq!(
            rofd_page_get_text(page, raw, raw.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(slot, sentinel);
        let mut area = rofd_rect_t {
            x_mm: 1.0,
            y_mm: 2.0,
            width_mm: 3.0,
            height_mm: 4.0,
        };
        let area_ptr = &mut area as *mut rofd_rect_t;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text_for_area(page, area_ptr, area_ptr.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(area.x_mm, 1.0);
        rofd_error_free(error);
        assert_eq!(
            rofd_page_get_text(page, page.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        rofd_error_free(error);
        assert_eq!(
            rofd_page_get_text_layout(page, page.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        rofd_error_free(error);
        assert_eq!(
            rofd_text_layout_get_count(layout, layout.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        rofd_error_free(error);
        let mut c = character(layout, 1);
        let cptr = &mut c as *mut rofd_text_char_t;
        assert_eq!(
            rofd_text_layout_get_char(layout, 1, cptr, cptr.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(c.utf8_length, 3);
        assert_eq!(character(layout, 1).utf8_length, 3);
        rofd_text_layout_free(layout);
        rofd_page_free(page);
    }
}

#[test]
fn concurrent_readers_share_live_page_and_snapshot() {
    // SAFETY: Read-only calls share live handles, each uses its own outputs, and joins precede free.
    unsafe {
        let page = page(false);
        let mut layout = ptr::null_mut();
        assert_eq!(
            rofd_page_get_text_layout(page, &mut layout, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        let page_address = page as usize;
        let layout_address = layout as usize;
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(move || {
                    let mut text = ptr::null_mut();
                    assert_eq!(
                        rofd_page_get_text(
                            page_address as *const rofd_page_t,
                            &mut text,
                            ptr::null_mut()
                        ),
                        ROFD_STATUS_OK
                    );
                    assert_eq!(bytes(text), "A中B".as_bytes());
                    assert_eq!(
                        character(layout_address as *const rofd_text_layout_t, 1).utf8_length,
                        3
                    );
                    rofd_string_free(text);
                });
            }
        });
        rofd_text_layout_free(layout);
        rofd_page_free(page);
    }
}
