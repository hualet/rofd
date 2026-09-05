use std::io::{Cursor, Write};
use std::sync::Arc;
use std::thread;

use fontdb::Database;
use rofd_core::{Document, LoadOptions, PageObject, ResourceLimits};
use rofd_render::{
    position_glyph_runs, Error, FontDiagnostic, FontSource, ResolvedFont, SystemFontResolver,
};
use zip::{write::SimpleFileOptions, ZipWriter};

const FONT: &[u8] = include_bytes!("fixtures/fonts/phase3-subset.ttf");
const LATIN_FONT: &[u8] = include_bytes!("fixtures/fonts/phase3-latin-subset.ttf");

fn package(font_file: Option<&[u8]>, text: &str, extra: &str) -> Document {
    package_named(font_file, text, extra, "Noto Sans CJK SC")
}

fn package_named(font_file: Option<&[u8]>, text: &str, extra: &str, font_name: &str) -> Document {
    package_with_names(font_file, text, extra, font_name, Some(font_name))
}

fn package_with_names(
    font_file: Option<&[u8]>,
    text: &str,
    extra: &str,
    font_name: &str,
    family_name: Option<&str>,
) -> Document {
    let font_file_element = font_file.map_or(String::new(), |_| {
        "<ofd:FontFile>Fonts/phase3-subset.ttf</ofd:FontFile>".to_owned()
    });
    let family_name =
        family_name.map_or(String::new(), |family| format!(r#" FamilyName="{family}""#));
    let resource = format!(
        r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts>
          <ofd:Font ID="10" FontName="{font_name}"{family_name}>{font_file_element}</ofd:Font>
        </ofd:Fonts></ofd:Res>"#
    );
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 30 30</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"><ofd:TextObject ID="2" Boundary="0 0 30 10" Font="10" Size="4">{text}{extra}</ofd:TextObject></ofd:Layer></ofd:Content></ofd:Page>"#
    );
    let entries = [
        (
            "OFD.xml",
            br#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>fonts</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#.as_slice(),
        ),
        (
            "Doc_0/Document.xml",
            br#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 30 30</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes></ofd:CommonData><ofd:Pages><ofd:Page ID="3" BaseLoc="Pages/Page.xml"/></ofd:Pages></ofd:Document>"#.as_slice(),
        ),
        ("Doc_0/Res.xml", resource.as_bytes()),
        ("Doc_0/Pages/Page.xml", page.as_bytes()),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    if let Some(bytes) = font_file {
        writer
            .start_file(
                "Doc_0/Fonts/phase3-subset.ttf",
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    Document::from_bytes(
        writer.finish().unwrap().into_inner(),
        LoadOptions::default(),
    )
    .unwrap()
}

fn text_object(document: &Document) -> rofd_core::TextObject {
    let page = document.page(0).unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!("expected text object");
    };
    text.clone()
}

#[test]
fn embedded_font_has_priority_and_absent_deltas_use_font_advance() {
    let document = package(
        Some(FONT),
        r#"<ofd:TextCode X="1" Y="2">A中</ofd:TextCode>"#,
        "",
    );
    let font = document.font_resource(10).unwrap();
    let text = text_object(&document);
    let mut database = Database::new();
    database.load_font_data(FONT.to_vec());
    let resolver = SystemFontResolver::from_database(database, Vec::new(), 1 << 20);

    let runs = position_glyph_runs(&resolver, &font, &text, &ResourceLimits::default()).unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text(), "A中");
    assert_eq!(runs[0].source_range(), 0..2);
    assert_eq!(runs[0].glyphs().len(), 2);
    assert_eq!(
        (runs[0].glyphs()[0].x(), runs[0].glyphs()[0].y()),
        (1.0, 2.0)
    );
    assert!(runs[0].glyphs()[1].x() > 1.0);
    assert_eq!(runs[0].glyphs()[1].y(), 2.0);
    assert_eq!(runs[0].glyphs()[1].source_range(), 1..4);
    assert_eq!(
        runs[0].glyphs()[0].font_source(),
        &FontSource::Embedded { resource_id: 10 }
    );
    assert!(runs[0].diagnostics().is_empty());
}

#[test]
fn explicit_zero_deltas_stay_zero_and_each_text_code_keeps_its_origin() {
    let document = package(
        Some(FONT),
        r#"<ofd:TextCode X="1" Y="2" DeltaX="0 0">AB</ofd:TextCode><ofd:TextCode X="8" Y="9">C</ofd:TextCode>"#,
        "",
    );
    let text = text_object(&document);
    assert!(text.runs()[0].has_explicit_delta_x());
    assert!(!text.runs()[1].has_explicit_delta_x());
    let runs = position_glyph_runs(
        &SystemFontResolver::from_database(Database::new(), Vec::new(), 1 << 20),
        &document.font_resource(10).unwrap(),
        &text,
        &ResourceLimits::default(),
    )
    .unwrap();

    assert_eq!(runs[0].glyphs()[0].x(), 1.0);
    assert_eq!(runs[0].glyphs()[1].x(), 1.0);
    assert_eq!(
        (runs[1].glyphs()[0].x(), runs[1].glyphs()[0].y()),
        (8.0, 9.0)
    );
}

#[test]
fn controlled_database_supports_declared_family_then_ordered_fallback() {
    let mut database = Database::new();
    database.load_font_data(FONT.to_vec());
    let resolver =
        SystemFontResolver::from_database(database, vec!["Noto Sans CJK SC".to_owned()], 1 << 20);
    let declared = package(None, r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#, "");
    let runs = position_glyph_runs(
        &resolver,
        &declared.font_resource(10).unwrap(),
        &text_object(&declared),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        runs[0].glyphs()[0].font_source(),
        FontSource::System { .. }
    ));

    let name_only = package_with_names(
        None,
        r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#,
        "",
        "NotoSansCJKsc-Regular",
        None,
    );
    let runs = position_glyph_runs(
        &resolver,
        &name_only.font_resource(10).unwrap(),
        &text_object(&name_only),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        runs[0].glyphs()[0].font_source(),
        FontSource::System { .. }
    ));

    let missing_family = package_named(
        None,
        r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#,
        "",
        "Definitely Missing",
    );
    let mut fallback_database = Database::new();
    fallback_database.load_font_data(FONT.to_vec());
    let fallback = SystemFontResolver::from_database(
        fallback_database,
        vec![
            "Unavailable First".to_owned(),
            "Noto Sans CJK SC".to_owned(),
        ],
        1 << 20,
    );
    let runs = position_glyph_runs(
        &fallback,
        &missing_family.font_resource(10).unwrap(),
        &text_object(&missing_family),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        runs[0].diagnostics()[0],
        FontDiagnostic::FamilyFallback { .. }
    ));
}

