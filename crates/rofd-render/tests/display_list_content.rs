use std::io::{Cursor, Write};

use fontdb::Database;
use rofd_core::{
    Color, Document, LayerSource, LineCap, LineJoin, LoadOptions, Point, ResourceKind,
};
use rofd_render::{
    Command, DisplayList, DisplayListBuilder, Error, FontResolver, ImageDecoder,
    RenderDiagnosticKind, ResolvedFont, SystemFontResolver,
};
use zip::{write::SimpleFileOptions, ZipWriter};

const FONT: &[u8] = include_bytes!("fixtures/fonts/phase3-subset.ttf");
const PNG: &[u8] = include_bytes!("fixtures/images/asymmetric-rgba.png");
const MASK_PNG: &[u8] = include_bytes!("fixtures/images/checker-mask.png");

fn document(page: &str, resources: &str, assets: &[(&str, &[u8])]) -> Document {
    let files = [
        (
            "OFD.xml",
            br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>display-content</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#.as_slice(),
        ),
        (
            "Doc_0/Document.xml",
            br#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes></ofd:CommonData><ofd:Pages><ofd:Page ID="1" BaseLoc="Page.xml"/></ofd:Pages></ofd:Document>"#.as_slice(),
        ),
        ("Doc_0/Res.xml", resources.as_bytes()),
        ("Doc_0/Page.xml", page.as_bytes()),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in files
        .into_iter()
        .chain(assets.iter().map(|(path, bytes)| (*path, *bytes)))
    {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    Document::from_bytes(
        writer.finish().unwrap().into_inner(),
        LoadOptions::default(),
    )
    .unwrap()
}

fn resources(image_file: &str) -> String {
    format!(
        r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Fixture"><ofd:FontFile>font.ttf</ofd:FontFile></ofd:Font></ofd:Fonts><ofd:MultiMedias><ofd:MultiMedia ID="20" Type="Image" Format="PNG"><ofd:MediaFile>{image_file}</ofd:MediaFile></ofd:MultiMedia><ofd:MultiMedia ID="21" Type="Image" Format="PNG"><ofd:MediaFile>image.png</ofd:MediaFile></ofd:MultiMedia></ofd:MultiMedias></ofd:Res>"#
    )
}

fn builder<'a>(
    resolver: &'a SystemFontResolver,
    decoder: &'a ImageDecoder,
) -> DisplayListBuilder<'a> {
    DisplayListBuilder::new(resolver, decoder)
}

