//! Stamp annotation parsing from signature files (Signatures.xml,
//! Signature.xml, and DER-encoded SignedValue.dat).

use std::io::{Cursor, Write};

use rofd_core::{Document, LoadOptions, Rect, SealPictureKind, WarningCode};
use zip::{write::SimpleFileOptions, ZipWriter};

fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    if content.len() < 0x80 {
        out.push(content.len() as u8);
    } else {
        let octets = content.len().to_be_bytes();
        let significant = &octets[octets.iter().position(|byte| *byte != 0).unwrap()..];
        out.push(0x80 | significant.len() as u8);
        out.extend_from_slice(significant);
    }
    out.extend_from_slice(content);
    out
}

fn sequence(children: &[Vec<u8>]) -> Vec<u8> {
    der(0x30, &children.concat())
}

/// Builds a minimal SES_Signature holding one picture.
fn signed_value(kind: &str, data: &[u8]) -> Vec<u8> {
    let picture = sequence(&[
        der(0x16, kind.as_bytes()),
        der(0x04, data),
        der(0x02, &[30]),
        der(0x02, &[20]),
    ]);
    let eseal = sequence(&[
        sequence(&[der(0x16, b"ES"), der(0x02, &[4])]),
        der(0x16, b"seal-id"),
        sequence(&[der(0x02, &[3])]),
        picture,
        sequence(&[]),
    ]);
    let to_sign = sequence(&[der(0x02, &[1]), eseal]);
    sequence(&[to_sign, der(0x04, b"cert"), der(0x06, &[0x2a])])
}

const PAGE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;

const DOCUMENT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="10" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#;

fn package(entries: &[(&str, &[u8])], with_signatures_declaration: bool) -> Vec<u8> {
    let signatures = if with_signatures_declaration {
        "  <ofd:Signatures>/Doc_0/Signs/Signatures.xml</ofd:Signatures>\n"
    } else {
        ""
    };
    let ofd_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">
  <ofd:DocBody>
    <ofd:DocInfo><ofd:DocID>signature-fixture</ofd:DocID></ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
{signatures}  </ofd:DocBody>
</ofd:OFD>"#
    );
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in [
        ("OFD.xml", ofd_xml.as_bytes()),
        ("Doc_0/Document.xml", DOCUMENT_XML.as_bytes()),
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

const SIGNATURES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Signatures xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:MaxSignId>2</ofd:MaxSignId>
  <ofd:Signature ID="1" BaseLoc="Sign_0/Signature.xml"/>
</ofd:Signatures>"#;

fn signature_xml(stamp_annot: &str, signed_value: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Signature xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:SignedInfo>
    <ofd:Provider ProviderName="test" Company="test" Version="1.0"/>
    <ofd:SignatureMethod>1.2.156.10197.1.501</ofd:SignatureMethod>
    <ofd:SignatureDateTime>20200817111329Z</ofd:SignatureDateTime>
    {stamp_annot}
  </ofd:SignedInfo>
  <ofd:SignedValue>{signed_value}</ofd:SignedValue>
</ofd:Signature>"#
    )
}

fn open(bytes: Vec<u8>) -> Document {
    Document::from_bytes(bytes, LoadOptions::default()).unwrap()
}

#[test]
fn document_without_signatures_returns_no_stamp_annotations() {
    let document = open(package(&[], false));
    assert!(document.stamp_annotations().unwrap().is_empty());
    assert!(document.page(0).unwrap().stamp_annotations().is_empty());
    assert!(document
        .warnings()
        .iter()
        .all(|warning| warning.code != WarningCode::SignatureSkipped));
}

