mod support;

use std::path::PathBuf;
use std::sync::{Arc, Barrier};

use rofd_core::{
    Color, Document, Error, FillRule, LayerType, LoadOptions, PageObject, Rect, ResourceLimits,
    Transform, WarningCode,
};
use support::minimal_ofd;

fn page_with(content: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  {content}
</ofd:Page>"#
    )
}

fn open_page(content: &str) -> rofd_core::Result<rofd_core::Page> {
    let document = Document::from_bytes(minimal_ofd(&page_with(content)), LoadOptions::default())?;
    document.page(0)
}

fn open_page_with_limits(
    content: &str,
    limits: ResourceLimits,
) -> rofd_core::Result<rofd_core::Page> {
    let document = Document::from_bytes(
        minimal_ofd(&page_with(content)),
        LoadOptions {
            limits,
            ..LoadOptions::default()
        },
    )?;
    document.page(0)
}

fn open_page_strict(content: &str) -> rofd_core::Result<rofd_core::Page> {
    let document = Document::from_bytes(
        minimal_ofd(&page_with(content)),
        LoadOptions {
            strictness: rofd_core::Strictness::Strict,
            ..LoadOptions::default()
        },
    )?;
    document.page(0)
}

fn simple_path(id: u64, data: &str) -> String {
    format!(
        r#"<ofd:PathObject ID="{id}" Boundary="0 0 10 10">
  <ofd:AbbreviatedData>{data}</ofd:AbbreviatedData>
</ofd:PathObject>"#
    )
}

#[test]
fn exposes_three_layer_types_in_source_order_and_defaults_to_body() {
    let page = open_page(
        r#"<ofd:Content>
  <ofd:Layer ID="1" Type="Background"/>
  <ofd:Layer ID="2"/>
  <ofd:Layer ID="3" Type="Foreground"/>
</ofd:Content>"#,
    )
    .unwrap();

    assert_eq!(page.layers().len(), 3);
    assert_eq!(page.layers()[0].object_id(), 1);
    assert_eq!(page.layers()[0].kind(), LayerType::Background);
    assert_eq!(page.layers()[1].kind(), LayerType::Body);
    assert_eq!(page.layers()[2].kind(), LayerType::Foreground);
}

#[test]
fn exposes_path_defaults() {
    let page = open_page(&format!(
        "<ofd:Content><ofd:Layer ID=\"1\">{}</ofd:Layer></ofd:Content>",
        simple_path(2, "M 0 0 L 10 10")
    ))
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected a path object");
    };

    assert_eq!(path.object_id(), 2);
    assert_eq!(
        path.boundary(),
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }
    );
    assert_eq!(path.transform(), Transform::IDENTITY);
    assert_eq!(path.path_data().commands().len(), 2);
    assert_eq!(path.stroke(), Some(Color::BLACK));
    assert_eq!(path.fill(), None);
    assert_eq!(path.line_width(), 0.353);
    assert_eq!(path.fill_rule(), FillRule::NonZero);
}

#[test]
fn exposes_explicit_path_values_and_disabled_paints() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="1 2 3 4" CTM="1 2 3 4 5 6" Stroke="false" Fill="true"
  LineWidth="1.25" Rule="Even-Odd" Alpha="200">
  <ofd:StrokeColor Value="invalid"/>
  <ofd:FillColor Value="10 20 30" Alpha="128"/>
  <ofd:AbbreviatedData>M 1 2 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected a path object");
    };

    assert_eq!(
        path.transform(),
        Transform::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0).unwrap()
    );
    assert_eq!(path.stroke(), None);
    assert_eq!(
        path.fill(),
        Some(Color {
            red: 10,
            green: 20,
            blue: 30,
            alpha: 100,
        })
    );
    assert_eq!(path.line_width(), 1.25);
    assert_eq!(path.fill_rule(), FillRule::EvenOdd);
}

