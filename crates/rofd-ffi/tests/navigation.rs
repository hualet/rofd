use std::ffi::{CStr, CString};
use std::io::{Cursor, Write};
use std::mem::{align_of, offset_of, size_of, MaybeUninit};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use rofd_ffi::*;
use zip::{write::SimpleFileOptions, ZipWriter};

struct Document(*mut rofd_document_t);

impl Document {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, xml) in [
            (
                "OFD.xml",
                "<OFD><DocBody><DocInfo/><DocRoot>Document.xml</DocRoot></DocBody></OFD>",
            ),
            (
                "Document.xml",
                r#"<Document><CommonData><PageArea><PhysicalBox>0 0 100 100</PhysicalBox></PageArea></CommonData>
                <Pages><Page ID="5" BaseLoc="Page.xml"/><Page ID="42" BaseLoc="Page.xml"/><Page ID="88" BaseLoc="Page.xml"/></Pages>
                <Outlines>
                  <OutlineElem Title="Root &amp; notes" Count="999" Expanded="false">
                    <Actions><Action Event="CLICK"><Goto><Bookmark Name="target"/></Goto></Action></Actions>
                    <OutlineElem Title="孩子"><Actions><Action Event="CLICK"><Goto><Dest Type="XYZ" PageID="88" Left="1.25" Top="2.5" Zoom="0"/></Goto></Action></Actions></OutlineElem>
                  </OutlineElem>
                  <OutlineElem Title="Actions"><Actions>
                    <Action Event="CLICK"><URI URI="child?q=1&amp;x=2" Base="https://example.invalid/"/></Action>
                    <Action Event="CLICK"><GotoA AttachID="attachment-a"/></Action>
                    <Action Event="CUSTOM"><Movie ResourceID="900"/></Action>
                  </Actions></OutlineElem>
                  <OutlineElem Title="Unresolved"><Actions>
                    <Action Event="CLICK"><Goto><Dest Type="Fit" PageID="999"/></Goto></Action>
                    <Action Event="CLICK"><Goto><Bookmark Name="missing"/></Goto></Action>
                  </Actions></OutlineElem>
                </Outlines>
                <Bookmarks><Bookmark Name="target"><Dest Type="FitH" PageID="42"><Top>9</Top></Dest></Bookmark></Bookmarks>
              </Document>"#,
            ),
            (
                "Page.xml",
                "<Page><Area><PhysicalBox>0 0 100 100</PhysicalBox></Area></Page>",
            ),
        ] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(xml.as_bytes()).unwrap();
        }
        let path = std::env::temp_dir().join(format!(
            "rofd-navigation-{}-{}.ofd",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
        let path_c = CString::new(path.to_str().unwrap()).unwrap();
        let mut document = ptr::null_mut();
        // SAFETY: Input path and output storage are live and disjoint.
        let status = unsafe {
            rofd_document_open(path_c.as_ptr(), ptr::null(), &mut document, ptr::null_mut())
        };
        std::fs::remove_file(path).unwrap();
        assert_eq!(status, ROFD_STATUS_OK);
        Self(document)
    }

    fn outline(&self) -> *mut rofd_outline_t {
        let mut outline = ptr::null_mut();
        // SAFETY: Live handle and independent writable output.
        unsafe {
            assert_eq!(
                rofd_document_get_outline(self.0, &mut outline, ptr::null_mut()),
                ROFD_STATUS_OK
            );
        }
        outline
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        // SAFETY: This wrapper uniquely owns the live handle.
        unsafe { rofd_document_free(self.0) };
    }
}

fn node() -> rofd_outline_node_t {
    // SAFETY: The C record consists solely of zero-valid numbers and pointers.
    let mut value: rofd_outline_node_t = unsafe { std::mem::zeroed() };
    value.struct_size = size_of::<rofd_outline_node_t>() as u32;
    value
}

fn action() -> rofd_action_t {
    // SAFETY: The C record consists solely of zero-valid numbers and pointers.
    let mut value: rofd_action_t = unsafe { std::mem::zeroed() };
    value.struct_size = size_of::<rofd_action_t>() as u32;
    value
}

fn destination() -> rofd_destination_t {
    // SAFETY: The C record consists solely of zero-valid numbers and pointers.
    let mut value: rofd_destination_t = unsafe { std::mem::zeroed() };
    value.struct_size = size_of::<rofd_destination_t>() as u32;
    value
}