#[test]
fn injected_builder_lowers_path_text_and_image_with_exact_state_order() {
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:PathObject ID="3" Boundary="1 2 10 10" Stroke="true" Fill="true" LineWidth="1.25" Join="Round" Cap="Square" DashOffset="0.5" DashPattern="1 2" MiterLimit="4"><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject><ofd:PageBlock ID="4"><ofd:TextObject ID="5" Boundary="10 20 30 10" CTM="1 0.5 0 1 2 3" Font="10" Size="4" Stroke="true" Fill="true" Alpha="128" LineWidth="0.8" Join="Bevel" Cap="Round" DashOffset="1" DashPattern="2 3" MiterLimit="5"><ofd:Clips><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 3 3" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 3 0</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips><ofd:FillColor Value="10 20 30" Alpha="128"/><ofd:TextCode X="1" Y="2">A中</ofd:TextCode></ofd:TextObject><ofd:ImageObject ID="6" Boundary="40 50 6 4" CTM="1 0 0.25 1 2 3" ResourceID="20" Alpha="128"><ofd:Clips><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 2 2" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 2 0</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips></ofd:ImageObject></ofd:PageBlock></ofd:Layer></ofd:Content></ofd:Page>"#;
    let text_document = document(
        page,
        &resources("image.png"),
        &[("Doc_0/font.ttf", FONT), ("Doc_0/image.png", PNG)],
    );
    let page = text_document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();

    let display = builder(&resolver, &decoder).build(&page).unwrap();
    let commands = display.commands();
    assert!(display.diagnostics().is_empty());
    let command_order = commands
        .iter()
        .map(|command| match command {
            Command::Save => "save",
            Command::ConcatTransform(_) => "transform",
            Command::ClipPath { .. } => "clip",
            Command::SetStroke(_) => "stroke",
            Command::SetFill(_) => "fill",
            Command::SetFillRule(_) => "fill-rule",
            Command::SetLineWidth(_) => "line-width",
            Command::SetLineJoin(_) => "line-join",
            Command::SetLineCap(_) => "line-cap",
            Command::SetMiterLimit(_) => "miter-limit",
            Command::SetDash { .. } => "dash",
            Command::SetAlpha(_) => "alpha",
            Command::DrawPath(_) => "path",
            Command::DrawGlyphRun(_) => "glyphs",
            Command::DrawImage { .. } => "image",
            Command::Restore => "restore",
            _ => "future-command",
        })
        .collect::<Vec<_>>();
    assert_eq!(
        command_order,
        [
            "save",
            "transform",
            "stroke",
            "fill",
            "fill-rule",
            "line-width",
            "line-join",
            "line-cap",
            "miter-limit",
            "dash",
            "path",
            "restore",
            "save",
            "clip",
            "transform",
            "stroke",
            "fill",
            "line-width",
            "line-join",
            "line-cap",
            "miter-limit",
            "dash",
            "glyphs",
            "restore",
            "save",
            "clip",
            "transform",
            "alpha",
            "image",
            "restore",
        ]
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Save))
            .count(),
        3
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Restore))
            .count(),
        3
    );
    let path_draw = commands
        .iter()
        .position(|command| matches!(command, Command::DrawPath(_)))
        .unwrap();
    let glyph_draw = commands
        .iter()
        .position(|command| matches!(command, Command::DrawGlyphRun(_)))
        .unwrap();
    let image_draw = commands
        .iter()
        .position(|command| matches!(command, Command::DrawImage { .. }))
        .unwrap();
    assert!(path_draw < glyph_draw && glyph_draw < image_draw);
    assert!(commands[..path_draw].contains(&Command::SetLineJoin(LineJoin::Round)));
    assert!(commands[..path_draw].contains(&Command::SetLineCap(LineCap::Square)));
    assert!(commands[..path_draw].contains(&Command::SetMiterLimit(4.0)));
    assert!(commands[..path_draw].contains(&Command::SetDash {
        offset: 0.5,
        pattern: vec![1.0, 2.0],
    }));
    assert!(commands[path_draw..glyph_draw].contains(&Command::SetLineJoin(LineJoin::Bevel)));
    assert!(commands[path_draw..glyph_draw].contains(&Command::SetLineCap(LineCap::Round)));
    assert!(commands[path_draw..glyph_draw]
        .iter()
        .any(|command| matches!(command, Command::ClipPath { .. })));
    assert!(
        commands[path_draw..glyph_draw].contains(&Command::SetFill(Some(Color {
            red: 10,
            green: 20,
            blue: 30,
            alpha: 64,
        })))
    );
    let text_transform = commands[path_draw..glyph_draw]
        .iter()
        .find_map(|command| match command {
            Command::ConcatTransform(transform) => Some(*transform),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        text_transform.apply(Point::new(1.0, 2.0).unwrap()).unwrap(),
        Point::new(13.0, 25.5).unwrap()
    );
    let Command::DrawGlyphRun(run) = &commands[glyph_draw] else {
        unreachable!()
    };
    assert_eq!(run.text(), "A中");
    assert_eq!(run.glyphs().len(), 2);
    assert_eq!(commands[image_draw - 1], Command::SetAlpha(128));
    assert!(commands[glyph_draw..image_draw]
        .iter()
        .any(|command| matches!(command, Command::ClipPath { .. })));
    let image_transform = commands[glyph_draw..image_draw]
        .iter()
        .find_map(|command| match command {
            Command::ConcatTransform(transform) => Some(*transform),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        image_transform
            .apply(Point::new(0.0, 0.0).unwrap())
            .unwrap(),
        Point::new(42.0, 53.0).unwrap()
    );
    let Command::DrawImage {
        image,
        width,
        height,
    } = &commands[image_draw]
    else {
        unreachable!()
    };
    assert_eq!(image.dimensions(), (3, 2));
    assert_eq!((*width, *height), (1.0, 1.0));

    let rebuilt = builder(&resolver, &decoder).build(&page).unwrap();
    assert_eq!(display, rebuilt);
    assert_eq!(display, display.clone());

    let hermetic_default = DisplayList::from_page(&page).unwrap();
    assert!(hermetic_default
        .commands()
        .iter()
        .any(|command| matches!(command, Command::DrawGlyphRun(_))));
    assert!(hermetic_default
        .commands()
        .iter()
        .any(|command| matches!(command, Command::DrawImage { .. })));
}

#[test]
fn configured_font_fallback_is_promoted_to_a_source_aware_render_diagnostic() {
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:TextObject ID="5" Boundary="0 0 20 10" Font="10" Size="4"><ofd:TextCode X="0" Y="0">中</ofd:TextCode></ofd:TextObject></ofd:Layer></ofd:Content></ofd:Page>"#;
    let resources = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Unavailable"/></ofd:Fonts></ofd:Res>"#;
    let document = document(page, resources, &[]);
    let page = document.page(0).unwrap();
    let mut database = Database::new();
    database.load_font_data(FONT.to_vec());
    let resolver =
        SystemFontResolver::from_database(database, vec!["Noto Sans CJK SC".to_owned()], 1 << 20);
    let decoder = ImageDecoder::default();

    let display = builder(&resolver, &decoder).build(&page).unwrap();
    assert!(display.diagnostics().iter().any(|diagnostic| {
        diagnostic.object_id() == 5
            && diagnostic.source() == LayerSource::Page
            && matches!(
                diagnostic.kind(),
                RenderDiagnosticKind::FontFallback {
                    character: '中',
                    scalar_index: 0,
                    ..
                }
            )
    }));
}

#[test]
fn image_extensions_and_missing_glyphs_are_structured_source_aware_diagnostics() {
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:TextObject ID="5" Boundary="0 0 20 10" Font="10" Size="4"><ofd:TextCode X="0" Y="0">🦄</ofd:TextCode></ofd:TextObject><ofd:ImageObject ID="6" Boundary="0 10 6 4" ResourceID="20" Substitution="21" ImageMask="22"><ofd:Border LineWidth="1"/></ofd:ImageObject></ofd:Layer></ofd:Content></ofd:Page>"#;
    let mask_resources = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Fixture"><ofd:FontFile>font.ttf</ofd:FontFile></ofd:Font></ofd:Fonts><ofd:MultiMedias><ofd:MultiMedia ID="20" Type="Image" Format="PNG"><ofd:MediaFile>image.png</ofd:MediaFile></ofd:MultiMedia><ofd:MultiMedia ID="21" Type="Image" Format="PNG"><ofd:MediaFile>image.png</ofd:MediaFile></ofd:MultiMedia><ofd:MultiMedia ID="22" Type="Image" Format="PNG"><ofd:MediaFile>mask.png</ofd:MediaFile></ofd:MultiMedia></ofd:MultiMedias></ofd:Res>"#;
    let image_document = document(
        page,
        mask_resources,
        &[
            ("Doc_0/font.ttf", FONT),
            ("Doc_0/image.png", PNG),
            ("Doc_0/mask.png", MASK_PNG),
        ],
    );
    let page = image_document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();

    let display = builder(&resolver, &decoder).build(&page).unwrap();
    assert_eq!(display.diagnostics().len(), 2);
    assert!(matches!(
        display.diagnostics()[0].kind(),
        RenderDiagnosticKind::MissingGlyph {
            character: '🦄',
            ..
        }
    ));
    // The image draws with the declared border, and the mismatched mask is
    // reported as inapplicable.
    assert!(matches!(
        display.diagnostics()[1].kind(),
        RenderDiagnosticKind::ImageMaskIncompatible { resource_id: 22 }
    ));
    assert!(display.diagnostics().iter().any(|diagnostic| {
        diagnostic.object_id() == 5
            && diagnostic.source() == LayerSource::Page
            && matches!(
                diagnostic.kind(),
                RenderDiagnosticKind::MissingGlyph {
                    character: '🦄',
                    used_visible_replacement: true,
                    ..
                }
            )
    }));
    // The substitution resource is not preferred by default. The mask
    // fixture has different dimensions from the image, so the mask is
    // reported as incompatible (attributed to the image object) and the
    // image draws with its declared border regardless.
    assert!(display.diagnostics().iter().any(|diagnostic| {
        diagnostic.object_id() == 6
            && diagnostic.source() == LayerSource::Page
            && matches!(
                diagnostic.kind(),
                RenderDiagnosticKind::ImageMaskIncompatible { resource_id: 22 }
            )
    }));
    assert!(display
        .commands()
        .iter()
        .any(|command| matches!(command, Command::DrawImage { .. })));
}

#[test]
fn missing_image_asset_skips_the_object_with_a_diagnostic() {
    // ofdrw logs the missing media file and renders the rest of the page
    // (its containsJPEG.ofd references render without the images), so a
    // missing asset is a non-fatal diagnostic, not a hard error.
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject><ofd:ImageObject ID="6" Boundary="0 0 6 4" ResourceID="20"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let document = document(page, &resources("missing.png"), &[("Doc_0/font.ttf", FONT)]);
    let page = document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();

    let display_list = builder(&resolver, &decoder).build(&page).unwrap();
    assert!(!display_list.commands().is_empty());
    assert_eq!(display_list.diagnostics().len(), 1);
    let diagnostic = &display_list.diagnostics()[0];
    assert_eq!(diagnostic.object_id(), 6);
    assert_eq!(
        diagnostic.kind(),
        &RenderDiagnosticKind::ImageResourceMissing { resource_id: 20 }
    );
}

#[cfg(not(feature = "jbig2"))]
#[test]
fn undecodable_jbig2_image_is_skipped_with_a_diagnostic() {
    // Without the `jbig2` feature a JB2/GBIG2 image is skipped like other
    // non-fatal image problems and the rest of the page still renders.
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject><ofd:ImageObject ID="6" Boundary="0 0 6 4" ResourceID="22"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let resources = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:MultiMedias><ofd:MultiMedia ID="22" Type="Image" Format="GBIG2"><ofd:MediaFile>qr.jb2</ofd:MediaFile></ofd:MultiMedia></ofd:MultiMedias></ofd:Res>"#;
    let jbig2 = b"\x97JB2\r\n\x1a\n\x01\x00\x00\x00\x01";
    let document = document(page, resources, &[("Doc_0/qr.jb2", jbig2)]);
    let page = document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();

    let display_list = builder(&resolver, &decoder).build(&page).unwrap();
    assert!(!display_list.commands().is_empty());
    assert_eq!(display_list.diagnostics().len(), 1);
    let diagnostic = &display_list.diagnostics()[0];
    assert_eq!(diagnostic.object_id(), 6);
    assert_eq!(
        diagnostic.kind(),
        &RenderDiagnosticKind::ImageFormatUnsupported { resource_id: 22 }
    );
}

#[cfg(feature = "jbig2")]
#[test]
fn corrupted_jbig2_streams_fail_loudly_like_other_decoders() {
    // With the `jbig2` feature a corrupt stream is a decode error like a
    // corrupt PNG, not a silent skip.
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject><ofd:ImageObject ID="6" Boundary="0 0 6 4" ResourceID="22"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let resources = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:MultiMedias><ofd:MultiMedia ID="22" Type="Image" Format="GBIG2"><ofd:MediaFile>qr.jb2</ofd:MediaFile></ofd:MultiMedia></ofd:MultiMedias></ofd:Res>"#;
    let jbig2 = b"\x97JB2\r\n\x1a\n\x01\x00\x00\x00\x01";
    let document = document(page, resources, &[("Doc_0/qr.jb2", jbig2)]);
    let page = document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();

    let result = builder(&resolver, &decoder).build(&page);
    assert!(matches!(
        result,
        Err(rofd_render::Error::ObjectResourceProcessing { object_id: 6, .. })
    ));
}

#[test]
fn loaded_resource_processing_errors_retain_object_resource_and_asset_context() {
    let text_page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:TextObject ID="5" Boundary="0 0 20 10" Font="10" Size="4"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject></ofd:Layer></ofd:Content></ofd:Page>"#;
    let text_document = document(
        text_page,
        &resources("image.png"),
        &[("Doc_0/font.ttf", b"not-a-font")],
    );
    let page = text_document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();
    assert!(matches!(
        builder(&resolver, &decoder).build(&page),
        Err(Error::ObjectResourceProcessing {
            object_id: 5,
            resource_id: 10,
            kind: ResourceKind::Font,
            asset_path,
            source,
        }) if asset_path == "Doc_0/font.ttf" && matches!(*source, Error::InvalidFont { .. })
    ));

    let image_page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:ImageObject ID="6" Boundary="0 0 6 4" ResourceID="20"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let image_document = document(
        image_page,
        &resources("image.png"),
        &[("Doc_0/image.png", b"not-an-image")],
    );
    let page = image_document.page(0).unwrap();
    assert!(matches!(
        builder(&resolver, &decoder).build(&page),
        Err(Error::ObjectResourceProcessing {
            object_id: 6,
            resource_id: 20,
            kind: ResourceKind::Image,
            asset_path,
            source,
        }) if asset_path == "Doc_0/image.png"
            && matches!(*source, Error::UnsupportedImageFormat { .. })
    ));
}

#[test]
fn injected_font_failure_aborts_the_transaction_before_later_objects() {
    struct RejectingResolver;
    impl FontResolver for RejectingResolver {
        fn resolve_primary(
            &self,
            _resource: &rofd_core::FontResource,
        ) -> rofd_render::Result<Option<ResolvedFont>> {
            Err(Error::InvalidFont {
                identity: "injected".to_owned(),
                message: "rejected".to_owned(),
            })
        }

        fn resolve_fallback(&self, _character: char) -> rofd_render::Result<Option<ResolvedFont>> {
            panic!("fallback must not be queried after the hard primary failure")
        }
    }

    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject><ofd:TextObject ID="5" Boundary="0 0 20 10" Font="10" Size="4"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject><ofd:ImageObject ID="6" Boundary="0 0 6 4" ResourceID="20"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let document = document(
        page,
        &resources("image.png"),
        &[("Doc_0/font.ttf", FONT), ("Doc_0/image.png", PNG)],
    );
    let page = document.page(0).unwrap();
    let decoder = ImageDecoder::default();
    assert!(matches!(
        DisplayListBuilder::new(&RejectingResolver, &decoder).build(&page),
        Err(Error::ObjectResourceProcessing {
            object_id: 5,
            resource_id: 10,
            kind: ResourceKind::Font,
            source,
            ..
        }) if matches!(*source, Error::InvalidFont { .. })
    ));
}

#[test]
fn display_list_enforces_aggregate_unique_decoded_image_bytes() {
    let two_resources = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:ImageObject ID="5" Boundary="0 0 3 2" ResourceID="20"/><ofd:ImageObject ID="6" Boundary="3 0 3 2" ResourceID="21"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let distinct_document = document(
        two_resources,
        &resources("image.png"),
        &[("Doc_0/image.png", PNG)],
    );
    let page = distinct_document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();
    assert!(matches!(
        DisplayListBuilder::new(&resolver, &decoder)
            .with_max_decoded_image_bytes(24)
            .unwrap()
            .build(&page),
        Err(Error::DisplayListImageBudgetExceeded {
            required_bytes: 48,
            max_bytes: 24,
        })
    ));

    let repeated_resource = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:ImageObject ID="5" Boundary="0 0 3 2" ResourceID="20"/><ofd:ImageObject ID="6" Boundary="3 0 3 2" ResourceID="20"/></ofd:Layer></ofd:Content></ofd:Page>"#;
    let repeated_document = document(
        repeated_resource,
        &resources("image.png"),
        &[("Doc_0/image.png", PNG)],
    );
    let page = repeated_document.page(0).unwrap();
    let display = DisplayListBuilder::new(&resolver, &decoder)
        .with_max_decoded_image_bytes(24)
        .unwrap()
        .build(&page)
        .unwrap();
    assert_eq!(
        display
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::DrawImage { .. }))
            .count(),
        2
    );
}

