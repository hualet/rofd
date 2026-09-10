//! Stamp annotation (electronic seal) rendering smoke tests.

use std::io::{Cursor, Write};

use cairo::{Context, Format, ImageSurface};
use rofd_core::{Document, LoadOptions};
use rofd_render::{CairoRenderer, ImageInterpolation, PixelRect, RenderOptions};
use zip::{write::SimpleFileOptions, ZipWriter};

/// A 3x2 RGBA PNG with six distinct pixels.
const PNG: &[u8] = include_bytes!("fixtures/images/asymmetric-rgba.png");

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

fn signed_value(kind: &str, data: &[u8]) -> Vec<u8> {
    let picture = sequence(&[
        der(0x16, kind.as_bytes()),
        der(0x04, data),
        der(0x02, &[30]),
        der(0x02, &[20]),
    ]);
    let eseal = sequence(&[sequence(&[der(0x16, b"ES")]), picture]);
    sequence(&[sequence(&[der(0x02, &[1]), eseal])])
}

fn mini_ofd() -> Vec<u8> {
    mini_ofd_with_overlay("")
}

fn mini_ofd_with_overlay(overlay: &str) -> Vec<u8> {
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 30 20</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 30 20" Fill="true" Stroke="false"><ofd:FillColor Value="200 32 38"/><ofd:AbbreviatedData>M 0 0 L 30 0 L 30 20 L 0 20 C</ofd:AbbreviatedData></ofd:PathObject>{overlay}</ofd:Layer></ofd:Content></ofd:Page>"#
    );
    let files: [(&str, &[u8]); 3] = [
        (
            "OFD.xml",
            br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>seal</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#,
        ),
        (
            "Doc_0/Document.xml",
            br#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 30 20</ofd:PhysicalBox></ofd:PageArea></ofd:CommonData><ofd:Pages><ofd:Page ID="1" BaseLoc="Page.xml"/></ofd:Pages></ofd:Document>"#,
        ),
        (
            "Doc_0/Page.xml",
            page.as_bytes(),
        ),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in files {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn package(kind: &str, picture: &[u8], stamp_annot: &str) -> Vec<u8> {
    let signature_xml = format!(
        r#"<ofd:Signature xmlns:ofd="http://www.ofdspec.org/2016"><ofd:SignedInfo>{stamp_annot}</ofd:SignedInfo><ofd:SignedValue>/Doc_0/Signs/Sign_0/SignedValue.dat</ofd:SignedValue></ofd:Signature>"#
    );
    let value = signed_value(kind, picture);
    let files: [(&str, &[u8]); 7] = [
        (
            "OFD.xml",
            br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>cairo-stamp</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot><ofd:Signatures>Doc_0/Signs/Signatures.xml</ofd:Signatures></ofd:DocBody></ofd:OFD>"#,
        ),
        (
            "Doc_0/Document.xml",
            br#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea></ofd:CommonData><ofd:Pages><ofd:Page ID="10" BaseLoc="Page.xml"/></ofd:Pages></ofd:Document>"#,
        ),
        (
            "Doc_0/Page.xml",
            br#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"/></ofd:Content></ofd:Page>"#,
        ),
        (
            "Doc_0/Signs/Signatures.xml",
            br#"<ofd:Signatures xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Signature ID="1" BaseLoc="Sign_0/Signature.xml"/></ofd:Signatures>"#,
        ),
        ("Doc_0/Signs/Sign_0/Signature.xml", signature_xml.as_bytes()),
        ("Doc_0/Signs/Sign_0/SignedValue.dat", &value),
        ("Doc_0/Signs/Sign_0/picture.bin", picture),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in files {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn render(stamp_annot: &str, kind: &str, picture: &[u8]) -> ImageSurface {
    let document =
        Document::from_bytes(package(kind, picture, stamp_annot), LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let options = RenderOptions {
        dpi: 25.4,
        image_interpolation: ImageInterpolation::Nearest,
        ..RenderOptions::default()
    };
    let (width, height) = CairoRenderer::pixel_size(&page, &options).unwrap();
    assert_eq!((width, height), (20, 20));
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface).unwrap();
    CairoRenderer
        .render_page(&page, &context, &options)
        .unwrap();
    drop(context);
    surface
}

fn pixel(surface: &mut ImageSurface, x: i32, y: i32) -> [u8; 4] {
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().unwrap();
    let offset = y as usize * stride + x as usize * 4;
    let native = u32::from_ne_bytes(data[offset..offset + 4].try_into().unwrap());
    [
        ((native >> 16) & 0xff) as u8,
        ((native >> 8) & 0xff) as u8,
        (native & 0xff) as u8,
        ((native >> 24) & 0xff) as u8,
    ]
}

#[test]
fn png_stamp_is_stretched_into_its_boundary() {
    let mut surface = render(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="2 2 6 4"/>"#,
        "png",
        PNG,
    );

    // The 3x2 source pixels each cover 2x2 device pixels at 1 px/mm; the
    // source green pixel is half-transparent and blends over the white page.
    assert_eq!(pixel(&mut surface, 2, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(&mut surface, 4, 2), [127, 255, 127, 255]);
    assert_eq!(pixel(&mut surface, 6, 2), [0, 0, 255, 255]);
    assert_eq!(pixel(&mut surface, 2, 4), [255, 255, 0, 255]);
    assert_eq!(pixel(&mut surface, 4, 4), [255, 0, 255, 255]);
    assert_eq!(pixel(&mut surface, 6, 4), [0, 255, 255, 255]);
    // Outside the boundary the page background is untouched.
    assert_eq!(pixel(&mut surface, 0, 0), [255, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 8, 2), [255, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 2, 6), [255, 255, 255, 255]);
}

#[test]
fn stamp_clip_restricts_the_painted_area() {
    let mut surface = render(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="2 2 6 4" Clip="0 0 3 4"/>"#,
        "png",
        PNG,
    );

    assert_eq!(pixel(&mut surface, 2, 2), [255, 0, 0, 255]);
    // The right half of the boundary is clipped away.
    assert_eq!(pixel(&mut surface, 6, 2), [255, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 6, 4), [255, 255, 255, 255]);
}

#[test]
fn ofd_stamp_renders_the_mini_document_page() {
    let mut surface = render(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="5 5 9 6"/>"#,
        "ofd",
        &mini_ofd(),
    );

    // The mini document paints an opaque red rectangle over its full area.
    let [red, green, blue, alpha] = pixel(&mut surface, 9, 8);
    assert!(red > 150 && green < 80 && blue < 80 && alpha == 255);
    // The page outside the stamp stays untouched.
    assert_eq!(pixel(&mut surface, 0, 0), [255, 255, 255, 255]);
    assert_eq!(pixel(&mut surface, 14, 8), [255, 255, 255, 255]);
}

#[test]
fn broken_or_unsupported_stamps_are_skipped_silently() {
    let mut surface = render(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="2 2 6 4"/>"#,
        "tif",
        b"not a real image",
    );
    assert_eq!(pixel(&mut surface, 2, 2), [255, 255, 255, 255]);

    let mut surface = render(
        r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="2 2 6 4"/>"#,
        "png",
        b"not a real image",
    );
    assert_eq!(pixel(&mut surface, 2, 2), [255, 255, 255, 255]);
}

#[test]
fn raster_and_ofd_stamp_regions_match_the_full_rotated_canvas() {
    let mini = mini_ofd_with_overlay(
        r#"<ofd:PathObject ID="3" Boundary="13.7 4.3 9.6 7.7" Fill="true" Stroke="false"><ofd:FillColor Value="10 30 220"/><ofd:AbbreviatedData>M 0 0 L 9.6 0 L 9.6 7.7 L 0 7.7 C</ofd:AbbreviatedData></ofd:PathObject>"#,
    );
    for (kind, picture) in [("png", PNG), ("ofd", mini.as_slice())] {
        let document = Document::from_bytes(package(kind, picture, r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="2.7 3.1 9.3 6.7" Clip="0.4 0.3 7.8 5.9"/>"#), LoadOptions::default()).unwrap();
        let page = document.page(0).unwrap();
        for rotation in [0, 90, 180, 270] {
            let options = RenderOptions {
                dpi: 43.7,
                scale: 1.17,
                rotation_degrees: rotation,
                ..RenderOptions::default()
            };
            let (width, height) = CairoRenderer::pixel_size(&page, &options).unwrap();
            let mut full = ImageSurface::create(Format::ARgb32, width, height).unwrap();
            let context = Context::new(&full).unwrap();
            CairoRenderer
                .render_page(&page, &context, &options)
                .unwrap();
            drop(context);
            for y in (0..height).step_by(7) {
                for x in (0..width).step_by(11) {
                    let viewport = PixelRect {
                        x,
                        y,
                        width: 11.min(width - x),
                        height: 7.min(height - y),
                    };
                    let mut tile =
                        ImageSurface::create(Format::ARgb32, viewport.width, viewport.height)
                            .unwrap();
                    let context = Context::new(&tile).unwrap();
                    CairoRenderer
                        .render_page_region(&page, &context, &options, viewport)
                        .unwrap();
                    drop(context);
                    for ty in 0..viewport.height {
                        for tx in 0..viewport.width {
                            assert_eq!(
                                pixel(&mut tile, tx, ty),
                                pixel(&mut full, x + tx, y + ty),
                                "{kind} rotation {rotation}, {viewport:?}, pixel {tx},{ty}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn high_zoom_ofd_stamp_renders_only_the_visible_source_region() {
    let document = Document::from_bytes(
        package(
            "ofd",
            &mini_ofd(),
            r#"<ofd:StampAnnot PageRef="10" ID="01" Boundary="5 5 9 6"/>"#,
        ),
        LoadOptions::default(),
    )
    .unwrap();
    let page = document.page(0).unwrap();
    for (rotation, x, y) in [
        (0, 18000, 16000),
        (90, 24000, 18000),
        (180, 22000, 24000),
        (270, 16000, 22000),
    ] {
        let options = RenderOptions {
            dpi: 25.4,
            scale: 2000.0,
            rotation_degrees: rotation,
            max_raster_bytes: 20000,
            ..RenderOptions::default()
        };
        assert_eq!(
            CairoRenderer::pixel_canvas_size(&page, &options).unwrap(),
            (40000, 40000)
        );
        // The full mini-OFD source would require a 60000x40000 raster, which
        // exceeds both Cairo's dimension limit and the available byte budget.
        let mut tile = ImageSurface::create(Format::ARgb32, 10, 8).unwrap();
        let context = Context::new(&tile).unwrap();
        CairoRenderer
            .render_page_region(
                &page,
                &context,
                &options,
                PixelRect {
                    x,
                    y,
                    width: 10,
                    height: 8,
                },
            )
            .unwrap();
        drop(context);
        assert_eq!(
            pixel(&mut tile, 5, 4),
            [200, 32, 38, 255],
            "rotation {rotation}"
        );
    }
}