#[test]
fn outline_owns_tree_actions_and_destinations_after_document_free() {
    let document = Document::new();
    let outline = document.outline();
    drop(document);
    // SAFETY: Owned snapshot and disjoint initialized output records remain live.
    unsafe {
        let mut count = 0;
        assert_eq!(
            rofd_outline_get_count(outline, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 4);
        let mut value = node();
        assert_eq!(
            rofd_outline_get_node(outline, 0, &mut value, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            CStr::from_ptr(value.title).to_str().unwrap(),
            "Root & notes"
        );
        assert_eq!(
            (
                value.parent,
                value.first_child,
                value.next_sibling,
                value.expanded,
                value.action_count
            ),
            (ROFD_NO_INDEX, 1, 2, 0, 1)
        );
        assert_eq!(
            rofd_outline_get_node(outline, 1, &mut value, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(CStr::from_ptr(value.title).to_str().unwrap(), "孩子");
        assert_eq!(
            (
                value.parent,
                value.first_child,
                value.next_sibling,
                value.expanded
            ),
            (0, ROFD_NO_INDEX, ROFD_NO_INDEX, 1)
        );
        let mut act = action();
        assert_eq!(
            rofd_outline_get_action(outline, 0, 0, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            (act.kind, act.event),
            (ROFD_ACTION_GOTO, ROFD_ACTION_EVENT_CLICK)
        );
        assert_eq!(CStr::from_ptr(act.bookmark).to_str().unwrap(), "target");
        let mut dest = destination();
        assert_eq!(
            rofd_outline_get_action_destination(outline, 0, 0, &mut dest, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            (dest.kind, dest.page_index, dest.page_id, dest.top_mm),
            (ROFD_DESTINATION_FIT_H, 1, 42, 9.0)
        );
        assert_ne!(dest.flags & ROFD_DESTINATION_HAS_PAGE_INDEX, 0);
        assert_eq!(
            rofd_outline_get_action_destination(outline, 1, 0, &mut dest, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            (
                dest.kind,
                dest.page_index,
                dest.left_mm,
                dest.top_mm,
                dest.zoom
            ),
            (ROFD_DESTINATION_XYZ, 2, 1.25, 2.5, 0.0)
        );
        assert_ne!(dest.flags & ROFD_DESTINATION_HAS_ZOOM, 0);
        assert_eq!(dest.flags & ROFD_DESTINATION_HAS_RIGHT, 0);
        rofd_outline_free(outline);
    }
}

#[test]
fn action_payloads_unknown_types_and_unresolved_targets_remain_inspectable() {
    let document = Document::new();
    let outline = document.outline();
    // SAFETY: Live snapshot and independent output records and error slot.
    unsafe {
        let mut act = action();
        assert_eq!(
            rofd_outline_get_action(outline, 2, 0, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(act.kind, ROFD_ACTION_URI);
        assert_eq!(CStr::from_ptr(act.uri).to_str().unwrap(), "child?q=1&x=2");
        assert_eq!(
            CStr::from_ptr(act.uri_base).to_str().unwrap(),
            "https://example.invalid/"
        );
        let mut dest = destination();
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_outline_get_action_destination(outline, 2, 0, &mut dest, &mut error),
            ROFD_STATUS_UNSUPPORTED
        );
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_UNSUPPORTED);
        assert_eq!(dest.flags, 0);
        rofd_error_free(error);
        assert_eq!(
            rofd_outline_get_action(outline, 2, 1, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(act.kind, ROFD_ACTION_ATTACHMENT);
        assert_ne!(act.flags & ROFD_ACTION_NEW_WINDOW, 0);
        assert_eq!(
            CStr::from_ptr(act.attachment_id).to_str().unwrap(),
            "attachment-a"
        );
        assert_eq!(
            rofd_outline_get_action(outline, 2, 2, &mut act, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(
            (act.kind, act.event),
            (ROFD_ACTION_UNKNOWN, ROFD_ACTION_EVENT_UNKNOWN)
        );
        assert_eq!(CStr::from_ptr(act.type_name).to_str().unwrap(), "Movie");
        assert_eq!(CStr::from_ptr(act.event_name).to_str().unwrap(), "CUSTOM");
        assert_eq!(
            rofd_outline_get_action_destination(outline, 3, 0, &mut dest, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(dest.page_id, 999);
        assert_eq!(dest.page_index, ROFD_NO_INDEX);
        assert_eq!(dest.flags & ROFD_DESTINATION_HAS_PAGE_INDEX, 0);
        assert_eq!(
            rofd_outline_get_action_destination(outline, 3, 1, &mut dest, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(dest.flags, 0);
        assert_eq!(dest.page_index, ROFD_NO_INDEX);
        let mut warnings = ptr::null_mut();
        assert_eq!(
            rofd_document_get_warnings(document.0, &mut warnings, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        let mut count = 0;
        assert_eq!(
            rofd_warning_list_get_count(warnings, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert!(count >= 2);
        let mut codes = std::collections::BTreeSet::new();
        for index in 0..count {
            let mut warning = rofd_warning_t {
                struct_size: size_of::<rofd_warning_t>() as u32,
                code: 0,
                path: ptr::null(),
                message: ptr::null(),
            };
            assert_eq!(
                rofd_warning_list_get_warning(warnings, index, &mut warning, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            assert!(!warning.path.is_null() && !warning.message.is_null());
            codes.insert(warning.code);
        }
        assert_eq!(
            codes,
            std::collections::BTreeSet::from([
                ROFD_WARNING_NAVIGATION_INVALID,
                ROFD_WARNING_NAVIGATION_UNSUPPORTED,
                ROFD_WARNING_NAVIGATION_COMPATIBILITY,
            ])
        );
        rofd_warning_list_free(warnings);
        rofd_outline_free(outline);
    }
}

#[test]
fn outline_queries_reject_null_indices_aliases_and_malformed_addresses() {
    let document = Document::new();
    let outline = document.outline();
    // SAFETY: Invalid-layout pointers are rejected before dereference; other storage is live.
    unsafe {
        assert_eq!(
            rofd_document_get_outline(ptr::null(), ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_outline_get_count(ptr::null(), ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_outline_get_node(outline, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_outline_get_action(outline, 0, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_outline_get_action_destination(outline, 0, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        let mut value = node();
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_outline_get_node(outline, usize::MAX, &mut value, &mut error),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert!(value.title.is_null());
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_PAGE_OUT_OF_RANGE);
        rofd_error_free(error);
        let mut act = action();
        assert_eq!(
            rofd_outline_get_action(outline, 0, usize::MAX, &mut act, ptr::null_mut()),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert!(act.type_name.is_null());
        let mut count = 0x12345678usize;
        assert_eq!(
            rofd_outline_get_count(outline, &mut count, (&mut count as *mut usize).cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0x12345678);
        assert_eq!(
            rofd_outline_get_count(ptr::dangling::<u8>().cast(), &mut count, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0x12345678);
        assert_eq!(
            rofd_outline_get_node(
                outline,
                0,
                ptr::dangling_mut::<u8>().cast(),
                ptr::null_mut()
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_outline_get_node(outline, 0, outline.cast(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_outline_get_count(outline, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 4);
        rofd_outline_free(outline);
        rofd_outline_free(ptr::null_mut());
    }
}

#[test]
fn all_navigation_record_prefixes_preserve_unknown_tails_and_declared_sizes() {
    let document = Document::new();
    let outline = document.outline();
    macro_rules! check_record {
        ($record:ty, $query:expr, $failure:expr, [$($field:ident),+]) => {{
            #[repr(C)]
            struct Extended {
                record: $record,
                tail: [u8; 17],
            }
            let mut storage = MaybeUninit::<Extended>::uninit();
            // SAFETY: The entire backing storage is initialized before any field read.
            unsafe {
                storage
                    .as_mut_ptr()
                    .cast::<u8>()
                    .write_bytes(0xa5, size_of::<Extended>());
                let record = ptr::addr_of_mut!((*storage.as_mut_ptr()).record);
                (*record).struct_size = size_of::<Extended>() as u32;
                assert_eq!($query(record), ROFD_STATUS_OK);
                assert_eq!((*record).struct_size, size_of::<Extended>() as u32);
                assert_eq!((*storage.as_ptr()).tail, [0xa5; 17]);
                let mut fields = [false; size_of::<$record>()];
                $(
                    let start = offset_of!($record, $field);
                    let len = size_of_val(&(*record).$field);
                    fields[start..start + len].fill(true);
                )+
                let bytes = std::slice::from_raw_parts(record.cast::<u8>(), size_of::<$record>());
                for (index, is_field) in fields.into_iter().enumerate() {
                    if !is_field {
                        assert_eq!(bytes[index], 0, "padding byte {index}");
                    }
                }
                assert_eq!($failure(record), ROFD_STATUS_PAGE_OUT_OF_RANGE);
                let bytes = std::slice::from_raw_parts(record.cast::<u8>(), size_of::<$record>());
                assert_eq!(&bytes[..4], &(size_of::<Extended>() as u32).to_ne_bytes());
                assert!(bytes[4..].iter().all(|&byte| byte == 0));
                assert_eq!((*storage.as_ptr()).tail, [0xa5; 17]);
                (*record).struct_size = 4;
                assert_eq!($query(record), ROFD_STATUS_INVALID_ARGUMENT);
                assert_eq!((*record).struct_size, 4);
                assert_eq!((*storage.as_ptr()).tail, [0xa5; 17]);
            }
        }};
    }
    check_record!(
        rofd_outline_node_t,
        |record| rofd_outline_get_node(outline, 0, record, ptr::null_mut()),
        |record| rofd_outline_get_node(outline, usize::MAX, record, ptr::null_mut()),
        [
            struct_size,
            expanded,
            title,
            parent,
            first_child,
            next_sibling,
            action_count
        ]
    );
    check_record!(
        rofd_action_t,
        |record| rofd_outline_get_action(outline, 0, 0, record, ptr::null_mut()),
        |record| rofd_outline_get_action(outline, 0, usize::MAX, record, ptr::null_mut()),
        [
            struct_size,
            kind,
            event,
            flags,
            type_name,
            event_name,
            uri,
            uri_base,
            attachment_id,
            bookmark
        ]
    );
    check_record!(
        rofd_destination_t,
        |record| rofd_outline_get_action_destination(outline, 0, 0, record, ptr::null_mut()),
        |record| rofd_outline_get_action_destination(
            outline,
            0,
            usize::MAX,
            record,
            ptr::null_mut()
        ),
        [
            struct_size,
            kind,
            flags,
            page_index,
            page_id,
            mode_name,
            left_mm,
            top_mm,
            right_mm,
            bottom_mm,
            zoom
        ]
    );
    // SAFETY: This test uniquely owns the snapshot.
    unsafe { rofd_outline_free(outline) };
}

#[test]
fn navigation_record_alias_preflight_preserves_inputs_and_publishes_independent_errors() {
    let document = Document::new();
    let outline = document.outline();
    macro_rules! check_record {
        ($record:ty, $query:expr) => {{
            let mut storage = MaybeUninit::<$record>::uninit();
            // SAFETY: All bytes are initialized. Bad layout/overlap is rejected before
            // pointer dereference or output initialization; independent errors are owned.
            unsafe {
                let record = storage.as_mut_ptr();
                record.cast::<u8>().write_bytes(0xa5, size_of::<$record>());
                (*record).struct_size = size_of::<$record>() as u32;
                let before =
                    std::slice::from_raw_parts(record.cast::<u8>(), size_of::<$record>()).to_vec();
                assert_eq!($query(record, record.cast()), ROFD_STATUS_INVALID_ARGUMENT);
                assert_eq!(
                    std::slice::from_raw_parts(record.cast::<u8>(), size_of::<$record>()),
                    before
                );

                let mut error = ptr::null_mut();
                assert_eq!(
                    $query(outline.cast(), &mut error),
                    ROFD_STATUS_INVALID_ARGUMENT
                );
                assert!(!error.is_null());
                assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
                rofd_error_free(error);

                let overflow = (usize::MAX & !(align_of::<$record>() - 1)) as *mut $record;
                error = ptr::null_mut();
                assert_eq!($query(overflow, &mut error), ROFD_STATUS_INVALID_ARGUMENT);
                assert!(
                    error.is_null(),
                    "malformed layouts leave every output untouched"
                );
            }
        }};
    }
    check_record!(rofd_outline_node_t, |record, error| rofd_outline_get_node(
        outline, 0, record, error
    ));
    check_record!(rofd_action_t, |record, error| rofd_outline_get_action(
        outline, 0, 0, record, error
    ));
    check_record!(rofd_destination_t, |record, error| {
        rofd_outline_get_action_destination(outline, 0, 0, record, error)
    });
    // SAFETY: All rejected calls must have left this owned snapshot unchanged.
    unsafe {
        let mut count = 0;
        assert_eq!(
            rofd_outline_get_count(outline, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 4);
        rofd_outline_free(outline);
    }
}
