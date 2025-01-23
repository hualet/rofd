mod document;
mod document_res;
mod ofd;
mod page;
mod render;
mod types;

use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use zip::ZipArchive;

use document::Document;
use document_res::Res;
use ofd::{Ofd, OfdNode};
use page::Page;
use render::Renderable;
use types::CT_PageArea;

pub fn read_ofd(file_path: &str) -> Result<Ofd, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);
    let mut zip = ZipArchive::new(reader)?;

    let mut content = String::new();

    // Find the OFD.xml file and parse the content to ofd object.
    {
        let mut ofd_file = zip.by_name("OFD.xml")?;
        ofd_file.read_to_string(&mut content)?;
    }

    // Parse the XML content into an OfdNode.
    let ofd_node = OfdNode::from_xml(&content)?;

    let ofd_result = Ofd {
        node: ofd_node,
        zip_archive: zip,
    };

    Ok(ofd_result)
}

pub fn export_ofd_to_png(ofd: &mut Ofd, output_path: &str) -> Result<(), Box<dyn Error>> {
    // create a new String to store the content of the DocRoot file.
    let mut content = String::new();

    // Find the DocRoot file and parse the content to a document object.
    {
        let doc_root_path = ofd.node.doc_body.doc_root.as_str();
        let mut doc_file = ofd.zip_archive.by_name(doc_root_path)?;
        doc_file.read_to_string(&mut content)?;
    }

    let mut document = Document::from_xml(&content)?;
    println!("document: {:#?}", document);

    // Find the DocumentRes.xml file and parse the content to a document_res object.
    {
        let doc_root_path = ofd.node.doc_body.doc_root.as_str();
        let root_path = Path::new(doc_root_path);
        let res_path = root_path
                        .parent().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent directory not found"))?
                        .join(document.common_data.document_res.as_str());
        let mut document_res_file = ofd.zip_archive.by_name(res_path.to_str().unwrap())?;

        content.clear();
        document_res_file.read_to_string(&mut content)?;
    }

    let document_res = Res::from_xml(&content)?;
    println!("document_res: {:#?}", document_res);
    document.res = document_res;

    // Create a cairo surface and context.
    let pybox = CT_PageArea::from(document.common_data.page_area.physical_box.clone()).toPixel();
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        pybox.width as i32,
        pybox.height as i32,
    )?;

    let mut context = cairo::Context::new(&surface)?;

    // draw page
    let pages = &document.pages.page;
    for page_info in pages {
        content.clear();

        // concat basename of document.doc_body.doc_root and page.base_loc
        {
            let doc_root_path = ofd.node.doc_body.doc_root.as_str();
            let root_path = Path::new(doc_root_path);
            let page_path = root_path
                            .parent()
                            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent directory not found"))?
                            .join(page_info.base_loc.as_ref()
                                                    .ok_or_else(||
                                                        {io::Error::new(io::ErrorKind::InvalidInput, "Page base location not found")})?
                                );
            let mut page_file = ofd.zip_archive.by_name(page_path.to_str().unwrap())?;
            page_file.read_to_string(&mut content)?;
        }
        let page = Page::from_xml(&content)?;
        page.render(&mut context, ofd, &document);
    }

    let mut file = File::create(output_path)?;
    surface.write_to_png(&mut file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_read_ofd() {
        let ofd_node = read_ofd("./learning/test.ofd").unwrap();
        println!("ofd: {:?}", ofd_node);
    }

    #[test]
    fn test_export_ofd_to_png() {
        let mut ofd_node = read_ofd("./learning/test.ofd").unwrap();
        println!("ofd: {:?}", ofd_node);
        export_ofd_to_png(&mut ofd_node, "target/out.png").unwrap();
    }
}
