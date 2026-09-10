use std::path::PathBuf;

use rofd_core::{Document, FindOptions, LoadOptions, Rect};

#[test]
fn opens_repository_invoice_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    let document = Document::open(path, LoadOptions::default()).unwrap();
    assert_eq!(document.page_count(), 1);
    let page = document.page(0).unwrap();
    let text = page.text().unwrap();
    assert!(text.as_str().contains("电子发票（普通发票）"));
    assert_eq!(
        text.find("发票号码", FindOptions::default()).unwrap().len(),
        1
    );
    assert_eq!(
        document.metadata().document_id.as_deref(),
        Some("2195d5df959c419cb575dab5eeabb065")
    );
    assert_eq!(
        document.page(0).unwrap().size(),
        Rect {
            x: 0.0,
            y: 0.0,
            width: 211.5,
            height: 140.0,
        }
    );
}
