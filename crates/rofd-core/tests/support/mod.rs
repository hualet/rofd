use std::io::{Cursor, Write};

use zip::{write::SimpleFileOptions, ZipWriter};

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

pub fn minimal_ofd(page_xml: &str) -> Vec<u8> {
    ofd_with_doc_bodies(page_xml, 1)
}

#[allow(dead_code)]
pub const PAGE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;
