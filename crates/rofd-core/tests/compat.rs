mod support;

use std::io::{Cursor, Write};
use std::path::PathBuf;

use rofd_core::{Document, Error, ImageFormat, LoadOptions};
use zip::{write::SimpleFileOptions, ZipWriter};

fn package(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in files {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

const DOCUMENT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    <ofd:DocumentRes>Res.xml</ofd:DocumentRes>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="2" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#;

const IMAGE_PAGE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content><ofd:Layer ID="1"><ofd:ImageObject ID="2" Boundary="0 0 10 10" ResourceID="10"/></ofd:Layer></ofd:Content>
</ofd:Page>"#;

#[test]
fn utf8_chinese_entry_names_resolve_end_to_end() {
    let bytes = package(&[
        ("OFD.xml", support::OFD_XML.as_bytes()),
        ("Doc_0/Document.xml", DOCUMENT_XML.as_bytes()),
        (
            "Doc_0/Res.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
             <ofd:Res xmlns:ofd=\"http://www.ofdspec.org/2016\"><ofd:MultiMedias>
             <ofd:MultiMedia ID=\"10\" Type=\"Image\"><ofd:MediaFile>资源/图像_10.png</ofd:MediaFile></ofd:MultiMedia>
             </ofd:MultiMedias></ofd:Res>"
                .as_bytes(),
        ),
        ("Doc_0/Pages/Page_0/Content.xml", IMAGE_PAGE_XML.as_bytes()),
        ("Doc_0/资源/图像_10.png", b"fake-png-bytes"),
    ]);
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    document.page(0).unwrap();
    let image = document.image_resource(10).unwrap();
    assert_eq!(image.format(), ImageFormat::Png);
    assert_eq!(image.encoded_bytes(), b"fake-png-bytes");
    assert_eq!(image.asset_path(), "Doc_0/资源/图像_10.png");
}

#[test]
fn page_files_may_use_arbitrary_non_conventional_names() {
    // Regression coverage for ofdrw issue #293: page files are located
    // through their explicit BaseLoc, not through a Page_N naming convention.
    let document_xml = DOCUMENT_XML.replace("Pages/Page_0/Content.xml", "Pages/第1页.xml");
    let bytes = package(&[
        ("OFD.xml", support::OFD_XML.as_bytes()),
        ("Doc_0/Document.xml", document_xml.as_bytes()),
        ("Doc_0/Pages/第1页.xml", support::PAGE_XML.as_bytes()),
    ]);
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    assert_eq!(document.page_count(), 1);
    document.page(0).unwrap();
}

#[test]
fn namespace_prefix_and_uri_are_not_strictly_validated() {
    // Real-world documents omit the OFD namespace or carry a wrong URI;
    // elements are matched by local name.
    for document_xml in [
        DOCUMENT_XML.replace(
            " xmlns:ofd=\"http://www.ofdspec.org/2016\"",
            " xmlns:ofd=\"http://example.com/wrong\"",
        ),
        DOCUMENT_XML
            .replace("ofd:", "")
            .replace(" xmlns=\"http://www.ofdspec.org/2016\"", ""),
    ] {
        let bytes = package(&[
            ("OFD.xml", support::OFD_XML.as_bytes()),
            ("Doc_0/Document.xml", document_xml.as_bytes()),
            (
                "Doc_0/Pages/Page_0/Content.xml",
                support::PAGE_XML.as_bytes(),
            ),
        ]);
        Document::from_bytes(bytes, LoadOptions::default())
            .unwrap()
            .page(0)
            .unwrap();
    }
}

#[test]
fn gbk_encoded_entry_names_fail_with_structured_missing_entry() {
    // The zip crate decodes names without the UTF-8 flag as CP437, so a
    // GBK-encoded entry name can never match the UTF-8 reference in the XML.
    // The failure must be a structured MissingEntry, not a panic or silent
    // mojibake match.
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gbk-entry-name.ofd");
    let document = Document::open(fixture, LoadOptions::default()).unwrap();
    document.page(0).unwrap();
    assert!(matches!(
        document.image_resource(10),
        Err(Error::MissingEntry(_))
    ));
}