#[test]
fn alpha_combination_rounds_to_the_nearest_integer() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="0 0 1 1" Alpha="128">
  <ofd:StrokeColor Value="1 2 3" Alpha="128"/>
  <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected a path object");
    };
    assert_eq!(path.stroke().unwrap().alpha, 64);
}

#[test]
fn absent_paint_colors_apply_object_alpha_to_their_distinct_defaults() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="0 0 1 1" Fill="true" Alpha="128">
  <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected a path object");
    };

    assert_eq!(
        path.stroke(),
        Some(Color {
            alpha: 128,
            ..Color::BLACK
        })
    );
    assert_eq!(
        path.fill(),
        Some(Color {
            alpha: 0,
            ..Color::BLACK
        })
    );
}

#[test]
fn accepts_xml_schema_numeric_boolean_attributes() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1">
  <ofd:PathObject ID="2" Boundary="0 0 1 1" Stroke="1" Fill="0">
    <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
  </ofd:PathObject>
  <ofd:PathObject ID="3" Boundary="0 0 1 1" Stroke="0" Fill="1">
    <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
  </ofd:PathObject>
</ofd:Layer></ofd:Content>"#,
    )
    .unwrap();

    let PageObject::Path(first) = &page.layers()[0].objects()[0] else {
        panic!("expected first path object");
    };
    assert_eq!(first.stroke(), Some(Color::BLACK));
    assert_eq!(first.fill(), None);
    let PageObject::Path(second) = &page.layers()[0].objects()[1] else {
        panic!("expected second path object");
    };
    assert_eq!(second.stroke(), None);
    assert_eq!(second.fill().unwrap().alpha, 0);
}

#[test]
fn nested_page_blocks_preserve_exact_source_order() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1">
  <ofd:PathObject ID="2" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>
  <ofd:PageBlock ID="3">
    <ofd:PathObject ID="4" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>
    <ofd:PageBlock ID="5"><ofd:PathObject ID="6" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock>
    <ofd:PathObject ID="7" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>
  </ofd:PageBlock>
  <ofd:PathObject ID="8" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>
</ofd:Layer></ofd:Content>"#,
    )
    .unwrap();

    let objects = page.layers()[0].objects();
    assert!(matches!(objects[0], PageObject::Path(_)));
    let PageObject::Group(group) = &objects[1] else {
        panic!("expected a page group");
    };
    assert_eq!(group.object_id(), 3);
    assert_eq!(group.objects().len(), 3);
    assert!(matches!(group.objects()[0], PageObject::Path(_)));
    let PageObject::Group(nested) = &group.objects()[1] else {
        panic!("expected a nested page group");
    };
    assert_eq!(nested.object_id(), 5);
    assert_eq!(nested.objects()[0].object_id(), 6);
    assert_eq!(group.objects()[2].object_id(), 7);
    assert_eq!(objects[2].object_id(), 8);
}

const COMPOSITE_CATALOG: &str = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CompositeGraphicUnits>
    <ofd:CompositeGraphicUnit ID="10" Width="20" Height="10">
      <ofd:Content ID="20">
        <ofd:PathObject ID="21" Boundary="0 0 5 5" Fill="true">
          <ofd:FillColor Value="1 2 3"/>
          <ofd:AbbreviatedData>M 0 0 L 5 0 L 5 5 L 0 5 C</ofd:AbbreviatedData>
        </ofd:PathObject>
      </ofd:Content>
    </ofd:CompositeGraphicUnit>
    <ofd:CompositeGraphicUnit ID="11" Width="20" Height="10">
      <ofd:Content ID="30">
        <ofd:CompositeObject ID="31" Boundary="0 0 10 10" ResourceID="11"/>
      </ofd:Content>
    </ofd:CompositeGraphicUnit>
  </ofd:CompositeGraphicUnits>
</ofd:Res>"#;

