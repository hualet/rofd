mod support;

use rofd_core::{AnnotationType, Document, LoadOptions, PageObject, WarningCode};
use support::ofd_with_document_page_and_entries;

const DOCUMENT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData>
  <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
</ofd:CommonData><ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
<ofd:Annotations>Annotations.xml</ofd:Annotations></ofd:Document>"#;

const PAGE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"/></ofd:Content></ofd:Page>"#;

const ENTRY_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Annotations xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Page PageID="900"><ofd:FileLoc>Annots/Page_0/Annotation.xml</ofd:FileLoc></ofd:Page><ofd:Page PageID="999"><ofd:FileLoc>Annots/missing.xml</ofd:FileLoc></ofd:Page></ofd:Annotations>"#;

const ANNOT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:PageAnnot xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Annot Type="Stamp" ID="13133"><ofd:Appearance Boundary="0 0 210 297"><ofd:PathObject ID="13135" Boundary="10 10 50 8" Fill="true"><ofd:FillColor Value="170 160 165"/><ofd:AbbreviatedData>M 0 0 L 50 0 L 50 8 L 0 8 C</ofd:AbbreviatedData></ofd:PathObject><ofd:TextObject ID="13136" Boundary="20 20 80 8" Font="10" Size="8"><ofd:FillColor Value="170 160 165"/><ofd:TextCode X="0" Y="8">保密资料</ofd:TextCode></ofd:TextObject></ofd:Appearance></ofd:Annot><ofd:Annot Type="Highlight" ID="13140" Visible="false"><ofd:Appearance Boundary="0 0 10 10"><ofd:PathObject ID="13141" Boundary="0 0 10 10" Fill="true"><ofd:FillColor Value="255 255 0"/><ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData></ofd:PathObject></ofd:Appearance></ofd:Annot></ofd:PageAnnot>"#;

const FONT_CATALOG: &str = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Fixture"/></ofd:Fonts></ofd:Res>"#;

fn annotated_document() -> Document {
    let document_xml = DOCUMENT_XML.replace(
        "<ofd:PageArea>",
        "<ofd:PublicRes>Res.xml</ofd:PublicRes><ofd:PageArea>",
    );
    let bytes = ofd_with_document_page_and_entries(
        &document_xml,
        PAGE_XML,
        &[
            ("Doc_0/Annotations.xml", ENTRY_XML.as_bytes()),
            ("Doc_0/Annots/Page_0/Annotation.xml", ANNOT_XML.as_bytes()),
            ("Doc_0/Res.xml", FONT_CATALOG.as_bytes()),
        ],
    );
    Document::from_bytes(bytes, LoadOptions::default()).unwrap()
}

#[test]
fn page_annotations_expose_type_visibility_and_appearance_objects() {
    let document = annotated_document();
    let page = document.page(0).unwrap();
    let annotations = page.annotations();
    assert_eq!(annotations.len(), 2);

    let stamp = &annotations[0];
    assert_eq!(stamp.object_id(), 13133);
    assert_eq!(stamp.kind(), AnnotationType::Stamp);
    assert!(stamp.visible());
    assert_eq!(stamp.boundary().width, 210.0);
    assert_eq!(stamp.objects().len(), 2);
    assert!(matches!(&stamp.objects()[0], PageObject::Path(path) if path.object_id() == 13135));
    assert!(matches!(&stamp.objects()[1], PageObject::Text(text) if text.object_id() == 13136));

    let highlight = &annotations[1];
    assert_eq!(highlight.kind(), AnnotationType::Highlight);
    assert!(!highlight.visible());
    assert_eq!(highlight.objects().len(), 1);
}

#[test]
fn missing_annotation_files_are_skipped_with_a_warning() {
    let document = annotated_document();
    let page = document.page(0).unwrap();
    // Annotation loading is lazy on first access, like signature stamps.
    assert_eq!(page.annotations().len(), 2);
    let warnings = document.warnings();
    assert!(
        warnings
            .iter()
            .any(|warning| warning.code == WarningCode::AnnotationSkipped
                && warning.message.contains("missing.xml")),
        "{warnings:?}"
    );
    // Page 999's missing file yields no annotations; page 900 keeps both.
    let page = document.page(0).unwrap();
    assert_eq!(page.annotations().len(), 2);
}

#[test]
fn documents_without_annotations_expose_none() {
    let bytes = support::minimal_ofd(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"/></ofd:Content></ofd:Page>"#,
    );
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    assert!(page.annotations().is_empty());
}