#[test]
fn fallback_is_selected_per_character_and_missing_uses_a_synthetic_box() {
    let document = package(
        Some(LATIN_FONT),
        r#"<ofd:TextCode X="0" Y="0">A中</ofd:TextCode>"#,
        "",
    );
    let mut database = Database::new();
    database.load_font_data(FONT.to_vec());
    let resolver =
        SystemFontResolver::from_database(database, vec!["Noto Sans CJK SC".to_owned()], 1 << 20);
    let runs = position_glyph_runs(
        &resolver,
        &document.font_resource(10).unwrap(),
        &text_object(&document),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        runs[0].glyphs()[0].font_source(),
        FontSource::Embedded { .. }
    ));
    assert!(matches!(
        runs[0].glyphs()[1].font_source(),
        FontSource::ConfiguredFallback { .. }
    ));

    let visible_replacement = package(
        Some(LATIN_FONT),
        r#"<ofd:TextCode X="0" Y="0">🦄</ofd:TextCode>"#,
        "",
    );
    let runs = position_glyph_runs(
        &SystemFontResolver::empty(Vec::new(), 1 << 20),
        &visible_replacement.font_resource(10).unwrap(),
        &text_object(&visible_replacement),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert!(!runs[0].glyphs()[0].is_synthetic_box());
    assert_ne!(runs[0].glyphs()[0].glyph_id(), 0);
    assert!(matches!(
        runs[0].diagnostics()[0],
        FontDiagnostic::MissingGlyph {
            used_visible_replacement: true,
            ..
        }
    ));

    let missing = package_named(
        None,
        r#"<ofd:TextCode X="0" Y="0">🦄</ofd:TextCode>"#,
        "",
        "Definitely Missing",
    );
    let runs = position_glyph_runs(
        &SystemFontResolver::empty(Vec::new(), 1 << 20),
        &missing.font_resource(10).unwrap(),
        &text_object(&missing),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(runs[0].glyphs()[0].font_source(), &FontSource::Missing);
    assert!(runs[0].glyphs()[0].is_synthetic_box());
    assert!(matches!(
        runs[0].diagnostics()[0],
        FontDiagnostic::MissingGlyph {
            used_visible_replacement: false,
            ..
        }
    ));
}

#[test]
fn cgtransform_glyphs_override_charmap_and_validate_font_range() {
    let document = package(
        Some(FONT),
        r#"<ofd:TextCode X="0" Y="0">AB</ofd:TextCode>"#,
        r#"<ofd:CGTransform CodePosition="0" CodeCount="2" GlyphCount="2"><ofd:Glyphs>3 2</ofd:Glyphs></ofd:CGTransform>"#,
    );
    let resolver = SystemFontResolver::from_database(Database::new(), Vec::new(), 1 << 20);
    let runs = position_glyph_runs(
        &resolver,
        &document.font_resource(10).unwrap(),
        &text_object(&document),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(
        runs[0]
            .glyphs()
            .iter()
            .map(|glyph| glyph.glyph_id())
            .collect::<Vec<_>>(),
        [3, 2]
    );
    assert_eq!(runs[0].glyphs()[0].source_scalar_range(), 0..2);
    assert!(runs[0].glyphs()[1].x() > runs[0].glyphs()[0].x());

    let one_to_many = package(
        Some(FONT),
        r#"<ofd:TextCode X="0" Y="0">AB</ofd:TextCode>"#,
        r#"<ofd:CGTransform CodePosition="0" CodeCount="1" GlyphCount="2"><ofd:Glyphs>2 3</ofd:Glyphs></ofd:CGTransform>"#,
    );
    let runs = position_glyph_runs(
        &resolver,
        &one_to_many.font_resource(10).unwrap(),
        &text_object(&one_to_many),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(runs[0].glyphs().len(), 3);
    assert_eq!(runs[0].glyphs()[0].character(), Some('A'));
    assert_eq!(runs[0].glyphs()[1].character(), None);
    assert_eq!(runs[0].glyphs()[2].character(), Some('B'));

    let invalid = package(
        Some(FONT),
        r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#,
        r#"<ofd:CGTransform CodePosition="0" CodeCount="1" GlyphCount="1"><ofd:Glyphs>99999</ofd:Glyphs></ofd:CGTransform>"#,
    );
    assert!(matches!(
        position_glyph_runs(
            &resolver,
            &invalid.font_resource(10).unwrap(),
            &text_object(&invalid),
            &ResourceLimits::default()
        ),
        Err(Error::InvalidGlyph {
            glyph_id: 99999,
            ..
        })
    ));
}

#[test]
fn cumulative_deltas_cross_run_cg_ranges_and_renderer_limits_are_exact() {
    let deltas = package(
        Some(FONT),
        r#"<ofd:TextCode X="1" Y="2" DeltaX="2 3 4">ABC</ofd:TextCode>"#,
        "",
    );
    let resolver = SystemFontResolver::empty(Vec::new(), 1 << 20);
    let runs = position_glyph_runs(
        &resolver,
        &deltas.font_resource(10).unwrap(),
        &text_object(&deltas),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(
        runs[0]
            .glyphs()
            .iter()
            .map(|glyph| glyph.x())
            .collect::<Vec<_>>(),
        [1.0, 3.0, 6.0]
    );

    let crossing = package(
        Some(FONT),
        r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode><ofd:TextCode X="10" Y="10">B</ofd:TextCode>"#,
        r#"<ofd:CGTransform CodePosition="0" CodeCount="2"><ofd:Glyphs>2</ofd:Glyphs></ofd:CGTransform>"#,
    );
    let runs = position_glyph_runs(
        &resolver,
        &crossing.font_resource(10).unwrap(),
        &text_object(&crossing),
        &ResourceLimits::default(),
    )
    .unwrap();
    assert_eq!(runs[0].glyphs().len(), 1);
    assert_eq!(runs[0].glyphs()[0].source_scalar_range(), 0..2);
    assert!(runs[1].glyphs().is_empty());

    let mut exact = ResourceLimits {
        max_glyphs_per_page: 3,
        max_text_expansion_entries: 4,
        ..ResourceLimits::default()
    };
    position_glyph_runs(
        &resolver,
        &deltas.font_resource(10).unwrap(),
        &text_object(&deltas),
        &exact,
    )
    .unwrap();
    exact.max_glyphs_per_page = 2;
    assert!(matches!(
        position_glyph_runs(
            &resolver,
            &deltas.font_resource(10).unwrap(),
            &text_object(&deltas),
            &exact
        ),
        Err(Error::InvalidTextLayout {
            field: "max_glyphs_per_page",
            ..
        })
    ));
    exact.max_glyphs_per_page = 3;
    exact.max_text_expansion_entries = 3;
    assert!(matches!(
        position_glyph_runs(
            &resolver,
            &deltas.font_resource(10).unwrap(),
            &text_object(&deltas),
            &exact
        ),
        Err(Error::InvalidTextLayout {
            field: "max_text_expansion_entries",
            ..
        })
    ));
}

#[test]
fn system_font_bytes_are_bounded_before_ownership_and_position_math_fails_closed() {
    let document = package(None, r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#, "");
    let mut database = Database::new();
    database.load_font_data(FONT.to_vec());
    let resolver = SystemFontResolver::from_database(
        database,
        vec!["Noto Sans CJK SC".to_owned()],
        (FONT.len() - 1) as u64,
    );
    assert!(matches!(
        position_glyph_runs(
            &resolver,
            &document.font_resource(10).unwrap(),
            &text_object(&document),
            &ResourceLimits::default()
        ),
        Err(Error::FontBytesExceeded { .. })
    ));

    let huge = package(
        Some(FONT),
        r#"<ofd:TextCode X="1.7976931348623157e308" Y="0" DeltaX="1.7976931348623157e308 0">AA</ofd:TextCode>"#,
        "",
    );
    assert!(matches!(
        position_glyph_runs(
            &SystemFontResolver::empty(Vec::new(), 1 << 20),
            &huge.font_resource(10).unwrap(),
            &text_object(&huge),
            &ResourceLimits::default()
        ),
        Err(Error::InvalidTextLayout {
            field: "glyph x",
            ..
        })
    ));
}

#[test]
fn invalid_embedded_data_and_face_index_are_structured_errors() {
    let document = package(
        Some(b"not a font"),
        r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#,
        "",
    );
    let resolver = SystemFontResolver::from_database(Database::new(), Vec::new(), 1 << 20);
    assert!(matches!(
        position_glyph_runs(
            &resolver,
            &document.font_resource(10).unwrap(),
            &text_object(&document),
            &ResourceLimits::default()
        ),
        Err(Error::InvalidFont { .. })
    ));
    assert!(matches!(
        ResolvedFont::from_bytes(
            Arc::from(FONT),
            99,
            "fixture".to_owned(),
            FontSource::System {
                identity: "fixture".to_owned()
            }
        ),
        Err(Error::InvalidFont { .. })
    ));
}

#[test]
fn concurrent_resolution_reuses_the_same_stable_font_identity() {
    let document = Arc::new(package(
        Some(FONT),
        r#"<ofd:TextCode X="0" Y="0">A</ofd:TextCode>"#,
        "",
    ));
    let resolver = Arc::new(SystemFontResolver::from_database(
        Database::new(),
        Vec::new(),
        1 << 20,
    ));
    let handles = (0..8)
        .map(|_| {
            let document = Arc::clone(&document);
            let resolver = Arc::clone(&resolver);
            thread::spawn(move || {
                let font = document.font_resource(10).unwrap();
                let runs = position_glyph_runs(
                    resolver.as_ref(),
                    &font,
                    &text_object(&document),
                    &ResourceLimits::default(),
                )
                .unwrap();
                let glyph = &runs[0].glyphs()[0];
                (
                    glyph.font_identity().to_owned(),
                    glyph.font().unwrap().encoded_bytes_arc(),
                )
            })
        })
        .collect::<Vec<_>>();
    let resolved = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert!(resolved
        .iter()
        .all(|(identity, _)| identity == &resolved[0].0));
    assert!(resolved
        .iter()
        .all(|(_, bytes)| Arc::ptr_eq(bytes, &resolved[0].1)));
}