fn composite_page(object: &str) -> rofd_core::Result<rofd_core::Page> {
    let document_xml = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea><ofd:PublicRes>Res.xml</ofd:PublicRes></ofd:CommonData><ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
    let page_xml = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1">{object}</ofd:Layer></ofd:Content></ofd:Page>"#
    );
    let bytes = support::ofd_with_document_page_and_entries(
        document_xml,
        &page_xml,
        &[("Doc_0/Res.xml", COMPOSITE_CATALOG.as_bytes())],
    );
    Document::from_bytes(bytes, LoadOptions::default())?.page(0)
}

#[test]
fn composite_objects_expand_their_referenced_vector_graphic() {
    let page = composite_page(
        r#"<ofd:CompositeObject ID="4" Boundary="10 20 30 40" CTM="2 0 0 2 0 0" ResourceID="10"/>"#,
    )
    .unwrap();
    let objects = page.layers()[0].objects();
    assert_eq!(objects.len(), 1);
    let PageObject::Composite(composite) = &objects[0] else {
        panic!("expected a composite object");
    };
    assert_eq!(composite.object_id(), 4);
    assert_eq!(composite.resource_id(), 10);
    assert_eq!(composite.boundary().x, 10.0);
    assert_eq!(composite.transform().a(), 2.0);
    assert_eq!(composite.objects().len(), 1);
    assert!(matches!(&composite.objects()[0], PageObject::Path(path) if path.object_id() == 21));
}

#[test]
fn vector_graphic_reference_cycles_are_rejected() {
    let error =
        composite_page(r#"<ofd:CompositeObject ID="4" Boundary="0 0 10 10" ResourceID="11"/>"#)
            .unwrap_err();
    assert!(
        matches!(error, Error::InvalidStructure { ref message, .. } if message.contains("cycle")),
        "{error:?}"
    );
}

#[test]
fn composite_objects_require_a_known_vector_graphic_resource() {
    let error =
        composite_page(r#"<ofd:CompositeObject ID="4" Boundary="0 0 10 10" ResourceID="99"/>"#)
            .unwrap_err();
    assert!(
        matches!(error, Error::InvalidPageObject { object_id: 4, field, .. } if field == "ResourceID"),
        "{error:?}"
    );
}

#[test]
fn content_absent_means_no_layers() {
    assert!(open_page("").unwrap().layers().is_empty());
}

#[test]
fn rejects_invalid_layer_type() {
    let error = open_page(r#"<ofd:Content><ofd:Layer ID="1" Type="Watermark"/></ofd:Content>"#)
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidValue {
            field: "layer type",
            ..
        }
    ));
}

#[test]
fn rejects_invalid_fill_rule() {
    let error = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1" Rule="evenodd"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidValue {
            field: "fill rule",
            ..
        }
    ));
}

#[test]
fn rejects_nonpositive_or_nonfinite_line_width() {
    for value in ["0", "-1", "NaN", "inf"] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1" LineWidth="{value}"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let error = open_page(&content).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidPageObject {
                field: "LineWidth",
                object_id: 2,
                ..
            }
        ));
    }
}

#[test]
fn rejects_invalid_stroke_and_fill_attributes() {
    for (attribute, field) in [("Stroke=\"yes\"", "stroke"), ("Fill=\"off\"", "fill")] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1" {attribute}><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let error = open_page(&content).unwrap_err();
        let matches_expected = if field == "color" {
            matches!(
                error,
                Error::InvalidPageObject {
                    field: "StrokeColor",
                    object_id: 2,
                    ..
                }
            )
        } else {
            matches!(error, Error::InvalidValue { field: actual, .. } if actual == field)
        };
        assert!(matches_expected, "expected invalid {field}, got {error:?}");
    }
}

#[test]
fn rejects_invalid_object_alpha_attributes() {
    for value in ["not-a-number", "-1", "256"] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1" Alpha="{value}"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let error = open_page(&content).unwrap_err();
        assert!(
            matches!(error, Error::InvalidValue { field: "alpha", .. }),
            "expected invalid alpha, got {error:?}"
        );
    }
}

