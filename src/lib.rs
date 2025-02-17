#![allow(dead_code)]

mod document;
mod ofd;
mod page;
mod render;
mod types;
mod elements;

use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use log::debug;
use zip::ZipArchive;

use document::{Document, DocumentRes, PublicRes, Annotations, PageAnnot};
use ofd::{Ofd, OfdNode};
use page::Page;
use render::Renderable;
// use types::ct;

pub enum TargetDevice {
    CairoContext(cairo::Context),
    QPainter(qmetaobject::QPainter),
}

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

pub fn render_ofd(ofd: &mut Ofd, surface: &mut TargetDevice) -> Result<(), Box<dyn Error>> {
    // create a new String to store the content of the DocRoot file.
    let mut content = String::new();

    // Find the DocRoot file and parse the content to a document object.
    {
        let doc_root_path = ofd.node.doc_body.doc_root.as_str();
        let mut doc_file = ofd.zip_archive.by_name(doc_root_path)?;
        doc_file.read_to_string(&mut content)?;
    }

    let mut document = Document::from_xml(&content)?;
    debug!("document: {:#?}", document);

    // Find the DocumentRes.xml file and parse the content to a DocumentRes object.
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

    let document_res = DocumentRes::from_xml(&content)?;
    debug!("document_res: {:#?}", document_res);
    document.doc_res = document_res;

    // Find the PublicRes.xml file and parse the content to a PublicRes object.
    {
        let doc_root_path = ofd.node.doc_body.doc_root.as_str();
        let root_path = Path::new(doc_root_path);
        let res_path = root_path
                        .parent().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent directory not found"))?
                        .join(document.common_data.public_res.as_str());
        let mut public_res_file = ofd.zip_archive.by_name(res_path.to_str().unwrap())?;

        content.clear();
        public_res_file.read_to_string(&mut content)?;
    }

    let public_res = PublicRes::from_xml(&content)?;
    debug!("public_res: {:#?}", public_res);
    document.public_res = public_res;

    // Find the Annotations.xml file and parse the content to a Annotations object.
    {
        let doc_root_path = ofd.node.doc_body.doc_root.as_str();
        let root_path = Path::new(doc_root_path);
        let annots_path = root_path
                        .parent().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent directory not found"))?
                        .join(document.annotations.clone().unwrap());
        let mut annotations_file = ofd.zip_archive.by_name(annots_path.to_str().unwrap())?;

        content.clear();
        annotations_file.read_to_string(&mut content)?;
    }

    let annotations = Annotations::from_xml(&content)?;
    debug!("annotations: {:#?}", annotations);

    {
        for page_annot in annotations.page.iter() {
            let doc_root_path = ofd.node.doc_body.doc_root.as_str();
            // let annots_root_path = Path::new(annots_path.as_str()).parent().unwrap();
            let root_path = Path::new(doc_root_path);
            let annot_path = root_path
                                .parent()
                                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent directory not found"))?
                                .join("Annots") // TODO(hualet): hardcoded
                                .join(page_annot.file_loc.as_str());
            let mut annot_file = ofd.zip_archive.by_name(annot_path.to_str().unwrap())?;
            content.clear();
            annot_file.read_to_string(&mut content)?;
            let page_annot = PageAnnot::from_xml(&content)?;
            debug!("page_annot: {:#?}", page_annot);
        }
    }

    // let _pybox = ct::PageArea::from(document.common_data.page_area.physical_box.clone()).to_pixel();

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

        match surface {
            TargetDevice::CairoContext(context) => {
                page.render_to_cairo_context(context, ofd, &document)?;
            },
            TargetDevice::QPainter(qpainter) => {
                page.render_to_qpainter(qpainter, ofd, &document);
            },
        }
    }

    Ok(())
}

pub fn export_ofd_to_png(ofd: &mut Ofd, output_path: &str, width: u32, height: u32) -> Result<(), Box<dyn Error>> {
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        width as i32,
        height as i32,
    )?;

    let context = cairo::Context::new(&surface)?;
    render_ofd(ofd, &mut TargetDevice::CairoContext(context))?;

    let mut file = File::create(output_path)?;
    surface.write_to_png(&mut file)?;

    Ok(())
}