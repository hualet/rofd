//! Deterministic synthetic OFD for dynamically linked C ABI consumers.

use std::fs::File;
use std::io::Write;

use zip::{write::SimpleFileOptions, ZipWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .ok_or("expected output OFD path")?;
    let mut zip = ZipWriter::new(File::create(output)?);
    for (path, xml) in [
        (
            "OFD.xml",
            r#"<OFD><DocBody><DocInfo><DocID>rofd-c-abi-navigation</DocID></DocInfo><DocRoot>Document.xml</DocRoot></DocBody></OFD>"#,
        ),
        (
            "Document.xml",
            r#"<Document><CommonData><PageArea><PhysicalBox>0 0 100 100</PhysicalBox></PageArea></CommonData>
            <Pages><Page ID="1" BaseLoc="Page.xml"/><Page ID="42" BaseLoc="Page.xml"/></Pages>
            <Outlines><OutlineElem Title="目录" Expanded="false" Count="500">
                <OutlineElem Title="第二页"><Actions><Action Event="CLICK"><Goto><Dest Type="XYZ" PageID="42" Left="12.5" Top="24"/></Goto></Action></Actions></OutlineElem>
              </OutlineElem><OutlineElem Title="外部动作"><Actions>
                <Action Event="CLICK"><URI URI="https://example.invalid/"/></Action>
                <Action Event="CLICK"><GotoA AttachID="attached-file"/></Action>
                <Action Event="CLICK"><Movie ResourceID="99"/></Action>
              </Actions></OutlineElem></Outlines>
          </Document>"#,
        ),
        (
            "Page.xml",
            r#"<Page><Area><PhysicalBox>0 0 100 100</PhysicalBox></Area></Page>"#,
        ),
    ] {
        zip.start_file(path, SimpleFileOptions::default())?;
        zip.write_all(xml.as_bytes())?;
    }
    zip.finish()?;
    Ok(())
}
