mod support;

use rofd_core::{Color, Document, Error, LoadOptions, PageObject, Strictness};
use support::ofd_with_document_page_and_entries;

const DOCUMENT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData>
  <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
  <ofd:PublicRes>Res.xml</ofd:PublicRes>
</ofd:CommonData><ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;

fn page_with(
    objects: &str,
    catalog: &str,
    options: LoadOptions,
) -> rofd_core::Result<rofd_core::Page> {
    let page_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1">{objects}</ofd:Layer></ofd:Content></ofd:Page>"#
    );
    let bytes = ofd_with_document_page_and_entries(
        DOCUMENT_XML,
        &page_xml,
        &[("Doc_0/Res.xml", catalog.as_bytes())],
    );
    Document::from_bytes(bytes, options)?.page(0)
}

fn path_fill(fill: &str) -> String {
    format!(
        r#"<ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true"><ofd:FillColor {fill}/><ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData></ofd:PathObject>"#
    )
}

fn fill_color(objects: &str, catalog: &str, options: LoadOptions) -> Color {
    let page = page_with(objects, catalog, options)
        .unwrap_or_else(|error| panic!("page should load: {error:?}"));
    match &page.layers()[0].objects()[0] {
        PageObject::Path(path) => path.fill().expect("fill color"),
        other => panic!("expected a path object, got {other:?}"),
    }
}

#[test]
fn channel_count_selects_gray_rgb_and_cmyk_spaces() {
    let catalog = "<Res/>";

    let gray = fill_color(
        &path_fill(r#"Value="128""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!(
        (gray.red, gray.green, gray.blue, gray.alpha),
        (128, 128, 128, 255)
    );

    let rgb = fill_color(
        &path_fill(r#"Value="10 20 30" Alpha="64""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((rgb.red, rgb.green, rgb.blue, rgb.alpha), (10, 20, 30, 64));

    // 100/0/0/0 CMYK: cyan at 100% removes all red.
    let cmyk = fill_color(
        &path_fill(r#"Value="100 0 0 0""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((cmyk.red, cmyk.green, cmyk.blue), (0, 255, 255));

    // 50/20/0/10 matches the shared conversion helper.
    let mixed = fill_color(
        &path_fill(r#"Value="50 20 0 10""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((mixed.red, mixed.green, mixed.blue), (115, 184, 230));
}

#[test]
fn color_space_reference_resolves_palette_index_colors() {
    let catalog = r#"<Res>
  <ColorSpaces>
    <ColorSpace ID="30" Type="CMYK"><Palette><CV>0 0 0 100</CV><CV>100 100 0 0</CV></Palette></ColorSpace>
    <ColorSpace ID="31" Type="GRAY"/>
  </ColorSpaces>
</Res>"#;

    let black = fill_color(
        &path_fill(r#"Index="0" ColorSpace="30""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((black.red, black.green, black.blue), (0, 0, 0));

    let blue = fill_color(
        &path_fill(r#"Index="1" ColorSpace="30" Alpha="32""#),
        catalog,
        LoadOptions::default(),
    );
    // Palette entry "100 100 0 0": full cyan and magenta ink leaves blue.
    assert_eq!(
        (blue.red, blue.green, blue.blue, blue.alpha),
        (0, 0, 255, 32)
    );

    // A declared space also validates plain values; strict mode rejects a
    // channel count that disagrees with the space.
    let gray = fill_color(
        &path_fill(r#"Value="200" ColorSpace="31""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((gray.red, gray.green, gray.blue), (200, 200, 200));

    let strict = LoadOptions {
        strictness: Strictness::Strict,
        ..LoadOptions::default()
    };
    let error = page_with(
        &path_fill(r#"Value="1 2 3" ColorSpace="30""#),
        catalog,
        strict,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::InvalidPageObject { field, .. } if field == "FillColor"),
        "{error:?}"
    );
}

#[test]
fn lenient_mode_defaults_missing_color_references() {
    let catalog = "<Res/>";

    // Unknown ColorSpace reference: lenient falls back like ofdrw to
    // resolving by channel count.
    let fallback = fill_color(
        &path_fill(r#"Value="1 2 3" ColorSpace="99""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((fallback.red, fallback.green, fallback.blue), (1, 2, 3));

    // Index without a palette: default colour.
    let default = fill_color(
        &path_fill(r#"Index="3" ColorSpace="99""#),
        catalog,
        LoadOptions::default(),
    );
    assert_eq!((default.red, default.green, default.blue), (0, 0, 0));

    let strict = LoadOptions {
        strictness: Strictness::Strict,
        ..LoadOptions::default()
    };
    let error = page_with(
        &path_fill(r#"Value="1 2 3" ColorSpace="99""#),
        catalog,
        strict.clone(),
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::InvalidPageObject { ref message, .. } if message.contains("unknown resource object ID 99")),
        "{error:?}"
    );
    let error = page_with(&path_fill(r#"Index="3" ColorSpace="99""#), catalog, strict).unwrap_err();
    assert!(
        matches!(error, Error::InvalidPageObject { ref message, .. } if message.contains("unknown resource object ID 99")),
        "{error:?}"
    );
}

#[test]
fn drawparam_colors_resolve_through_color_spaces() {
    let catalog = r#"<Res>
  <DrawParams>
    <DrawParam ID="20"><FillColor Value="100 0 0 0"/></DrawParam>
    <DrawParam ID="21" Relative="20"><StrokeColor Index="1" ColorSpace="30"/></DrawParam>
  </DrawParams>
  <ColorSpaces>
    <ColorSpace ID="30" Type="RGB"><Palette><CV>9 8 7</CV><CV>4 5 6</CV></Palette></ColorSpace>
  </ColorSpaces>
</Res>"#;
    let object = r#"<ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true" Stroke="true" DrawParam="21"><ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData></ofd:PathObject>"#;

    let page = page_with(object, catalog, LoadOptions::default()).unwrap();
    match &page.layers()[0].objects()[0] {
        PageObject::Path(path) => {
            let fill = path.fill().expect("inherited fill color");
            assert_eq!((fill.red, fill.green, fill.blue), (0, 255, 255));
            let stroke = path.stroke().expect("local stroke color");
            assert_eq!((stroke.red, stroke.green, stroke.blue), (4, 5, 6));
        }
        other => panic!("expected a path object, got {other:?}"),
    }
}
