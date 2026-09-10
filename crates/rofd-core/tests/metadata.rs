use std::io::{Cursor, Write};

use rofd_core::{Document, LoadOptions, Metadata, WarningCode};
use zip::{write::SimpleFileOptions, ZipWriter};

fn document(info: &str) -> Document {
    let ofd = format!(
        "<OFD><DocBody><DocInfo>{info}</DocInfo><DocRoot>Document.xml</DocRoot></DocBody></OFD>"
    );
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, xml) in [
        ("OFD.xml", ofd.as_str()),
        ("Document.xml", "<Document><CommonData><PageArea><PhysicalBox>0 0 210 297</PhysicalBox></PageArea></CommonData><Pages><Page ID=\"1\" BaseLoc=\"Page.xml\"/></Pages></Document>"),
        ("Page.xml", "<Page><Content><Layer ID=\"2\"/></Content></Page>"),
    ] {
        zip.start_file(path, SimpleFileOptions::default()).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
    }
    Document::from_bytes(zip.finish().unwrap().into_inner(), LoadOptions::default()).unwrap()
}

#[test]
fn metadata_preserves_all_fields_and_ordered_repeated_empty_keywords() {
    let document = document(
        "<DocID>id-一</DocID><Title>标题</Title><Author>作者</Author><Subject>主题</Subject><Abstract>摘要</Abstract><Creator>应用</Creator><CreatorVersion>1.2</CreatorVersion><CreationDate>2026-09-09</CreationDate><ModDate>2026-09-10</ModDate><Keywords><Keyword>one</Keyword><Keyword>二</Keyword><Keyword>one</Keyword><Keyword/></Keywords>",
    );
    assert_eq!(
        document.metadata(),
        &Metadata {
            document_id: Some("id-一".into()),
            title: Some("标题".into()),
            author: Some("作者".into()),
            subject: Some("主题".into()),
            abstract_text: Some("摘要".into()),
            creator: Some("应用".into()),
            creator_version: Some("1.2".into()),
            creation_date: Some("2026-09-09".into()),
            modification_date: Some("2026-09-10".into()),
            keywords: vec!["one".into(), "二".into(), "one".into(), "".into()],
        }
    );
}

#[test]
fn missing_metadata_defaults_differ_from_explicitly_empty_values() {
    assert_eq!(document("").metadata(), &Metadata::default());
    let document = document("<Title/><Author></Author><Keywords/>");
    assert_eq!(document.metadata().title.as_deref(), Some(""));
    assert_eq!(document.metadata().author.as_deref(), Some(""));
    assert_eq!(document.metadata().subject, None);
    assert!(document.metadata().keywords.is_empty());
}

#[test]
fn warnings_are_lazy_independent_snapshots() {
    let document = document("");
    let initial = document.warnings();
    assert!(initial.is_empty());
    assert!(document.warnings().is_empty());
    document.page(0).unwrap();
    let after_page = document.warnings();
    assert_eq!(after_page.len(), 1);
    assert_eq!(after_page[0].code, WarningCode::PageAreaFallback);
    assert_eq!(after_page[0].path, "Page.xml");
    assert!(!after_page[0].message.is_empty());
    assert!(initial.is_empty());
    document.page(0).unwrap();
    assert_eq!(document.warnings(), after_page);
}
