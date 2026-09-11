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
            r#"<Page><Area><PhysicalBox>0 0 100 100</PhysicalBox></Area><Actions>
              <Action Event="PO"><URI URI="file:///not-opened"/></Action>
              <Action Event="CLICK"><Region>
                <Area Start="1 2"><Line Point1="4 2"/><Line Point1="4 6"/><Close/></Area>
                <Area Start="10 20"><Line Point1="15 20"/><Line Point1="15 28"/><Close/></Area>
              </Region><Goto><Dest Type="XYZ" PageID="42" Left="12.5" Top="24" Zoom="0"/></Goto></Action>
              <Action Event="CLICK"><GotoA AttachID="attached-file" NewWindow="false"/></Action>
              <Action Event="CUSTOM"><Movie ResourceID="99"/></Action>
            </Actions><Content><Layer ID="1"><PathObject ID="2" Boundary="10 20 20 10" CTM="20 0 0 10 0 0" Stroke="false" Fill="true"><AbbreviatedData>M 0 0 L 1 0 L 1 1 C</AbbreviatedData><Actions><Action Event="CLICK"><URI URI="relative" Base="https://example.invalid/"/></Action></Actions></PathObject></Layer></Content></Page>"#,
        ),
    ] {
        zip.start_file(path, SimpleFileOptions::default())?;
        zip.write_all(xml.as_bytes())?;
    }
    zip.finish()?;
    Ok(())
}
