use std::ffi::{CStr, CString};
use std::io::{Cursor, Write};
use std::mem::{align_of, size_of, MaybeUninit};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use rofd_ffi::*;
use zip::{write::SimpleFileOptions, ZipWriter};

fn open(page_xml: &str, strict: bool) -> (*mut rofd_document_t, *mut rofd_page_t) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, xml) in [
        (
            "OFD.xml",
            "<OFD><DocBody><DocInfo/><DocRoot>Document.xml</DocRoot></DocBody></OFD>",
        ),
        (
            "Document.xml",
            r#"<Document><CommonData><PageArea><PhysicalBox>5 7 100 100</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Page.xml"/></Pages><Bookmarks><Bookmark Name="target"><Dest Type="XYZ" PageID="42" Left="12.5" Top="24" Zoom="0"/></Bookmark></Bookmarks></Document>"#,
        ),
        ("Page.xml", page_xml),
    ] {
        zip.start_file(path, SimpleFileOptions::default()).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
    }
    let path = std::env::temp_dir().join(format!(
        "rofd-links-{}-{}.ofd",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
    let path_c = CString::new(path.to_str().unwrap()).unwrap();
    let mut document = ptr::null_mut();
    let mut page = ptr::null_mut();
    // SAFETY: All strings and initialized options/outputs are live and disjoint.
    unsafe {
        let mut options: rofd_load_options_t = std::mem::zeroed();
        rofd_load_options_init(&mut options, size_of::<rofd_load_options_t>());
        if strict {
            options.strictness = ROFD_STRICTNESS_STRICT;
        }
        assert_eq!(
            rofd_document_open(path_c.as_ptr(), &options, &mut document, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            rofd_document_get_page(document, 0, &mut page, ptr::null_mut()),
            ROFD_STATUS_OK
        );
    }
    (document, page)
}

const PAGE: &str = r#"<Page><Area><PhysicalBox>5 7 100 100</PhysicalBox></Area><Actions>
  <Action Event="PO"><URI URI="file:///not-opened"/></Action>
  <Action Event="CLICK"><Region>
    <Area Start="1 2"><Line Point1="4 2"/><Line Point1="4 6"/><Close/></Area>
    <Area Start="10 20"><Line Point1="15 20"/><Line Point1="15 28"/><Close/></Area>
  </Region><Goto><Bookmark Name="target"/></Goto></Action>
  <Action Event="CLICK"><GotoA AttachID="文件-a" NewWindow="false"/></Action>
  <Action Event="CUSTOM"><Movie ResourceID="12"/></Action>
</Actions><Content><Layer ID="1"><PathObject ID="2" Boundary="10 20 20 10" CTM="20 0 0 10 0 0" Stroke="false" Fill="true"><AbbreviatedData>M 0 0 L 1 0 L 1 1 C</AbbreviatedData><Actions><Action Event="CLICK"><URI URI="relative" Base="https://example.invalid/"/></Action></Actions></PathObject></Layer></Content></Page>"#;

fn snapshot() -> *mut rofd_link_list_t {
    let (document, page) = open(PAGE, false);
    let mut links = ptr::null_mut();
    // SAFETY: The source handles are live; snapshot output is independent storage.
    unsafe {
        assert_eq!(
            rofd_page_get_links(page, &mut links, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        rofd_page_free(page);
        rofd_document_free(document);
    }
    links
}

fn action() -> rofd_action_t {
    // SAFETY: All fields are zero-valid numbers and raw pointers.
    let mut value: rofd_action_t = unsafe { std::mem::zeroed() };
    value.struct_size = size_of::<rofd_action_t>() as u32;
    value
}

fn destination() -> rofd_destination_t {
    // SAFETY: All fields are zero-valid numbers and raw pointers.
    let mut value: rofd_destination_t = unsafe { std::mem::zeroed() };
    value.struct_size = size_of::<rofd_destination_t>() as u32;
    value
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> rofd_rect_t {
    rofd_rect_t {
        x_mm: x,
        y_mm: y,
        width_mm: w,
        height_mm: h,
    }
}

#[test]
fn owned_link_snapshot_preserves_regions_actions_and_destinations_after_source_free() {
    let links = snapshot();
    // SAFETY: Owned immutable snapshot and independent writable output storage.
    unsafe {
        let mut count = 0;
        assert_eq!(
            rofd_link_list_get_count(links, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 5);
        let mut area = rect(0.0, 0.0, 0.0, 0.0);
        let mut act = action();
        for index in 0..count {
            let mut actions = 0;
            assert_eq!(
                rofd_link_list_get_action_count(links, index, &mut actions, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            assert_eq!(actions, 1);
        }
        assert_eq!(
            rofd_link_list_get_region(links, 0, 0, &mut area, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(area, rect(5.0, 7.0, 100.0, 100.0));
        assert_eq!(
            rofd_link_list_get_action(links, 0, 0, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(act.event, ROFD_ACTION_EVENT_PAGE_OPEN);
        assert_eq!(
            CStr::from_ptr(act.uri).to_str().unwrap(),
            "file:///not-opened"
        );
        assert_eq!(
            rofd_link_list_get_region_count(links, 1, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 2);
        for (index, expected) in [rect(1.0, 2.0, 3.0, 4.0), rect(10.0, 20.0, 5.0, 8.0)]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                rofd_link_list_get_region(links, 1, index, &mut area, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            assert_eq!(area, expected);
        }
        let mut dest = destination();
        assert_eq!(
            rofd_link_list_get_action_destination(links, 1, 0, &mut dest, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(dest.page_id, 42);
        assert_eq!(dest.page_index, 0);
        assert_eq!(
            dest.flags,
            ROFD_DESTINATION_HAS_PAGE_ID
                | ROFD_DESTINATION_HAS_PAGE_INDEX
                | ROFD_DESTINATION_HAS_LEFT
                | ROFD_DESTINATION_HAS_TOP
                | ROFD_DESTINATION_HAS_ZOOM
        );
        assert_eq!((dest.left_mm, dest.top_mm, dest.zoom), (12.5, 24.0, 0.0));
        assert_eq!(
            rofd_link_list_get_action(links, 2, 0, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!((act.kind, act.flags), (ROFD_ACTION_ATTACHMENT, 0));
        assert_eq!(
            CStr::from_ptr(act.attachment_id).to_str().unwrap(),
            "文件-a"
        );
        assert_eq!(
            rofd_link_list_get_action(links, 3, 0, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            (act.kind, act.event),
            (ROFD_ACTION_UNKNOWN, ROFD_ACTION_EVENT_UNKNOWN)
        );
        assert_eq!(CStr::from_ptr(act.type_name).to_str().unwrap(), "Movie");
        assert_eq!(CStr::from_ptr(act.event_name).to_str().unwrap(), "CUSTOM");
        assert_eq!(
            rofd_link_list_get_region(links, 4, 0, &mut area, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(area, rect(10.0, 20.0, 20.0, 10.0));
        assert_eq!(
            rofd_link_list_get_action(links, 4, 0, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            CStr::from_ptr(act.uri_base).to_str().unwrap(),
            "https://example.invalid/"
        );
        rofd_link_list_free(links);
    }
}

#[test]
fn empty_pages_have_owned_empty_snapshots_and_invalid_regions_never_fall_back() {
    for (xml, strict, expected) in [
        (
            "<Page><Area><PhysicalBox>0 0 100 100</PhysicalBox></Area></Page>",
            true,
            ROFD_STATUS_OK,
        ),
        (
            r#"<Page><Area><PhysicalBox>0 0 100 100</PhysicalBox></Area><Actions><Action Event="CLICK"><Region><Area Start="NaN 0"/></Region><URI URI="file:///not-opened"/></Action></Actions></Page>"#,
            false,
            ROFD_STATUS_OK,
        ),
        (
            r#"<Page><Area><PhysicalBox>0 0 100 100</PhysicalBox></Area><Actions><Action Event="CLICK"><Region><Area Start="NaN 0"/></Region><URI URI="file:///not-opened"/></Action></Actions></Page>"#,
            true,
            ROFD_STATUS_INVALID_DOCUMENT,
        ),
    ] {
        let (document, page) = open(xml, strict);
        // SAFETY: Handles and independent output storage remain live for each call.
        unsafe {
            let mut links = ptr::dangling_mut();
            let mut error = ptr::null_mut();
            assert_eq!(rofd_page_get_links(page, &mut links, &mut error), expected);
            if expected == ROFD_STATUS_OK {
                assert!(!links.is_null());
                let mut count = 9;
                assert_eq!(
                    rofd_link_list_get_count(links, &mut count, ptr::null_mut()),
                    ROFD_STATUS_OK
                );
                assert_eq!(count, 0);
                rofd_link_list_free(links);
            } else {
                assert!(links.is_null());
                assert!(!error.is_null());
            }
            let mut warnings = ptr::null_mut();
            assert_eq!(
                rofd_document_get_warnings(document, &mut warnings, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            let mut warning_count = 0;
            assert_eq!(
                rofd_warning_list_get_count(warnings, &mut warning_count, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            assert_eq!(warning_count, usize::from(!strict));
            rofd_warning_list_free(warnings);
            rofd_error_free(error);
            rofd_page_free(page);
            rofd_document_free(document);
        }
    }
}

#[test]
fn link_query_failures_zero_outputs_and_preserve_aliases() {
    let links = snapshot();
    // SAFETY: Valid storage is live; deliberately malformed layouts and overlapping
    // slots are rejected before dereference or any conflicting writes.
    unsafe {
        assert_eq!(
            rofd_page_get_links(ptr::null(), ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_link_list_get_count(ptr::null(), ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_link_list_get_region_count(links, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_link_list_get_action_count(links, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_link_list_get_region(links, 0, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_link_list_get_action(links, 0, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_link_list_get_action_destination(links, 0, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        let mut count = 99;
        assert_eq!(
            rofd_link_list_get_region_count(links, usize::MAX, &mut count, ptr::null_mut()),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert_eq!(count, 0);
        count = 99;
        assert_eq!(
            rofd_link_list_get_action_count(links, usize::MAX, &mut count, ptr::null_mut()),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert_eq!(count, 0);
        let mut area = rect(1.0, 2.0, 3.0, 4.0);
        assert_eq!(
            rofd_link_list_get_region(links, 0, usize::MAX, &mut area, ptr::null_mut()),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert_eq!(area, rect(0.0, 0.0, 0.0, 0.0));
        let mut act = action();
        assert_eq!(
            rofd_link_list_get_action(links, 0, usize::MAX, &mut act, ptr::null_mut()),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert!(act.type_name.is_null());
        let mut dest = destination();
        assert_eq!(
            rofd_link_list_get_action_destination(links, 0, 0, &mut dest, ptr::null_mut()),
            ROFD_STATUS_UNSUPPORTED
        );
        assert_eq!(dest.flags, 0);
        count = 0x12345678;
        assert_eq!(
            rofd_link_list_get_count(links, &mut count, (&mut count as *mut usize).cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0x12345678);
        area = rect(1.0, 2.0, 3.0, 4.0);
        assert_eq!(
            rofd_link_list_get_region(
                links,
                0,
                0,
                &mut area,
                (&mut area as *mut rofd_rect_t).cast()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(area, rect(1.0, 2.0, 3.0, 4.0));
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_link_list_get_region(links, 0, 0, links.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(!error.is_null());
        rofd_error_free(error);
        error = ptr::null_mut();
        let overflow = (usize::MAX & !(align_of::<rofd_rect_t>() - 1)) as *mut rofd_rect_t;
        assert_eq!(
            rofd_link_list_get_region(links, 0, 0, overflow, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(error.is_null());
        assert_eq!(
            rofd_link_list_get_count(ptr::dangling::<u8>().cast(), &mut count, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0x12345678);
        assert_eq!(
            rofd_link_list_get_count(links, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 5);
        rofd_link_list_free(links);
        rofd_link_list_free(ptr::null_mut());
    }
}

#[test]
fn link_action_and_destination_records_keep_tails_and_failure_prefix_contracts() {
    let links = snapshot();
    macro_rules! check {
        ($record:ty, $query:expr) => {{
            #[repr(C)]
            struct Extended {
                record: $record,
                tail: [u8; 17],
            }
            let mut storage = MaybeUninit::<Extended>::uninit();
            // SAFETY: Backing bytes are all initialized and never moved; rejected
            // overlap/layout calls must leave the record unchanged.
            unsafe {
                storage
                    .as_mut_ptr()
                    .cast::<u8>()
                    .write_bytes(0xa5, size_of::<Extended>());
                let record = ptr::addr_of_mut!((*storage.as_mut_ptr()).record);
                (*record).struct_size = size_of::<Extended>() as u32;
                assert_eq!($query(1, record, ptr::null_mut()), ROFD_STATUS_OK);
                assert_eq!((*record).struct_size, size_of::<Extended>() as u32);
                assert_eq!((*storage.as_ptr()).tail, [0xa5; 17]);
                assert_eq!(
                    $query(usize::MAX, record, ptr::null_mut()),
                    ROFD_STATUS_PAGE_OUT_OF_RANGE
                );
                let bytes = std::slice::from_raw_parts(record.cast::<u8>(), size_of::<$record>());
                assert!(bytes[4..].iter().all(|&byte| byte == 0));
                let before = bytes.to_vec();
                assert_eq!(
                    $query(1, record, record.cast()),
                    ROFD_STATUS_INVALID_ARGUMENT
                );
                assert_eq!(
                    std::slice::from_raw_parts(record.cast::<u8>(), size_of::<$record>()),
                    before
                );
                let mut error = ptr::null_mut();
                assert_eq!(
                    $query(1, links.cast(), &mut error),
                    ROFD_STATUS_INVALID_ARGUMENT
                );
                assert!(!error.is_null());
                rofd_error_free(error);
                error = ptr::null_mut();
                let overflow = (usize::MAX & !(align_of::<$record>() - 1)) as *mut $record;
                assert_eq!(
                    $query(1, overflow, &mut error),
                    ROFD_STATUS_INVALID_ARGUMENT
                );
                assert!(error.is_null());
                (*record).struct_size = 4;
                assert_eq!(
                    $query(1, record, ptr::null_mut()),
                    ROFD_STATUS_INVALID_ARGUMENT
                );
                assert_eq!((*record).struct_size, 4);
                assert_eq!((*storage.as_ptr()).tail, [0xa5; 17]);
            }
        }};
    }
    check!(rofd_action_t, |link, record, error| {
        rofd_link_list_get_action(links, link, 0, record, error)
    });
    check!(rofd_destination_t, |link, record, error| {
        rofd_link_list_get_action_destination(links, link, 0, record, error)
    });
    // SAFETY: This test uniquely owns the snapshot and no borrowed data is used later.
    unsafe { rofd_link_list_free(links) };
}

#[test]
fn link_snapshot_output_preflight_does_not_mutate_source_page_or_aliased_slots() {
    let (document, page) = open(PAGE, false);
    // SAFETY: The live page and output storage remain valid; deliberate aliases
    // and malformed layouts are rejected before writes or page dereferences.
    unsafe {
        let mut output: *mut rofd_link_list_t = ptr::dangling_mut();
        let before = output;
        let slot = &mut output as *mut *mut rofd_link_list_t;
        assert_eq!(
            rofd_page_get_links(page, slot, slot.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(output, before);
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_page_get_links(page, page.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(!error.is_null());
        rofd_error_free(error);
        assert_eq!(
            rofd_page_get_links(ptr::dangling::<u8>().cast(), &mut output, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(output, before);
        assert_eq!(
            rofd_page_get_links(page, &mut output, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        let mut count = 0;
        assert_eq!(
            rofd_link_list_get_count(output, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 5);
        rofd_link_list_free(output);
        rofd_page_free(page);
        rofd_document_free(document);
    }
}
