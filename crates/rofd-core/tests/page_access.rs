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

const DOCUMENT_WITHOUT_PAGE_AREA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData><ofd:MaxUnitID>2</ofd:MaxUnitID></ofd:CommonData>
  <ofd:Pages><ofd:Page ID="2" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#;

const PAGE_WITHOUT_AREA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;

#[test]
fn document_without_page_area_loads_leniently_and_warns() {
    // Real-world producers omit CommonData.PageArea when every page declares
    // its own Area (e.g. the ofdrw fixture 发票示例.ofd).
    let document = Document::from_bytes(
        support::ofd_with_document_page_and_entries(DOCUMENT_WITHOUT_PAGE_AREA, PAGE_XML, &[]),
        LoadOptions::default(),
    )
    .unwrap();
    assert_eq!(document.page(0).unwrap().size().width, 210.0);
    assert!(document
        .warnings()
        .iter()
        .any(|warning| warning.code == rofd_core::WarningCode::DocumentPageAreaMissing));
}

#[test]
fn document_without_page_area_is_rejected_in_strict_mode() {
    let error = Document::from_bytes(
        support::ofd_with_document_page_and_entries(DOCUMENT_WITHOUT_PAGE_AREA, PAGE_XML, &[]),
        LoadOptions {
            strictness: rofd_core::Strictness::Strict,
            ..LoadOptions::default()
        },
    )
    .unwrap_err();
    assert!(matches!(error, Error::InvalidStructure { .. }));
}

#[test]
fn page_without_any_area_cannot_be_sized() {
    let document = Document::from_bytes(
        support::ofd_with_document_page_and_entries(DOCUMENT_WITHOUT_PAGE_AREA, PAGE_WITHOUT_AREA, &[]),
        LoadOptions::default(),
    )
    .unwrap();
    assert!(matches!(
        document.page(0),
        Err(Error::InvalidStructure { .. })
    ));
}