#[test]
fn rejects_invalid_boundary_transform_path_and_enabled_color() {
    let cases = [
        (
            "Boundary=\"NaN 0 1 1\"",
            "<ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>",
            "boundary",
        ),
        (
            "Boundary=\"0 0 1 1\" CTM=\"1 0 0 nope 0 0\"",
            "<ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>",
            "transform",
        ),
        (
            "Boundary=\"0 0 1 1\"",
            "<ofd:AbbreviatedData>L 0 0</ofd:AbbreviatedData>",
            "path data",
        ),
        (
            "Boundary=\"0 0 1 1\"",
            "<ofd:StrokeColor Value=\"256 0 0\"/><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>",
            "color",
        ),
    ];
    for (attributes, children, field) in cases {
        let content = format!(
            "<ofd:Content><ofd:Layer ID=\"1\"><ofd:PathObject ID=\"2\" {attributes}>{children}</ofd:PathObject></ofd:Layer></ofd:Content>"
        );
        let error = open_page(&content).unwrap_err();
        let matches_expected = if field == "color" {
            matches!(
                error,
                Error::InvalidPageObject {
                    field: "StrokeColor",
                    object_id: 2,
                    ..
                }
            )
        } else {
            matches!(error, Error::InvalidValue { field: actual, .. } if actual == field)
        };
        assert!(matches_expected, "expected invalid {field}, got {error:?}");
    }
}

#[test]
fn strict_mode_reports_owner_context_for_missing_path_color_value() {
    for (attributes, child, expected_field) in [
        ("Fill=\"true\"", "<ofd:FillColor/>", "FillColor"),
        ("Stroke=\"true\"", "<ofd:StrokeColor/>", "StrokeColor"),
    ] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1" {attributes}>{child}<ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let error = open_page_strict(&content).unwrap_err();
        assert!(
            matches!(error, Error::InvalidPageObject { object_id: 2, field, ref path, .. } if field == expected_field && path.ends_with("Content.xml")),
            "expected {expected_field}, got {error:?}"
        );
    }
}

#[test]
fn boundary_allows_negative_origins_and_leniently_tolerates_zero_dimensions() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="-1 -2 10 10"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected path object");
    };
    assert_eq!(path.boundary().x, -1.0);
    assert_eq!(path.boundary().y, -2.0);

    // Real-world producers emit zero or negative dimension boundaries for
    // degenerate objects (e.g. ofdrw's keyword.ofd); lenient mode accepts
    // them and only non-finite values stay invalid.
    for boundary in ["0 0 0 1", "0 0 1 0", "0 0 -1 1", "0 0 1 -1"] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="{boundary}"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        open_page(&content).unwrap();
    }

    for boundary in ["NaN 0 1 1", "0 inf 1 1"] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="{boundary}"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let error = open_page(&content).unwrap_err();
        assert!(
            matches!(
                error,
                Error::InvalidValue {
                    field: "boundary",
                    ..
                }
            ),
            "expected invalid boundary, got {error:?}"
        );
    }
}

#[test]
fn strict_mode_requires_positive_boundary_dimensions() {
    for boundary in ["0 0 0 1", "0 0 1 0"] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="{boundary}"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let document = Document::from_bytes(
            minimal_ofd(&page_with(&content)),
            LoadOptions {
                strictness: rofd_core::Strictness::Strict,
                ..LoadOptions::default()
            },
        )
        .unwrap();
        let error = document.page(0).unwrap_err();
        assert!(
            matches!(
                error,
                Error::InvalidValue {
                    field: "boundary",
                    ..
                }
            ),
            "expected invalid boundary, got {error:?}"
        );
    }
}

#[test]
fn rejects_zero_or_malformed_layer_group_and_leaf_ids() {
    let cases = [
        r#"<ofd:Content><ofd:Layer ID="0"/></ofd:Content>"#,
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="not-an-id"/></ofd:Layer></ofd:Content>"#,
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="not-an-id" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CompositeObject ID="0"/></ofd:Layer></ofd:Content>"#,
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CompositeObject ID="not-an-id"/></ofd:Layer></ofd:Content>"#,
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CompositeObject ID="0"/></ofd:Layer></ofd:Content>"#,
    ];
    for content in cases {
        let error = open_page(content).unwrap_err();
        assert!(
            matches!(
                error,
                Error::InvalidValue {
                    field: "object ID",
                    ..
                }
            ),
            "expected invalid object ID, got {error:?}"
        );
    }
}

