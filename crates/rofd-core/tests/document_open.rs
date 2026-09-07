mod support;

use rofd_core::{Document, Error, LoadOptions};
use support::{minimal_ofd, PAGE_XML};

#[test]
fn opens_document_and_exposes_metadata_and_page_count() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    assert_eq!(document.page_count(), 1);
    assert_eq!(
        document.metadata().document_id.as_deref(),
        Some("fixture-id")
    );
    assert_eq!(document.metadata().title.as_deref(), Some("Fixture"));
    assert_eq!(document.metadata().creator.as_deref(), Some("rofd tests"));
}

#[test]
fn missing_entry_point_is_reported() {
    let error = Document::from_bytes(Vec::new(), LoadOptions::default()).unwrap_err();
    assert!(matches!(error, Error::Container(_)));
}

#[test]
fn missing_file_on_disk_is_a_structured_io_error() {
    let error =
        Document::open("definitely/not/a/real/file.ofd", LoadOptions::default()).unwrap_err();
    assert!(matches!(error, Error::Io { .. }));
}

#[test]
fn multiple_doc_bodies_are_explicitly_unsupported_in_v02() {
    let error = Document::from_bytes(
        support::ofd_with_doc_bodies(PAGE_XML, 2),
        LoadOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(error, Error::UnsupportedFeature(_)));
}
