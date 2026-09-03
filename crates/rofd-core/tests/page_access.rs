mod support;

use rofd_core::{Document, Error, LoadOptions, Rect};
use support::{minimal_ofd, PAGE_XML};

#[test]
fn loads_page_size_and_identity() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    assert_eq!(page.index(), 0);
    assert_eq!(page.object_id(), 2);
    assert_eq!(
        page.size(),
        Rect {
            x: 0.0,
            y: 0.0,
            width: 210.0,
            height: 297.0,
        }
    );
}

#[test]
fn page_uses_document_area_when_page_area_is_absent() {
    let page_without_area = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;
    let document =
        Document::from_bytes(minimal_ofd(page_without_area), LoadOptions::default()).unwrap();
    assert_eq!(document.page(0).unwrap().size().width, 210.0);
}

#[test]
fn reports_out_of_range_page() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    let error = document.page(1).unwrap_err();
    assert!(matches!(
        error,
        Error::PageOutOfRange {
            index: 1,
            page_count: 1
        }
    ));
}