#[test]
fn rejects_duplicate_ids_anywhere_on_a_page() {
    let error = open_page_strict(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::InvalidStructure { message, .. } if message.contains("duplicate object ID 3"))
    );
}

#[test]
fn lenient_mode_tolerates_duplicate_and_missing_object_ids() {
    // Several ofdrw converter fixtures duplicate object IDs inside one
    // template; ofdrw's converter/发票示例.ofd omits object and layer IDs
    // entirely. Both forms load in lenient mode.
    let page = open_page(
        r#"<ofd:Content><ofd:Layer><ofd:PageBlock ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject><ofd:PathObject Boundary="0 0 2 2"><ofd:AbbreviatedData>M 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let objects = &page.layers()[0].objects();
    assert_eq!(objects.len(), 3);
    let PageObject::Path(missing) = &objects[2] else {
        panic!("expected path object");
    };
    assert!(missing.object_id() > 3);
}

#[test]
fn strict_mode_rejects_missing_object_id() {
    let error = open_page_strict(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::InvalidStructure { ref message, .. } if message.contains("object ID is missing")),
        "{error:?}"
    );
}

#[test]
fn counts_layers_groups_and_leaves_against_page_object_limit() {
    let limits = ResourceLimits {
        max_page_objects: 2,
        ..ResourceLimits::default()
    };
    let error = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::LimitExceeded(message) if message.contains("page object count"))
    );
}

#[test]
fn page_object_limit_accepts_exactly_one_layer_one_group_and_one_leaf() {
    let limits = ResourceLimits {
        max_page_objects: 3,
        ..ResourceLimits::default()
    };
    let page = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap();

    assert_eq!(page.layers()[0].object_id(), 1);
    let PageObject::Group(group) = &page.layers()[0].objects()[0] else {
        panic!("expected page group");
    };
    assert_eq!(group.object_id(), 2);
    assert_eq!(group.objects()[0].object_id(), 3);
}

#[test]
fn page_object_limit_rejects_flat_oversize_before_raw_deserialization() {
    let limits = ResourceLimits {
        max_page_objects: 1,
        ..ResourceLimits::default()
    };
    let error = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:TextObject/></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap_err();

    assert!(
        matches!(error, Error::LimitExceeded(message) if message.contains("page object count 2 exceeds limit 1"))
    );
}

#[test]
fn page_object_limit_counts_one_unknown_unit_not_its_payload() {
    let limits = ResourceLimits {
        max_page_objects: 2,
        ..ResourceLimits::default()
    };
    let page = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CustomUnit ID="2"><ofd:Payload><ofd:PathObject/><ofd:ImageObject/></ofd:Payload></ofd:CustomUnit></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap();

    // The whole unknown unit is dropped in lenient mode, so neither it nor
    // its payload contributes page objects.
    assert!(page.layers()[0].objects().is_empty());
}

#[test]
fn path_command_limit_is_cumulative_across_the_page() {
    let limits = ResourceLimits {
        max_path_commands: 3,
        ..ResourceLimits::default()
    };
    let content = format!(
        "<ofd:Content><ofd:Layer ID=\"1\">{}{}</ofd:Layer></ofd:Content>",
        simple_path(2, "M 0 0 L 1 1"),
        simple_path(3, "M 2 2 L 3 3")
    );
    let error = open_page_with_limits(&content, limits).unwrap_err();
    assert!(
        matches!(error, Error::LimitExceeded(message) if message.contains("path command count"))
    );
}