#[test]
fn repeated_template_text_respects_exact_document_budgets_even_with_warm_caches() {
    fn open(limits: rofd_core::ResourceLimits) -> Document {
        let entries = [
            (
                "OFD.xml",
                br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>template-budget</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#.as_slice(),
            ),
            (
                "Doc_0/Document.xml",
                br#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes><ofd:TemplatePage ID="30" BaseLoc="Template.xml"/></ofd:CommonData><ofd:Pages><ofd:Page ID="1" BaseLoc="Page.xml"/></ofd:Pages></ofd:Document>"#.as_slice(),
            ),
            (
                "Doc_0/Res.xml",
                br#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Fixture"><ofd:FontFile>font.ttf</ofd:FontFile></ofd:Font></ofd:Fonts></ofd:Res>"#.as_slice(),
            ),
            (
                "Doc_0/Page.xml",
                br#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Template TemplateID="30"/><ofd:Template TemplateID="30"/></ofd:Page>"#.as_slice(),
            ),
            (
                "Doc_0/Template.xml",
                br#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="31"><ofd:TextObject ID="32" Boundary="0 0 10 10" Font="10" Size="4"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject></ofd:Layer></ofd:Content></ofd:Page>"#.as_slice(),
            ),
            ("Doc_0/font.ttf", FONT),
        ];
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (path, bytes) in entries {
            writer
                .start_file(path, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        Document::from_bytes(
            writer.finish().unwrap().into_inner(),
            LoadOptions {
                limits,
                ..LoadOptions::default()
            },
        )
        .unwrap()
    }

    let exact = rofd_core::ResourceLimits {
        max_text_characters_per_page: 2,
        max_glyphs_per_page: 2,
        max_text_expansion_entries: 6,
        ..rofd_core::ResourceLimits::default()
    };
    let document = open(exact.clone());
    let page = document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();
    let builder = DisplayListBuilder::new(&resolver, &decoder);
    for _ in 0..2 {
        let display = builder.build(&page).unwrap();
        assert_eq!(
            display
                .commands()
                .iter()
                .filter(|command| matches!(command, Command::DrawGlyphRun(_)))
                .count(),
            2
        );
    }

    let one_over = open(rofd_core::ResourceLimits {
        max_text_characters_per_page: 1,
        ..exact
    });
    assert!(one_over.page(0).is_err());
}

#[test]
fn visible_annotations_lower_after_layers_and_invisible_ones_do_not() {
    let page = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2"><ofd:PathObject ID="3" Boundary="0 0 10 10" Fill="true"><ofd:FillColor Value="0 0 0"/><ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content></ofd:Page>"#;
    let document_xml = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:PageArea></ofd:CommonData><ofd:Pages><ofd:Page ID="1" BaseLoc="Page.xml"/></ofd:Pages><ofd:Annotations>Annotations.xml</ofd:Annotations></ofd:Document>"#;
    let entry = r#"<ofd:Annotations xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Page PageID="1"><ofd:FileLoc>Annot.xml</ofd:FileLoc></ofd:Page></ofd:Annotations>"#;
    let annot = r#"<ofd:PageAnnot xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Annot Type="Watermark" ID="30"><ofd:Appearance Boundary="20 30 40 10"><ofd:PathObject ID="31" Boundary="0 0 40 10" Fill="true"><ofd:FillColor Value="128 128 128"/><ofd:AbbreviatedData>M 0 0 L 40 0 L 40 10 L 0 10 C</ofd:AbbreviatedData></ofd:PathObject></ofd:Appearance></ofd:Annot><ofd:Annot Type="Highlight" ID="40" Visible="false"><ofd:Appearance Boundary="0 0 5 5"><ofd:PathObject ID="41" Boundary="0 0 5 5" Fill="true"><ofd:FillColor Value="255 0 0"/><ofd:AbbreviatedData>M 0 0 L 5 0 L 5 5 L 0 5 C</ofd:AbbreviatedData></ofd:PathObject></ofd:Appearance></ofd:Annot></ofd:PageAnnot>"#;
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, ZipWriter};
    let ofd_xml = br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>annot</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("OFD.xml", ofd_xml.as_slice()),
        ("Doc_0/Document.xml", document_xml.as_bytes()),
        ("Doc_0/Page.xml", page.as_bytes()),
        ("Doc_0/Annotations.xml", entry.as_bytes()),
        ("Doc_0/Annot.xml", annot.as_bytes()),
    ] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let decoder = ImageDecoder::default();

    let display = builder(&resolver, &decoder).build(&page).unwrap();

    // Two paths draw: the page path plus the visible watermark appearance;
    // the invisible highlight lowers nothing.
    let draws = display
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::DrawPath(_)))
        .count();
    assert_eq!(draws, 2);
    // The appearance is translated to its page boundary.
    assert!(display.commands().iter().any(|command| matches!(
        command,
        Command::ConcatTransform(transform) if transform.e() == 20.0 && transform.f() == 30.0
    )));
    assert!(display.diagnostics().is_empty());
}
