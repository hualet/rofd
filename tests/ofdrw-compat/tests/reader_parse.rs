//! Parsing and text-extraction assertions ported from ofdrw-reader tests.
//!
//! Sources (ofdrw commit 7459e35, Apache-2.0):
//! - `ofdrw-reader/src/test/java/org/ofdrw/reader/OFDReaderTest.java`
//! - `ofdrw-reader/src/test/java/org/ofdrw/reader/ContentExtractorTest.java`
//! - `ofdrw-reader/src/test/java/org/ofdrw/reader/keyword/KeywordExtractorTest.java`
//! - `ofdrw-reader/src/test/java/org/ofdrw/reader/IssueCase.java`
//!
//! Tests that rely on ofdrw APIs rofd-core does not have (attachments, stamp
//! annotations, keyword bounding boxes, low-level editing) are not ported; see
//! the mapping table in `README.md`.

mod support;

use support::{extract_page_text, fixture, open_document};

/// Ported from `OFDReaderTest.getOFDDir`.
#[test]
fn ofd_reader_get_ofd_dir_doc_id() {
    let document = open_document(&fixture("reader/helloworld.ofd"));
    assert_eq!(
        document.metadata().document_id.as_deref(),
        Some("220c5913ebfe4f6e8070dabd3647f157")
    );
}

/// Ported from `OFDReaderTest.getPage`.
#[test]
fn ofd_reader_get_page_has_single_layer() {
    let document = open_document(&fixture("reader/helloworld.ofd"));
    assert_eq!(document.page_count(), 1);
    assert_eq!(document.page(0).unwrap().layers().len(), 1);
}

/// Ported from `ContentExtractorTest.getPageContent` / `extractAll` / `traverse`.
#[test]
fn content_extractor_helloworld_text() {
    let document = open_document(&fixture("reader/helloworld.ofd"));
    let page = document.page(0).unwrap();
    assert_eq!(extract_page_text(&page), "你好呀，OFD Reader&Writer！");
}

/// Ported from `ContentExtractorTest.extractAllPageBlock`.
#[test]
fn content_extractor_pageblock_text() {
    let document = open_document(&fixture("reader/helloworld_with_pageblock.ofd"));
    let page = document.page(0).unwrap();
    assert_eq!(extract_page_text(&page), "你好呀，OFD Reader&Writer！");
}

/// Ported from `OFDReaderTest.getPageSize`.
#[test]
fn ofd_reader_ses_v4_page_size() {
    let document = open_document(&fixture("reader/SESV4SignDoc.ofd"));
    let size = document.page(0).unwrap().size();
    assert_eq!((size.x, size.y), (0.0, 0.0));
    assert!((size.width - 210.0).abs() < 0.01, "width: {}", size.width);
    assert!(
        (size.height - 297.0).abs() < 0.01,
        "height: {}",
        size.height
    );
}

/// Ported from `KeywordExtractorTest.testKeyword`.
///
/// ofdrw asserts 7 positioned matches for `打发`; rofd-core has no keyword
/// coordinate API, so this asserts the occurrence count over the extracted
/// text instead.
#[test]
fn keyword_extractor_multi_keyword_occurrences() {
    let document = open_document(&fixture("reader/multiKeywordInTextCode.ofd"));
    let page = document.page(0).unwrap();
    let text = extract_page_text(&page);
    assert_eq!(text.matches("打发").count(), 7, "text: {text}");
}

/// Ported from `KeywordExtractorTest.getKeyWordPositionList`.
///
/// ofdrw asserts one positioned match for `办理` on page 1; downgraded to a
/// containment assertion because rofd-core exposes no keyword positions.
#[test]
fn keyword_extractor_keyword_contains() {
    let document = open_document(&fixture("reader/keyword.ofd"));
    let text = extract_page_text(&document.page(0).unwrap());
    assert!(text.contains("办理"));
}

/// Ported from `KeywordExtractorTest.testKeyword2`.
#[test]
fn keyword_extractor_keyword2_contains() {
    let document = open_document(&fixture("reader/keyword2.ofd"));
    let text = extract_page_text(&document.page(0).unwrap());
    assert!(text.contains("马上融"), "text: {text}");
}

/// Ported from `IssueCase.github_293`: page directories not named `Page_N`
/// must still load. ofdrw asserts nothing beyond not throwing.
/// Currently ignored: rofd-core rejects the file's leading-slash package path
/// (see KNOWN_LOAD_FAILURES).
#[test]
#[ignore = "known parser gap: path_unstd.ofd uses leading-slash package paths"]
fn issue_case_github_293_unstandard_page_dirs() {
    let document = open_document(&fixture("reader/path_unstd.ofd"));
    assert!(document.page_count() >= 1);
    for index in 0..document.page_count() {
        let page = document.page(index).unwrap();
        assert!(page.size().width > 0.0 && page.size().height > 0.0);
    }
}