#[test]
fn cumulative_path_command_limit_accepts_the_exact_page_total() {
    let limits = ResourceLimits {
        max_path_commands: 4,
        ..ResourceLimits::default()
    };
    let content = format!(
        "<ofd:Content><ofd:Layer ID=\"1\">{}{}</ofd:Layer></ofd:Content>",
        simple_path(2, "M 0 0 L 1 1"),
        simple_path(3, "M 2 2 L 3 3")
    );
    let page = open_page_with_limits(&content, limits).unwrap();

    assert_eq!(page.layers()[0].objects().len(), 2);
}

#[test]
fn page_block_depth_is_checked_before_recursive_deserialization() {
    let limits = ResourceLimits {
        max_page_block_depth: 2,
        ..ResourceLimits::default()
    };
    let error = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:PageBlock ID="3"><ofd:PageBlock ID="4"><ofd:TextObject ID="5"/></ofd:PageBlock></ofd:PageBlock></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::LimitExceeded(message) if message.contains("page block depth 3 exceeds limit 2"))
    );
}

#[test]
fn page_block_depth_limit_accepts_the_exact_nesting_depth() {
    let limits = ResourceLimits {
        max_page_block_depth: 2,
        ..ResourceLimits::default()
    };
    let page = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:PageBlock ID="3"><ofd:PathObject ID="4" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap();

    let PageObject::Group(outer) = &page.layers()[0].objects()[0] else {
        panic!("expected outer page group");
    };
    assert!(matches!(outer.objects()[0], PageObject::Group(_)));
}

#[test]
fn page_block_depth_ignores_page_block_names_inside_unknown_unit_payload() {
    let limits = ResourceLimits {
        max_page_block_depth: 0,
        ..ResourceLimits::default()
    };
    let page = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CustomUnit ID="2"><ofd:Payload><ofd:PageBlock/></ofd:Payload></ofd:CustomUnit></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap();

    assert!(page.layers()[0].objects().is_empty());
}

#[test]
fn xml_depth_limit_rejects_deep_ignored_payload_before_deserialization() {
    let limits = ResourceLimits {
        max_xml_depth: 5,
        ..ResourceLimits::default()
    };
    let error = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:CompositeObject ID="2"><ofd:Payload><ofd:A><ofd:B/></ofd:A></ofd:Payload></ofd:CompositeObject></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap_err();

    assert!(
        matches!(error, Error::LimitExceeded(message) if message.contains("XML depth 6 exceeds limit 5"))
    );
}

#[test]
fn xml_depth_limit_accepts_the_exact_nesting_depth() {
    let limits = ResourceLimits {
        max_xml_depth: 5,
        ..ResourceLimits::default()
    };
    let page = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap();

    assert_eq!(page.layers()[0].objects()[0].object_id(), 2);
}

