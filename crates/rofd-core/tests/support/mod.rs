use std::io::{Cursor, Write};

use zip::{write::SimpleFileOptions, ZipWriter};

#[allow(dead_code)]
pub fn ofd_with_doc_bodies(page_xml: &str, doc_body_count: usize) -> Vec<u8> {
    let body = r#"<ofd:DocBody>
    <ofd:DocInfo>
      <ofd:DocID>fixture-id</ofd:DocID>
      <ofd:Title>Fixture</ofd:Title>
      <ofd:Creator>rofd tests</ofd:Creator>
    </ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
  </ofd:DocBody>"#;
    let ofd_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">{}</ofd:OFD>"#,
        body.repeat(doc_body_count)
    );
    let entries = [
        ("OFD.xml", ofd_xml.as_str()),
        (
            "Doc_0/Document.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    <ofd:MaxUnitID>2</ofd:MaxUnitID>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="2" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#,
        ),
        ("Doc_0/Pages/Page_0/Content.xml", page_xml),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[allow(dead_code)]
pub fn minimal_ofd(page_xml: &str) -> Vec<u8> {
    ofd_with_doc_bodies(page_xml, 1)
}

#[allow(dead_code)]
pub fn ofd_with_entries(document_xml: &str, entries: &[(&str, &[u8])]) -> Vec<u8> {
    let ofd_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">
  <ofd:DocBody><ofd:DocInfo><ofd:DocID>fixture-id</ofd:DocID></ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody>
</ofd:OFD>"#;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in [
        ("OFD.xml", ofd_xml.as_slice()),
        ("Doc_0/Document.xml", document_xml.as_bytes()),
        ("Doc_0/Pages/Page_0/Content.xml", PAGE_XML.as_bytes()),
    ]
    .into_iter()
    .chain(entries.iter().copied())
    {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[allow(dead_code)]
pub fn ofd_with_document_page_and_entries(
    document_xml: &str,
    page_xml: &str,
    entries: &[(&str, &[u8])],
) -> Vec<u8> {
    let ofd_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody>
  <ofd:DocInfo><ofd:DocID>fixture-id</ofd:DocID></ofd:DocInfo>
  <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
</ofd:DocBody></ofd:OFD>"#;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in [
        ("OFD.xml", ofd_xml.as_slice()),
        ("Doc_0/Document.xml", document_xml.as_bytes()),
        ("Doc_0/Pages/Page_0/Content.xml", page_xml.as_bytes()),
    ]
    .into_iter()
    .chain(entries.iter().copied())
    {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[allow(dead_code)]
pub fn ofd_with_document_and_entries(document_xml: &str, entries: &[(&str, &[u8])]) -> Vec<u8> {
    let ofd_xml = br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody>
  <ofd:DocInfo><ofd:DocID>fixture-id</ofd:DocID></ofd:DocInfo>
  <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
</ofd:DocBody></ofd:OFD>"#;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in [
        ("OFD.xml", ofd_xml.as_slice()),
        ("Doc_0/Document.xml", document_xml.as_bytes()),
    ]
    .into_iter()
    .chain(entries.iter().copied())
    {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[allow(dead_code)]
pub const PAGE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;
