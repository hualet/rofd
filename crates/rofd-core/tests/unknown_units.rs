mod support;

use rofd_core::{Document, Error, LoadOptions, PageObject, Strictness, WarningCode};
use support::minimal_ofd;

const PAGE_WITH_UNKNOWN_UNITS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content>
    <ofd:Layer ID="1">
      <ofd:PathObject ID="2" Boundary="10 10 50 20" Fill="true">
        <ofd:FillColor Value="255 0 0"/>
        <ofd:AbbreviatedData>M 0 0 L 50 0 L 50 20 L 0 20 C</ofd:AbbreviatedData>
      </ofd:PathObject>
      <ofd:VideoObject ID="9"/>
      <ofd:CustomThing ID="10">
        <ofd:PathObject ID="11" Boundary="0 0 1 1">
          <ofd:AbbreviatedData>M 0 0 L 1 0 C</ofd:AbbreviatedData>
        </ofd:PathObject>
      </ofd:CustomThing>
    </ofd:Layer>
  </ofd:Content>
</ofd:Page>"#;

#[test]
fn lenient_mode_skips_unknown_graphic_units_with_warning() {
    let document =
        Document::from_bytes(minimal_ofd(PAGE_WITH_UNKNOWN_UNITS), LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();

    let objects = page.layers()[0].objects();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].object_id(), 2);
    assert!(matches!(objects[0], PageObject::Path(_)));

    let warnings = document.warnings();
    let skipped: Vec<_> = warnings
        .iter()
        .filter(|warning| warning.code == WarningCode::UnknownGraphicUnitSkipped)
        .collect();
    assert_eq!(skipped.len(), 2, "{warnings:?}");
    assert!(
        skipped[0].message.contains("VideoObject (ID 9)"),
        "{skipped:?}"
    );
    assert!(
        skipped[1].message.contains("CustomThing (ID 10)"),
        "{skipped:?}"
    );
    assert_eq!(skipped[0].path, "Doc_0/Pages/Page_0/Content.xml");
}

#[test]
fn lenient_mode_records_warning_once_across_cached_page_loads() {
    let document =
        Document::from_bytes(minimal_ofd(PAGE_WITH_UNKNOWN_UNITS), LoadOptions::default()).unwrap();
    document.page(0).unwrap();
    document.page(0).unwrap();
    assert_eq!(document.warnings().len(), 2);
}

#[test]
fn strict_mode_rejects_unknown_graphic_units() {
    let options = LoadOptions {
        strictness: Strictness::Strict,
        ..LoadOptions::default()
    };
    let document = Document::from_bytes(minimal_ofd(PAGE_WITH_UNKNOWN_UNITS), options).unwrap();
    let error = document.page(0).unwrap_err();
    assert!(
        matches!(error, Error::InvalidStructure { ref message, .. } if message.contains("unknown graphic unit")),
        "{error:?}"
    );
}

const PAGE_WITH_UNKNOWN_UNIT_IN_PAGE_BLOCK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content>
    <ofd:Layer ID="1">
      <ofd:PageBlock ID="20">
        <ofd:AnnotationWidget ID="21"/>
        <ofd:PathObject ID="22" Boundary="0 0 10 10" Fill="true">
          <ofd:FillColor Value="0 0 255"/>
          <ofd:AbbreviatedData>M 0 0 L 10 0 L 10 10 L 0 10 C</ofd:AbbreviatedData>
        </ofd:PathObject>
      </ofd:PageBlock>
    </ofd:Layer>
  </ofd:Content>
</ofd:Page>"#;

#[test]
fn lenient_mode_skips_unknown_units_nested_in_page_blocks() {
    let document = Document::from_bytes(
        minimal_ofd(PAGE_WITH_UNKNOWN_UNIT_IN_PAGE_BLOCK),
        LoadOptions::default(),
    )
    .unwrap();
    let page = document.page(0).unwrap();

    let objects = page.layers()[0].objects();
    assert_eq!(objects.len(), 1);
    let PageObject::Group(group) = &objects[0] else {
        panic!("expected a page group");
    };
    assert_eq!(group.objects().len(), 1);
    assert_eq!(group.objects()[0].object_id(), 22);

    let warnings = document.warnings();
    assert!(
        warnings.iter().any(
            |warning| warning.code == WarningCode::UnknownGraphicUnitSkipped
                && warning.message.contains("AnnotationWidget (ID 21)")
        ),
        "{warnings:?}"
    );
}
