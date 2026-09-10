use std::ffi::{c_char, CStr, CString};
use std::io::{Cursor, Write};
use std::mem::{align_of, offset_of, size_of, MaybeUninit};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use rofd_ffi::*;
use zip::{write::SimpleFileOptions, ZipWriter};

struct Document(*mut rofd_document_t);

impl Document {
    fn new(info: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let ofd = format!("<OFD><DocBody><DocInfo>{info}</DocInfo><DocRoot>Document.xml</DocRoot></DocBody></OFD>");
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (path, xml) in [
            ("OFD.xml", ofd.as_str()),
            ("Document.xml", "<Document><CommonData><PageArea><PhysicalBox>0 0 210 297</PhysicalBox></PageArea></CommonData><Pages><Page ID=\"1\" BaseLoc=\"Page.xml\"/></Pages></Document>"),
            ("Page.xml", "<Page><Content><Layer ID=\"2\"/></Content></Page>"),
        ] {
            zip.start_file(path, SimpleFileOptions::default()).unwrap();
            zip.write_all(xml.as_bytes()).unwrap();
        }
        let path = std::env::temp_dir().join(format!(
            "rofd-metadata-{}-{}.ofd",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let mut handle = ptr::null_mut();
        // SAFETY: Live input path and a disjoint writable output. NULL options select defaults.
        let status = unsafe {
            rofd_document_open(cpath.as_ptr(), ptr::null(), &mut handle, ptr::null_mut())
        };
        std::fs::remove_file(path).unwrap();
        assert_eq!(status, ROFD_STATUS_OK);
        Self(handle)
    }

    fn metadata(&self) -> *mut rofd_metadata_t {
        let mut metadata = ptr::null_mut();
        // SAFETY: The document is live and the output is independent writable storage.
        unsafe {
            assert_eq!(
                rofd_document_get_metadata(self.0, &mut metadata, ptr::null_mut()),
                ROFD_STATUS_OK
            );
        }
        metadata
    }

    fn warnings(&self) -> *mut rofd_warning_list_t {
        let mut warnings = ptr::null_mut();
        // SAFETY: The document is live and the output is independent writable storage.
        unsafe {
            assert_eq!(
                rofd_document_get_warnings(self.0, &mut warnings, ptr::null_mut()),
                ROFD_STATUS_OK
            );
        }
        warnings
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        // SAFETY: This wrapper owns the live handle exactly once.
        unsafe {
            rofd_document_free(self.0);
        }
    }
}

type Reader = unsafe extern "C" fn(*const rofd_metadata_t) -> *const c_char;
const READERS: [Reader; 9] = [
    rofd_metadata_get_document_id,
    rofd_metadata_get_title,
    rofd_metadata_get_author,
    rofd_metadata_get_subject,
    rofd_metadata_get_abstract,
    rofd_metadata_get_creator,
    rofd_metadata_get_creator_version,
    rofd_metadata_get_creation_date,
    rofd_metadata_get_modification_date,
];

fn warning() -> rofd_warning_t {
    rofd_warning_t {
        struct_size: size_of::<rofd_warning_t>() as u32,
        code: 77,
        path: ptr::dangling(),
        message: ptr::dangling(),
    }
}

#[test]
fn metadata_is_owned_with_all_fields_utf8_keywords_and_optional_values() {
    let document = Document::new("<DocID>文档</DocID><Title>标题</Title><Author>作者</Author><Subject>主题</Subject><Abstract>摘要</Abstract><Creator>应用</Creator><CreatorVersion>1.2</CreatorVersion><CreationDate>2026-09-09</CreationDate><ModDate>2026-09-10</ModDate><Keywords><Keyword>one</Keyword><Keyword>二</Keyword><Keyword>one</Keyword><Keyword/></Keywords>");
    let metadata = document.metadata();
    drop(document);
    // SAFETY: The independently owned metadata is kept live for all reads and freed once.
    unsafe {
        for (reader, expected) in READERS.into_iter().zip([
            "文档",
            "标题",
            "作者",
            "主题",
            "摘要",
            "应用",
            "1.2",
            "2026-09-09",
            "2026-09-10",
        ]) {
            assert_eq!(CStr::from_ptr(reader(metadata)).to_str().unwrap(), expected);
        }
        let mut count = usize::MAX;
        assert_eq!(
            rofd_metadata_get_keyword_count(metadata, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 4);
        for (index, expected) in ["one", "二", "one", ""].into_iter().enumerate() {
            let mut keyword = ptr::null();
            assert_eq!(
                rofd_metadata_get_keyword(metadata, index, &mut keyword, ptr::null_mut()),
                ROFD_STATUS_OK
            );
            assert_eq!(CStr::from_ptr(keyword).to_str().unwrap(), expected);
        }
        rofd_metadata_free(metadata);
    }
    let missing = Document::new("").metadata();
    let empty = Document::new("<Title/><Keywords/>").metadata();
    // SAFETY: Both snapshots remain live even though their source document wrappers were dropped.
    unsafe {
        for reader in READERS {
            assert!(reader(missing).is_null());
        }
        assert!(!rofd_metadata_get_title(empty).is_null());
        assert_eq!(
            CStr::from_ptr(rofd_metadata_get_title(empty)).to_bytes(),
            b""
        );
        let mut count = usize::MAX;
        assert_eq!(
            rofd_metadata_get_keyword_count(empty, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 0);
        rofd_metadata_free(missing);
        rofd_metadata_free(empty);
    }
}

#[test]
fn warnings_snapshot_is_lazy_and_stable_after_page_and_document_free() {
    let document = Document::new("");
    let before = document.warnings();
    let mut page = ptr::null_mut();
    // SAFETY: The document is live and page output is disjoint writable storage.
    unsafe {
        assert_eq!(
            rofd_document_get_page(document.0, 0, &mut page, ptr::null_mut()),
            ROFD_STATUS_OK
        );
    }
    let after = document.warnings();
    // SAFETY: Free the unique page handle before checking independently owned snapshots.
    unsafe {
        rofd_page_free(page);
    }
    drop(document);
    // SAFETY: Both snapshots remain live and every output is initialized and disjoint.
    unsafe {
        let mut count = usize::MAX;
        assert_eq!(
            rofd_warning_list_get_count(before, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 0);
        assert_eq!(
            rofd_warning_list_get_count(after, &mut count, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(count, 1);
        let mut value = warning();
        assert_eq!(
            rofd_warning_list_get_warning(after, 0, &mut value, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!(value.code, ROFD_WARNING_PAGE_AREA_FALLBACK);
        assert_eq!(CStr::from_ptr(value.path).to_bytes(), b"Page.xml");
        assert!(!CStr::from_ptr(value.message).to_bytes().is_empty());
        rofd_warning_list_free(before);
        rofd_warning_list_free(after);
    }
}

#[test]
fn null_handles_outputs_and_frees_obey_the_contract() {
    // SAFETY: NULL inputs are supported failures; each non-null output is aligned and writable.
    unsafe {
        for reader in READERS {
            assert!(reader(ptr::null()).is_null());
        }
        rofd_metadata_free(ptr::null_mut());
        rofd_warning_list_free(ptr::null_mut());
        let mut metadata = ptr::dangling_mut();
        let mut warnings = ptr::dangling_mut();
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_document_get_metadata(ptr::null(), &mut metadata, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(metadata.is_null());
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
        assert_eq!(
            rofd_document_get_warnings(ptr::null(), &mut warnings, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(warnings.is_null());
        let mut count = usize::MAX;
        assert_eq!(
            rofd_metadata_get_keyword_count(ptr::null(), &mut count, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0);
        count = usize::MAX;
        assert_eq!(
            rofd_warning_list_get_count(ptr::null(), &mut count, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 0);
        let mut keyword = ptr::dangling();
        assert_eq!(
            rofd_metadata_get_keyword(ptr::null(), 0, &mut keyword, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert!(keyword.is_null());
        let mut value = warning();
        assert_eq!(
            rofd_warning_list_get_warning(ptr::null(), 0, &mut value, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(value.code, 0);
        assert!(value.path.is_null() && value.message.is_null());
        let document = Document::new("");
        let metadata = document.metadata();
        let warnings = document.warnings();
        assert_eq!(
            rofd_document_get_metadata(document.0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_document_get_warnings(document.0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_metadata_get_keyword_count(metadata, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_metadata_get_keyword(metadata, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_warning_list_get_count(warnings, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, ptr::null_mut(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        rofd_metadata_free(metadata);
        rofd_warning_list_free(warnings);
    }
}

#[test]
fn out_of_range_and_versioned_warning_outputs_are_transactional() {
    let document = Document::new("");
    let metadata = document.metadata();
    let warnings = document.warnings();
    #[repr(C)]
    struct Extended {
        value: rofd_warning_t,
        tail: [u8; 17],
    }
    let mut value = MaybeUninit::<Extended>::uninit();
    // SAFETY: Initialize every byte before reading the record, then provide live disjoint outputs.
    unsafe {
        value
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xa5, size_of::<Extended>());
        let record = ptr::addr_of_mut!((*value.as_mut_ptr()).value);
        (*record).struct_size = size_of::<Extended>() as u32;
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_warning_list_get_warning(warnings, usize::MAX, record, &mut error),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_PAGE_OUT_OF_RANGE);
        rofd_error_free(error);
        assert_eq!((*record).struct_size, size_of::<Extended>() as u32);
        let bytes = std::slice::from_raw_parts(record.cast::<u8>(), size_of::<Extended>());
        assert!(bytes[4..size_of::<rofd_warning_t>()]
            .iter()
            .all(|b| *b == 0));
        assert!(bytes[size_of::<rofd_warning_t>()..]
            .iter()
            .all(|b| *b == 0xa5));
        (*record).struct_size = 4;
        (*record).code = 99;
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, record, ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!((*record).struct_size, 4);
        assert_eq!((*record).code, 99);
        let mut keyword = ptr::dangling();
        error = ptr::null_mut();
        assert_eq!(
            rofd_metadata_get_keyword(metadata, usize::MAX, &mut keyword, &mut error),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert!(keyword.is_null());
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_PAGE_OUT_OF_RANGE);
        rofd_error_free(error);
        rofd_metadata_free(metadata);
        rofd_warning_list_free(warnings);
    }
}

#[test]
fn snapshot_preflight_rejects_source_output_and_error_aliases() {
    let document = Document::new("<Title>unchanged</Title>");
    let metadata = document.metadata();
    let warnings = document.warnings();
    // SAFETY: Raw aliases exercise preflight rejection; none creates a simultaneous Rust reference.
    unsafe {
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_document_get_metadata(document.0, document.0.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
        assert_eq!(
            rofd_document_get_warnings(document.0, document.0.cast(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_metadata_get_keyword_count(metadata, metadata.cast(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_metadata_get_keyword(metadata, 0, metadata.cast(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_warning_list_get_count(warnings, warnings.cast(), ptr::null_mut()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        let mut count = 73;
        assert_eq!(
            rofd_metadata_get_keyword_count(metadata, &mut count, metadata.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 73);
        let mut shared = 81usize;
        let output = ptr::addr_of_mut!(shared);
        assert_eq!(
            rofd_metadata_get_keyword_count(metadata, output, output.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(shared, 81);
        let mut record = warning();
        let output = ptr::addr_of_mut!(record);
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, output, output.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(record.code, 77);
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, &mut record, warnings.cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(record.code, 77);
        assert_eq!(
            CStr::from_ptr(rofd_metadata_get_title(metadata)).to_bytes(),
            b"unchanged"
        );
        let mut page = ptr::null_mut();
        assert_eq!(
            rofd_document_get_page(document.0, 0, &mut page, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        rofd_page_free(page);
        rofd_metadata_free(metadata);
        rofd_warning_list_free(warnings);
    }
}

#[test]
fn invalid_address_layouts_leave_all_outputs_untouched() {
    let document = Document::new("");
    let metadata = document.metadata();
    let warnings = document.warnings();
    // SAFETY: Malformed addresses are never dereferenced; this explicitly checks preflight.
    unsafe {
        let mut result = ptr::dangling_mut();
        let mut error = ptr::dangling_mut();
        let malformed = ptr::dangling::<u8>().cast::<rofd_document_t>();
        assert_eq!(
            rofd_document_get_metadata(malformed, &mut result, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(result, ptr::dangling_mut());
        assert_eq!(error, ptr::dangling_mut());
        let mut list = ptr::dangling_mut();
        assert_eq!(
            rofd_document_get_warnings(malformed, &mut list, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(list, ptr::dangling_mut());
        let mut count = 97;
        assert_eq!(
            rofd_metadata_get_keyword_count(ptr::dangling::<u8>().cast(), &mut count, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 97);
        assert_eq!(
            rofd_warning_list_get_count(ptr::dangling::<u8>().cast(), &mut count, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 97);
        let mut record = warning();
        assert_eq!(
            rofd_warning_list_get_warning(ptr::dangling::<u8>().cast(), 0, &mut record, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(record.code, 77);
        let overflow = (usize::MAX - align_of::<usize>() + 1) as *const rofd_metadata_t;
        assert_eq!(
            rofd_metadata_get_keyword_count(overflow, &mut count, &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 97);
        assert_eq!(
            rofd_metadata_get_keyword_count(metadata, ptr::dangling_mut::<u8>().cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_warning_list_get_count(warnings, &mut count, ptr::dangling_mut::<u8>().cast()),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(count, 97);
        assert_eq!(
            rofd_warning_list_get_warning(
                warnings,
                0,
                ptr::dangling_mut::<u8>().cast(),
                &mut error
            ),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, overflow.cast_mut().cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(error, ptr::dangling_mut());
        rofd_metadata_free(metadata);
        rofd_warning_list_free(warnings);
    }
}

#[test]
fn warning_record_layout_and_public_codes_are_fixed() {
    assert_eq!(offset_of!(rofd_warning_t, struct_size), 0);
    assert_eq!(offset_of!(rofd_warning_t, code), 4);
    assert_eq!(offset_of!(rofd_warning_t, path), 8);
    assert_eq!(
        offset_of!(rofd_warning_t, message),
        8 + size_of::<*const c_char>()
    );
    assert_eq!(
        [
            ROFD_WARNING_UNKNOWN,
            ROFD_WARNING_PAGE_AREA_FALLBACK,
            ROFD_WARNING_DOCUMENT_PAGE_AREA_MISSING,
            ROFD_WARNING_SIGNATURE_SKIPPED,
            ROFD_WARNING_UNKNOWN_GRAPHIC_UNIT_SKIPPED,
            ROFD_WARNING_ANNOTATION_SKIPPED,
            ROFD_WARNING_HISTORICAL_DOC_BODY_SKIPPED,
        ],
        [0, 1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn successful_warning_output_preserves_declared_size_and_unknown_tail() {
    let document = Document::new("");
    let mut page = ptr::null_mut();
    // SAFETY: A live source document and a disjoint writable page output.
    unsafe {
        assert_eq!(
            rofd_document_get_page(document.0, 0, &mut page, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        rofd_page_free(page);
    }
    let warnings = document.warnings();
    #[repr(C)]
    struct Extended {
        value: rofd_warning_t,
        tail: [u8; 13],
    }
    let mut extended = MaybeUninit::<Extended>::uninit();
    // SAFETY: Initialize the entire record and tail before reading; all normal outputs are disjoint.
    unsafe {
        extended
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xa5, size_of::<Extended>());
        let record = ptr::addr_of_mut!((*extended.as_mut_ptr()).value);
        (*record).struct_size = size_of::<Extended>() as u32;
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, record, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        assert_eq!((*record).struct_size, size_of::<Extended>() as u32);
        assert_eq!((*record).code, ROFD_WARNING_PAGE_AREA_FALLBACK);
        let bytes =
            std::slice::from_raw_parts(extended.as_ptr().cast::<u8>(), size_of::<Extended>());
        assert!(bytes[size_of::<rofd_warning_t>()..]
            .iter()
            .all(|b| *b == 0xa5));
        // The source handle and record alias only as raw pointers; preflight must protect the handle.
        let mut error = ptr::null_mut();
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, warnings.cast(), &mut error),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(rofd_error_get_status(error), ROFD_STATUS_INVALID_ARGUMENT);
        rofd_error_free(error);
        assert_eq!(
            rofd_warning_list_get_warning(warnings, 0, record, ptr::null_mut()),
            ROFD_STATUS_OK
        );
        rofd_warning_list_free(warnings);
    }
}
