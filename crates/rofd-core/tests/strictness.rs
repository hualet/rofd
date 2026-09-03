mod support;

use rofd_core::{Document, Error, LoadOptions, Strictness, WarningCode};
use support::{minimal_ofd, PAGE_XML};

const PAGE_WITHOUT_AREA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;

#[test]
fn lenient_mode_falls_back_and_records_warning() {
    let document =
        Document::from_bytes(minimal_ofd(PAGE_WITHOUT_AREA), LoadOptions::default()).unwrap();
    document.page(0).unwrap();
    document.page(0).unwrap();
    assert_eq!(document.warnings().len(), 1);
    assert_eq!(document.warnings()[0].code, WarningCode::PageAreaFallback);
}

#[test]
fn conforming_page_does_not_record_warning() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    document.page(0).unwrap();
    assert!(document.warnings().is_empty());
}

#[test]
fn strict_mode_rejects_missing_page_area() {
    let options = LoadOptions {
        strictness: Strictness::Strict,
        ..LoadOptions::default()
    };
    let document = Document::from_bytes(minimal_ofd(PAGE_WITHOUT_AREA), options).unwrap();
    let error = document.page(0).unwrap_err();
    assert!(matches!(error, Error::InvalidStructure { .. }));
}
