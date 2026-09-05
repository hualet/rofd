mod support;

use rofd_core::{Document, Error, ImageFormat, LoadOptions, PageObject, ResourceLimits, Transform};

fn image_page(object: &str, catalog: &str) -> rofd_core::Result<rofd_core::Page> {
    let document_xml = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes></ofd:CommonData><ofd:Pages><ofd:Page ID="9" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
    let page_xml = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="1">{object}</ofd:Layer></ofd:Content></ofd:Page>"#
    );
    let bytes = support::ofd_with_document_page_and_entries(
        document_xml,
        &page_xml,
        &[("Doc_0/Res.xml", catalog.as_bytes())],
    );
    Document::from_bytes(
        bytes,
        LoadOptions {
            limits: ResourceLimits::default(),
            ..LoadOptions::default()
        },
    )?
    .page(0)
}

#[test]
fn exposes_validated_image_references_transform_alpha_clips_and_border_presence() {
    let document_xml = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes></ofd:CommonData><ofd:Pages><ofd:Page ID="9" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
    let page_xml = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"><ofd:ImageObject ID="2" Boundary="1 2 3 4" CTM="1 0 0 1 5 6" ResourceID="10" Alpha="127" Substitution="11" ImageMask="12"><ofd:Clips><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 1 1" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 1 0 L 1 1 C</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips><ofd:Border/></ofd:ImageObject></ofd:Layer></ofd:Content></ofd:Page>"#;
    let catalog = br#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>a</MediaFile></MultiMedia><MultiMedia ID="11" Type="Image" Format="JPEG"><MediaFile>b</MediaFile></MultiMedia><MultiMedia ID="12" Type="Image" Format="PNG"><MediaFile>c</MediaFile></MultiMedia></MultiMedias></Res>"#;
    let bytes = support::ofd_with_document_page_and_entries(
        document_xml,
        page_xml,
        &[("Doc_0/Res.xml", catalog.as_slice())],
    );
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let PageObject::Image(image) = &page.layers()[0].objects()[0] else {
        panic!("expected image");
    };
    assert_eq!(image.object_id(), 2);
    assert_eq!(image.resource_id(), 10);
    assert_eq!(image.resource_format(), ImageFormat::Png);
    assert_eq!(image.substitution_id(), Some(11));
    assert_eq!(image.image_mask_id(), Some(12));
    assert_eq!(image.alpha(), 127);
    assert_eq!(
        image.transform(),
        Transform::new(1.0, 0.0, 0.0, 1.0, 5.0, 6.0).unwrap()
    );
    assert_eq!(image.clips().len(), 1);
    assert!(image.has_border());
}

#[test]
fn self_closing_image_does_not_consume_the_following_graphic_unit() {
    let document_xml = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes></ofd:CommonData><ofd:Pages><ofd:Page ID="9" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
    let page_xml = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:ImageObject ID="3" Boundary="0 0 1 1" ResourceID="10"/></ofd:PageBlock><ofd:PathObject ID="4" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content></ofd:Page>"#;
    let catalog = br#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>a</MediaFile></MultiMedia></MultiMedias></Res>"#;
    let bytes = support::ofd_with_document_page_and_entries(
        document_xml,
        page_xml,
        &[("Doc_0/Res.xml", catalog.as_slice())],
    );
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    assert_eq!(page.layers()[0].objects().len(), 2);
}

#[test]
fn image_defaults_do_not_read_encoded_assets_during_page_validation() {
    let catalog = r#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="JPEG"><MediaFile>missing.jpg</MediaFile></MultiMedia></MultiMedias></Res>"#;
    let page = image_page(
        r#"<ofd:ImageObject ID="2" Boundary="0 0 1 1" ResourceID="10"/>"#,
        catalog,
    )
    .unwrap();
    let PageObject::Image(image) = &page.layers()[0].objects()[0] else {
        panic!()
    };
    assert_eq!(image.transform(), Transform::IDENTITY);
    assert_eq!(image.alpha(), 255);
    assert_eq!(image.resource_format(), ImageFormat::Jpeg);
    assert!(image.clips().is_empty());
    assert_eq!(image.substitution_id(), None);
    assert_eq!(image.image_mask_id(), None);
    assert!(!image.has_border());
}

#[test]
fn image_resource_references_are_nonzero_known_images_with_object_context() {
    let catalog = r#"<Res><Fonts><Font ID="20" FontName="Wrong"/></Fonts><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>a.png</MediaFile></MultiMedia></MultiMedias></Res>"#;
    for (attributes, field) in [
        ("ResourceID=\"0\"", "ResourceID"),
        ("ResourceID=\"99\"", "ResourceID"),
        ("ResourceID=\"20\"", "ResourceID"),
        ("ResourceID=\"10\" Substitution=\"20\"", "Substitution"),
        ("ResourceID=\"10\" ImageMask=\"99\"", "ImageMask"),
    ] {
        let object = format!(r#"<ofd:ImageObject ID="2" Boundary="0 0 1 1" {attributes}/>"#);
        let error = image_page(&object, catalog).unwrap_err();
        assert!(
            matches!(error, Error::InvalidPageObject { object_id: 2, field: actual, ref path, .. } if actual == field && path.ends_with("Content.xml")),
            "{error:?}"
        );
    }
}

#[test]
fn image_required_fields_and_alpha_have_object_context() {
    let catalog = r#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>a.png</MediaFile></MultiMedia></MultiMedias></Res>"#;
    for (object, expected_field) in [
        (r#"<ofd:ImageObject ID="2" ResourceID="10"/>"#, "Boundary"),
        (
            r#"<ofd:ImageObject ID="2" Boundary="0 0 1 1"/>"#,
            "ResourceID",
        ),
        (
            r#"<ofd:ImageObject ID="2" Boundary="0 0 1 1" ResourceID="10" Alpha="-1"/>"#,
            "Alpha",
        ),
    ] {
        let error = image_page(object, catalog).unwrap_err();
        assert!(
            matches!(error, Error::InvalidPageObject { object_id: 2, field, ref path, .. } if field == expected_field && path.ends_with("Content.xml")),
            "expected {expected_field}, got {error:?}"
        );
    }
}

#[test]
fn malformed_image_object_does_not_consume_following_siblings() {
    let catalog = r#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>a.png</MediaFile></MultiMedia></MultiMedias></Res>"#;
    let objects = r#"<ofd:ImageObject ID="2" ResourceID="10"/><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>"#;
    let error = image_page(objects, catalog).unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidPageObject {
            object_id: 2,
            field: "Boundary",
            ..
        }
    ));
}

#[test]
fn unknown_image_children_fail_closed_while_border_is_retained() {
    let catalog = r#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>a.png</MediaFile></MultiMedia></MultiMedias></Res>"#;
    let object = r#"<ofd:ImageObject ID="2" Boundary="0 0 1 1" ResourceID="10"><ofd:Unknown/></ofd:ImageObject>"#;
    assert!(
        matches!(image_page(object, catalog), Err(Error::InvalidStructure { message, .. }) if message.contains("Unknown"))
    );
    let nested_border = r#"<ofd:ImageObject ID="2" Boundary="0 0 1 1" ResourceID="10"><ofd:Border><ofd:Unknown/></ofd:Border></ofd:ImageObject>"#;
    assert!(
        matches!(image_page(nested_border, catalog), Err(Error::InvalidStructure { message, .. }) if message.contains("Border"))
    );
}