#[test]
fn concurrent_lazy_page_initialization_records_one_fallback_warning() {
    let page_without_area = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;
    let document =
        Document::from_bytes(minimal_ofd(page_without_area), LoadOptions::default()).unwrap();
    let worker_count = 8;
    let barrier = Arc::new(Barrier::new(worker_count));
    let workers = (0..worker_count)
        .map(|_| {
            let document = document.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                document.page(0).unwrap().size()
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        assert_eq!(worker.join().unwrap().width, 210.0);
    }
    assert_eq!(document.warnings().len(), 1);
}

#[test]
fn failed_concurrent_page_initialization_does_not_publish_fallback_warnings() {
    let page_without_area = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="NaN 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>
</ofd:Page>"#;
    let document =
        Document::from_bytes(minimal_ofd(page_without_area), LoadOptions::default()).unwrap();
    let worker_count = 8;
    let barrier = Arc::new(Barrier::new(worker_count));
    let workers = (0..worker_count)
        .map(|_| {
            let document = document.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                document.page(0).unwrap_err()
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        assert!(matches!(
            worker.join().unwrap(),
            Error::InvalidValue {
                field: "boundary",
                ..
            }
        ));
    }
    assert!(matches!(
        document.page(0).unwrap_err(),
        Error::InvalidValue {
            field: "boundary",
            ..
        }
    ));
    assert!(document.warnings().is_empty());
}

#[test]
fn lenient_mode_skips_unknown_graphic_units_with_a_warning() {
    // ofdrw ignores page-block children it does not recognize; lenient mode
    // mirrors that and reports the dropped element instead of failing.
    let document = Document::from_bytes(
        minimal_ofd(&page_with(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:VideoObject ID="2"/></ofd:Layer></ofd:Content>"#,
        )),
        LoadOptions::default(),
    )
    .unwrap();
    let page = document.page(0).unwrap();
    assert!(page.layers()[0].objects().is_empty());
    let warnings = document.warnings();
    assert!(
        warnings.iter().any(
            |warning| warning.code == WarningCode::UnknownGraphicUnitSkipped
                && warning.message.contains("VideoObject")
        ),
        "{warnings:?}"
    );
}

#[test]
fn repository_fixture_exposes_paths_text_image_and_unsupported_nodes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    let document = Document::open(path, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();

    fn counts(objects: &[PageObject]) -> (usize, usize, usize, usize) {
        objects.iter().fold(
            (0, 0, 0, 0),
            |(paths, text, images, unsupported), object| match object {
                PageObject::Path(_) => (paths + 1, text, images, unsupported),
                PageObject::Text(_) => (paths, text + 1, images, unsupported),
                PageObject::Image(_) => (paths, text, images + 1, unsupported),
                PageObject::Composite(_) => (paths, text, images, unsupported),
                PageObject::Unsupported(_) => (paths, text, images, unsupported + 1),
                PageObject::Group(group) => {
                    let nested = counts(group.objects());
                    (
                        paths + nested.0,
                        text + nested.1,
                        images + nested.2,
                        unsupported + nested.3,
                    )
                }
            },
        )
    }

    let (paths, text, images, unsupported) =
        page.layers().iter().fold((0, 0, 0, 0), |total, layer| {
            let layer_counts = counts(layer.objects());
            (
                total.0 + layer_counts.0,
                total.1 + layer_counts.1,
                total.2 + layer_counts.2,
                total.3 + layer_counts.3,
            )
        });
    assert!(paths > 0);
    assert_eq!(text, 47);
    assert_eq!(images, 1);
    assert_eq!(unsupported, 0);

    fn find_text(objects: &[PageObject], id: u64) -> Option<&rofd_core::TextObject> {
        objects.iter().find_map(|object| match object {
            PageObject::Text(text) if text.object_id() == id => Some(text),
            PageObject::Group(group) => find_text(group.objects(), id),
            _ => None,
        })
    }
    let repeated = page
        .layers()
        .iter()
        .find_map(|layer| find_text(layer.objects(), 66))
        .expect("fixture TextObject 66");
    assert_eq!(repeated.runs()[0].delta_x(), vec![2.54; 18]);
}

fn path_object_with(attributes: &str) -> String {
    page_with(&format!(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" {attributes}><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
    ))
}

#[test]
fn dash_pattern_accepts_any_positive_count_including_odd() {
    let page = Document::from_bytes(
        minimal_ofd(&path_object_with(r#"DashPattern="3 1 2""#)),
        LoadOptions::default(),
    )
    .unwrap()
    .page(0)
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected path object");
    };
    assert_eq!(path.stroke_style().dash_pattern(), [3.0, 1.0, 2.0]);
}

#[test]
fn dash_pattern_rejects_hex_components_empty_and_non_positive_values() {
    for value in ["#FF #FF", "", "3 0 1", "3 -1"] {
        assert!(
            Document::from_bytes(
                minimal_ofd(&path_object_with(&format!(r#"DashPattern="{value}""#))),
                LoadOptions::default(),
            )
            .unwrap()
            .page(0)
            .is_err(),
            "DashPattern {value:?} must be rejected"
        );
    }
}

#[test]
fn path_object_with_trailing_clips_does_not_consume_the_following_graphic_unit() {
    // ofdrw's reader/path_unstd.ofd templates end PathObjects with Clips and
    // declare StrokeColor before AbbreviatedData; serde-xml-rs 0.6 loses the
    // following sibling unless the payload is extracted standalone.
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" Stroke="true"><ofd:StrokeColor Value="0 0 0"/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData><ofd:Clips TransFlag="false"><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 5 5" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 1 0 L 1 1 C</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips></ofd:PathObject><ofd:PathObject ID="3" Boundary="0 0 10 10"><ofd:AbbreviatedData>M 0 0 L 2 2</ofd:AbbreviatedData></ofd:PathObject><ofd:PathObject ID="4" Boundary="0 0 10 10"><ofd:AbbreviatedData>M 0 0 L 3 3</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let objects = page.layers()[0].objects();
    assert_eq!(objects.len(), 3);
    let PageObject::Path(clipped) = &objects[0] else {
        panic!("expected path object");
    };
    assert_eq!(clipped.object_id(), 2);
    assert_eq!(clipped.clips().len(), 1);
    assert!(matches!(&objects[1], PageObject::Path(path) if path.object_id() == 3));
    assert!(matches!(&objects[2], PageObject::Path(path) if path.object_id() == 4));
}

#[test]
fn subpath_start_operator_s_loads_in_lenient_and_strict_modes() {
    // `S` begins a subpath like `M` (ofdrw's converter/n.ofd uses it).
    let content = r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10"><ofd:AbbreviatedData>S 0 0 L 156 0 C</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#;
    for page in [open_page(content), open_page_strict(content)] {
        let page = page.unwrap();
        let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
            panic!("expected path object");
        };
        assert_eq!(path.path_data().commands().len(), 3);
    }
}

#[test]
fn lenient_mode_defaults_missing_path_boundary_to_the_origin() {
    // ofdrw's converter/intro-数科.ofd omits Boundary on gradient-filled
    // PathObjects; ofdrw draws them untranslated.
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2"><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected path object");
    };
    assert_eq!(
        path.boundary(),
        Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    );
}

#[test]
fn strict_mode_rejects_missing_path_boundary() {
    let error = open_page_strict(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2"><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(
        matches!(
            error,
            Error::InvalidPageObject {
                object_id: 2,
                field: "Boundary",
                ..
            }
        ),
        "expected missing Boundary, got {error:?}"
    );
}

#[test]
fn lenient_mode_accepts_hash_prefixed_hex_color_channels() {
    // ofdrw's converter/n.ofd writes channels like `#ee #20 #25`, which ofdrw
    // parses per token: `#` prefix means hexadecimal (converter/AWTMaker).
    let page = open_page(
        r##"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true"><ofd:FillColor Value="#ee #20 #25"/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"##,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected path object");
    };
    assert_eq!(
        path.fill(),
        Some(Color {
            red: 238,
            green: 32,
            blue: 37,
            alpha: 255,
        })
    );
}

#[test]
fn strict_mode_rejects_hash_prefixed_hex_color_channels() {
    let error = open_page_strict(
        r##"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true"><ofd:FillColor Value="#ee #20 #25"/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"##,
    )
    .unwrap_err();
    assert!(
        matches!(
            error,
            Error::InvalidPageObject {
                object_id: 2,
                field: "FillColor",
                ..
            }
        ),
        "expected invalid FillColor, got {error:?}"
    );
}

#[test]
fn lenient_mode_treats_paint_color_without_value_as_unpainted() {
    // ofdrw's converter/intro-数科.ofd declares gradient-only FillColor
    // elements (AxialShd child, no Value attribute); ofdrw's image converter
    // paints nothing for them.
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true" Stroke="true"><ofd:StrokeColor/><ofd:FillColor/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected path object");
    };
    assert_eq!(path.stroke(), None);
    assert_eq!(path.fill(), None);
}

#[test]
fn strict_mode_rejects_paint_color_without_value() {
    let error = open_page_strict(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10" Fill="true"><ofd:FillColor/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(
        matches!(
            error,
            Error::InvalidPageObject {
                object_id: 2,
                field: "FillColor",
                ..
            }
        ),
        "expected missing FillColor value, got {error:?}"
    );
}