#[test]
fn stamp_annotations_are_parsed_from_signature_files() {
    let signature = signature_xml(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="90.0 8.0 30.0 20.0"/>"#,
        "/Doc_0/Signs/Sign_0/SignedValue.dat",
    );
    let document = open(package(
        &[
            ("Doc_0/Signs/Signatures.xml", SIGNATURES_XML.as_bytes()),
            ("Doc_0/Signs/Sign_0/Signature.xml", signature.as_bytes()),
            (
                "Doc_0/Signs/Sign_0/SignedValue.dat",
                &signed_value("png", b"\x89PNG\r\n\x1a\nfake"),
            ),
        ],
        true,
    ));

    let annotations = document.stamp_annotations().unwrap();
    assert_eq!(annotations.len(), 1);
    let annotation = &annotations[0];
    assert_eq!(annotation.page_ref, 10);
    assert_eq!(annotation.id.as_deref(), Some("01"));
    assert_eq!(
        annotation.boundary,
        Rect {
            x: 90.0,
            y: 8.0,
            width: 30.0,
            height: 20.0,
        }
    );
    assert_eq!(annotation.clip, None);
    assert_eq!(annotation.picture.kind, SealPictureKind::Png);
    assert_eq!(annotation.picture.data, b"\x89PNG\r\n\x1a\nfake");
    assert_eq!(annotation.picture.width_mm, Some(30.0));
    assert_eq!(annotation.picture.height_mm, Some(20.0));

    let page = document.page(0).unwrap();
    assert_eq!(page.object_id(), 10);
    assert_eq!(page.stamp_annotations().len(), 1);
    assert!(document
        .warnings()
        .iter()
        .all(|warning| warning.code != WarningCode::SignatureSkipped));
}

#[test]
fn stamp_annot_clip_and_page_ref_filtering() {
    let signature = signature_xml(
        r#"<ofd:StampAnnot ID="s002" Boundary="202 40 40 40" PageRef="999" Clip="0 0 8 40"/>"#,
        "SignedValue.dat",
    );
    let document = open(package(
        &[
            ("Doc_0/Signs/Signatures.xml", SIGNATURES_XML.as_bytes()),
            ("Doc_0/Signs/Sign_0/Signature.xml", signature.as_bytes()),
            (
                "Doc_0/Signs/Sign_0/SignedValue.dat",
                &signed_value("PNG", b"fake-png"),
            ),
        ],
        true,
    ));

    let annotations = document.stamp_annotations().unwrap();
    assert_eq!(annotations.len(), 1);
    assert_eq!(
        annotations[0].clip,
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 40.0,
        })
    );
    // The picture type name is matched case-insensitively.
    assert_eq!(annotations[0].picture.kind, SealPictureKind::Png);
    // PageRef 999 matches no page, so the page sees nothing.
    assert!(document.page(0).unwrap().stamp_annotations().is_empty());
}

#[test]
fn missing_signatures_file_degrades_to_empty_with_warning() {
    let document = open(package(&[], true));
    assert!(document.stamp_annotations().unwrap().is_empty());
    assert!(document.page(0).unwrap().stamp_annotations().is_empty());
    let warnings = document.warnings();
    assert!(warnings
        .iter()
        .any(|warning| warning.code == WarningCode::SignatureSkipped));
}

#[test]
fn broken_signature_is_skipped_with_warning() {
    let signature = signature_xml(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="90.0 8.0 30.0 20.0"/>"#,
        "/Doc_0/Signs/Sign_0/SignedValue.dat",
    );
    let document = open(package(
        &[
            ("Doc_0/Signs/Signatures.xml", SIGNATURES_XML.as_bytes()),
            ("Doc_0/Signs/Sign_0/Signature.xml", signature.as_bytes()),
            ("Doc_0/Signs/Sign_0/SignedValue.dat", b"not DER at all"),
        ],
        true,
    ));
    assert!(document.stamp_annotations().unwrap().is_empty());
    let warnings = document.warnings();
    assert!(warnings
        .iter()
        .any(|warning| warning.code == WarningCode::SignatureSkipped
            && warning.message.contains("signature 1")));
}

#[test]
fn invalid_stamp_annot_boundary_skips_only_that_annotation() {
    let signature = signature_xml(
        r#"<ofd:StampAnnot PageRef="10" ID="bad" Boundary="not a box"/>
        <ofd:StampAnnot PageRef="10" ID="good" Boundary="1 2 3 4"/>"#,
        "/Doc_0/Signs/Sign_0/SignedValue.dat",
    );
    let document = open(package(
        &[
            ("Doc_0/Signs/Signatures.xml", SIGNATURES_XML.as_bytes()),
            ("Doc_0/Signs/Sign_0/Signature.xml", signature.as_bytes()),
            (
                "Doc_0/Signs/Sign_0/SignedValue.dat",
                &signed_value("ofd", b"PK\x03\x04"),
            ),
        ],
        true,
    ));
    let annotations = document.stamp_annotations().unwrap();
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].id.as_deref(), Some("good"));
    assert_eq!(annotations[0].picture.kind, SealPictureKind::Ofd);
    assert!(document
        .warnings()
        .iter()
        .any(|warning| warning.code == WarningCode::SignatureSkipped));
}
