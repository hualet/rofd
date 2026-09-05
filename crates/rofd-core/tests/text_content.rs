mod support;

use rofd_core::{
    Color, Document, Error, LineCap, LineJoin, LoadOptions, PageObject, ResourceLimits, Transform,
};

fn package(page_objects: &str, resources: &str, limits: ResourceLimits) -> Document {
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData>
  <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
  <ofd:PublicRes>Res.xml</ofd:PublicRes>
</ofd:CommonData><ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1">{page_objects}</ofd:Layer></ofd:Content></ofd:Page>"#
    );
    Document::from_bytes(
        support::ofd_with_document_page_and_entries(
            document,
            &page,
            &[("Doc_0/Res.xml", resources.as_bytes())],
        ),
        LoadOptions {
            limits,
            ..LoadOptions::default()
        },
    )
    .unwrap()
}

fn font_catalog(extra: &str) -> String {
    format!(r#"<Res><Fonts><Font ID="10" FontName="Fixture"/></Fonts>{extra}</Res>"#)
}

fn page_result(
    page_objects: &str,
    resources: &str,
    limits: ResourceLimits,
) -> rofd_core::Result<rofd_core::Page> {
    package(page_objects, resources, limits).page(0)
}

#[test]
fn exposes_validated_text_runs_glyph_maps_clips_and_effective_style() {
    let text = r#"<ofd:TextObject ID="2" Boundary="1 2 30 10" CTM="1 0 0 1 4 5"
      Font="10" Size="3.5" Fill="true" Stroke="true" Alpha="128"
      LineWidth="1.25" Join="Round" Cap="Square" DashOffset="0.5"
      DashPattern="1 2" MiterLimit="4">
      <ofd:FillColor Value="10 20 30" Alpha="128"/>
      <ofd:StrokeColor Value="40 50 60"/>
      <ofd:Clips TransFlag="false"><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 3 3" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 3 0 L 3 3 C</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips>
      <ofd:TextCode X="1" Y="2" DeltaX="1 g 3 0.5" DeltaY="">A中😀Z</ofd:TextCode>
      <ofd:TextCode Y="7">Q</ofd:TextCode>
      <ofd:CGTransform CodePosition="1" CodeCount="2" GlyphCount="2"><ofd:Glyphs>10 11</ofd:Glyphs></ofd:CGTransform>
    </ofd:TextObject>"#;
    let document = package(text, &font_catalog(""), ResourceLimits::default());
    let page = document.page(0).unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!("expected text object");
    };

    assert_eq!(text.object_id(), 2);
    assert_eq!(text.font_id(), 10);
    assert_eq!(text.font_size(), 3.5);
    assert_eq!(
        text.transform(),
        Transform::new(1.0, 0.0, 0.0, 1.0, 4.0, 5.0).unwrap()
    );
    assert_eq!(
        text.fill(),
        Some(Color {
            red: 10,
            green: 20,
            blue: 30,
            alpha: 64
        })
    );
    assert_eq!(
        text.stroke(),
        Some(Color {
            red: 40,
            green: 50,
            blue: 60,
            alpha: 128
        })
    );
    assert_eq!(text.stroke_style().line_width(), 1.25);
    assert_eq!(text.stroke_style().line_join(), LineJoin::Round);
    assert_eq!(text.stroke_style().line_cap(), LineCap::Square);
    assert_eq!(text.stroke_style().dash_offset(), 0.5);
    assert_eq!(text.stroke_style().dash_pattern(), [1.0, 2.0]);
    assert_eq!(text.stroke_style().miter_limit(), 4.0);
    assert_eq!(text.clips().len(), 1);
    assert_eq!(text.runs().len(), 2);
    assert_eq!(text.runs()[0].text(), "A中😀Z");
    assert_eq!((text.runs()[0].x(), text.runs()[0].y()), (1.0, 2.0));
    assert_eq!(text.runs()[0].delta_x(), [1.0, 0.5, 0.5, 0.5]);
    assert_eq!(text.runs()[0].delta_y(), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!((text.runs()[1].x(), text.runs()[1].y()), (1.0, 7.0));
    assert_eq!(text.glyph_maps().len(), 1);
    assert_eq!(text.glyph_maps()[0].code_position(), 1);
    assert_eq!(text.glyph_maps()[0].code_count(), 2);
    assert_eq!(text.glyph_maps()[0].glyphs(), [10, 11]);
}

#[test]
fn draw_parameters_resolve_relative_then_object_reference_then_local_values() {
    let resources = font_catalog(
        r#"<DrawParams>
          <DrawParam ID="20" LineWidth="0.8" Join="Bevel" Cap="Round" DashOffset="1" DashPattern="2 3" MiterLimit="5"><FillColor Value="1 2 3"/><StrokeColor Value="4 5 6"/></DrawParam>
          <DrawParam ID="21" Relative="20" LineWidth="1.5"><FillColor Value="7 8 9"/></DrawParam>
        </DrawParams>"#,
    );
    let objects = r#"<ofd:PathObject ID="2" Boundary="0 0 10 10" DrawParam="21" Fill="true" LineWidth="2.5" Join="Round"><ofd:StrokeColor Value="10 11 12"/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject>"#;
    let document = package(objects, &resources, ResourceLimits::default());
    let page = document.page(0).unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected path");
    };
    assert_eq!(path.line_width(), 2.5);
    assert_eq!(path.stroke_style().line_join(), LineJoin::Round);
    assert_eq!(path.stroke_style().line_cap(), LineCap::Round);
    assert_eq!(path.stroke_style().dash_pattern(), [2.0, 3.0]);
    assert_eq!(path.fill().unwrap().red, 7);
    assert_eq!(path.stroke().unwrap().red, 10);
}

#[test]
fn many_objects_share_one_deep_drawparam_chain() {
    let mut entries = String::from(r#"<DrawParam ID="20" LineWidth="1"/>"#);
    for id in 21..=531 {
        entries.push_str(&format!(r#"<DrawParam ID="{id}" Relative="{}"/>"#, id - 1));
    }
    let resources = font_catalog(&format!("<DrawParams>{entries}</DrawParams>"));
    let objects = (1_000..1_512)
        .map(|id| {
            format!(
                r#"<ofd:PathObject ID="{id}" Boundary="0 0 1 1" DrawParam="531"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>"#
            )
        })
        .collect::<String>();
    let page = package(&objects, &resources, ResourceLimits::default())
        .page(0)
        .unwrap();
    assert_eq!(page.layers()[0].objects().len(), 512);
    for object in page.layers()[0].objects() {
        let PageObject::Path(path) = object else {
            panic!("expected path object");
        };
        assert_eq!(path.line_width(), 1.0);
    }
}

#[test]
fn path_children_are_accepted_in_source_order_inside_a_group() {
    let objects = r#"<ofd:PageBlock ID="2"><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:StrokeColor Value="1 2 3"/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject><ofd:PathObject ID="4" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock>"#;
    let document = package(objects, &font_catalog(""), ResourceLimits::default());
    let page = document.page(0).unwrap();
    let PageObject::Group(group) = &page.layers()[0].objects()[0] else {
        panic!("expected group");
    };
    assert_eq!(group.objects().len(), 2);
}

#[test]
fn text_object_does_not_consume_the_following_group() {
    let objects = r#"<ofd:TextObject ID="2" Boundary="0 0 5 5" Font="10" Size="3"><ofd:FillColor Value="1 2 3"/><ofd:TextCode DeltaX="3 3" X="0" Y="3">名称:</ofd:TextCode></ofd:TextObject><ofd:PageBlock ID="3"><ofd:PathObject ID="4" Boundary="0 0 1 1"><ofd:StrokeColor Value="1 2 3"/><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject><ofd:PathObject ID="5" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData></ofd:PathObject></ofd:PageBlock>"#;
    let document = package(objects, &font_catalog(""), ResourceLimits::default());
    let page = document.page(0).unwrap();
    assert_eq!(page.layers()[0].objects().len(), 2);
}

#[test]
fn text_defaults_and_per_axis_origin_inheritance_are_explicit() {
    let objects = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="1" Y="2">A</ofd:TextCode><ofd:TextCode X="3">B</ofd:TextCode><ofd:TextCode Y="4">C</ofd:TextCode></ofd:TextObject>"#;
    let page = package(objects, &font_catalog(""), ResourceLimits::default())
        .page(0)
        .unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!()
    };
    assert_eq!(text.transform(), Transform::IDENTITY);
    assert_eq!(text.fill(), Some(Color::BLACK));
    assert_eq!(text.stroke(), None);
    assert_eq!((text.runs()[1].x(), text.runs()[1].y()), (3.0, 2.0));
    assert_eq!((text.runs()[2].x(), text.runs()[2].y()), (3.0, 4.0));
}

#[test]
fn deltas_use_unicode_scalars_zero_fill_and_standard_g_character_span() {
    let objects = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0" DeltaX="g 4 2.54" DeltaY="1">中😀AB</ofd:TextCode></ofd:TextObject>"#;
    let page = package(objects, &font_catalog(""), ResourceLimits::default())
        .page(0)
        .unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!()
    };
    assert_eq!(text.runs()[0].delta_x(), [2.54, 2.54, 2.54, 2.54]);
    assert_eq!(text.runs()[0].delta_y(), [1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn text_code_preserves_whitespace_entities_cdata_and_scalar_accounting() {
    let objects = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0" DeltaX="g 8 1">  edge  </ofd:TextCode><ofd:TextCode DeltaX="g 3 2">   </ofd:TextCode><ofd:TextCode DeltaX="g 5 3">A&amp;<![CDATA[ B]]>C</ofd:TextCode></ofd:TextObject>"#;
    let limits = ResourceLimits {
        max_text_characters_per_page: 16,
        ..ResourceLimits::default()
    };
    let page = page_result(objects, &font_catalog(""), limits).unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!("expected text object");
    };
    assert_eq!(text.runs()[0].text(), "  edge  ");
    assert_eq!(text.runs()[1].text(), "   ");
    assert_eq!(text.runs()[2].text(), "A& BC");
    assert_eq!(text.runs()[0].delta_x(), [1.0; 8]);
    assert_eq!(text.runs()[1].delta_x(), [2.0; 3]);
    assert_eq!(text.runs()[2].delta_x(), [3.0; 5]);

    let one_over = ResourceLimits {
        max_text_characters_per_page: 15,
        ..ResourceLimits::default()
    };
    assert!(matches!(
        page_result(objects, &font_catalog(""), one_over),
        Err(Error::LimitExceeded(_))
    ));
}

#[test]
fn text_required_fields_and_local_scalars_have_object_context() {
    let cases = [
        (
            r#"<ofd:TextObject ID="2" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Boundary",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Size="2"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Font",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Size",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2" Stroke="maybe"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Stroke",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2" Fill="maybe"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Fill",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2" Alpha="256"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Alpha",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2"><ofd:Clips TransFlag="maybe"><ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "TransFlag",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2"><ofd:Clips><ofd:Clip><ofd:Area CTM="bad"><ofd:Path Boundary="0 0 1 1" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "Area.CTM",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2"><ofd:FillColor/><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "FillColor",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2" Stroke="true"><ofd:StrokeColor/><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#,
            "StrokeColor",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A</ofd:TextCode><ofd:CGTransform><ofd:Glyphs>1</ofd:Glyphs></ofd:CGTransform></ofd:TextObject>"#,
            "CodePosition",
        ),
        (
            r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A</ofd:TextCode><ofd:CGTransform CodePosition="0"/></ofd:TextObject>"#,
            "Glyphs",
        ),
    ];
    for (object, expected_field) in cases {
        let error = page_result(object, &font_catalog(""), ResourceLimits::default()).unwrap_err();
        assert!(
            matches!(error, Error::InvalidPageObject { object_id: 2, field, ref path, .. } if field == expected_field && path.ends_with("Content.xml")),
            "expected {expected_field}, got {error:?}"
        );
    }
}

#[test]
fn malformed_text_object_does_not_consume_following_siblings() {
    let objects = r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject>"#;
    let error = page_result(objects, &font_catalog(""), ResourceLimits::default()).unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidPageObject {
            object_id: 2,
            field: "Size",
            ..
        }
    ));
}

#[test]
fn truly_malformed_page_xml_remains_an_xml_error() {
    let objects = r#"<ofd:TextObject ID="2" Boundary="0 0 1 1" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A</ofd:TextObject>"#;
    assert!(matches!(
        page_result(objects, &font_catalog(""), ResourceLimits::default()),
        Err(Error::Xml { ref path, .. }) if path.ends_with("Content.xml")
    ));
}

#[test]
fn rejects_invalid_origins_delta_grammar_and_nonfinite_values_with_context() {
    let cases = [
        (
            r#"<ofd:TextCode Y="0">AB</ofd:TextCode>"#,
            "TextCode origin",
        ),
        (
            r#"<ofd:TextCode X="0" Y="0" DeltaX="1 2 3">AB</ofd:TextCode>"#,
            "DeltaX",
        ),
        (
            r#"<ofd:TextCode X="0" Y="0" DeltaX="G 2 1">AB</ofd:TextCode>"#,
            "DeltaX",
        ),
        (
            r#"<ofd:TextCode X="0" Y="0" DeltaX="g 0 1">AB</ofd:TextCode>"#,
            "DeltaX",
        ),
        (
            r#"<ofd:TextCode X="NaN" Y="0">A</ofd:TextCode>"#,
            "TextCode.X",
        ),
        (
            r#"<ofd:TextCode X="0" Y="0" DeltaY="inf">AB</ofd:TextCode>"#,
            "DeltaY",
        ),
    ];
    for (run, expected_field) in cases {
        let object = format!(
            r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2">{run}</ofd:TextObject>"#
        );
        let error = page_result(&object, &font_catalog(""), ResourceLimits::default()).unwrap_err();
        assert!(
            matches!(error, Error::InvalidPageObject { object_id: 2, field, ref path, .. } if field == expected_field && path.ends_with("Content.xml")),
            "{error:?}"
        );
    }
}

#[test]
fn cgtransform_defaults_and_validation_use_the_concatenated_scalar_stream() {
    let valid = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A😀</ofd:TextCode><ofd:TextCode>B</ofd:TextCode><ofd:CGTransform CodePosition="1"><ofd:Glyphs>42</ofd:Glyphs></ofd:CGTransform></ofd:TextObject>"#;
    let page = package(valid, &font_catalog(""), ResourceLimits::default())
        .page(0)
        .unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!()
    };
    assert_eq!(text.glyph_maps()[0].code_count(), 1);
    assert_eq!(text.glyph_maps()[0].glyphs(), [42]);

    let invalid = [
        (
            r#"<ofd:CGTransform CodePosition="3"><ofd:Glyphs>1</ofd:Glyphs></ofd:CGTransform>"#,
            "CodePosition",
        ),
        (
            r#"<ofd:CGTransform CodePosition="0" CodeCount="0"><ofd:Glyphs>1</ofd:Glyphs></ofd:CGTransform>"#,
            "CodeCount",
        ),
        (
            r#"<ofd:CGTransform CodePosition="0" GlyphCount="2"><ofd:Glyphs>1</ofd:Glyphs></ofd:CGTransform>"#,
            "GlyphCount",
        ),
        (r#"<ofd:CGTransform CodePosition="0"/>"#, "Glyphs"),
        (
            r#"<ofd:CGTransform CodePosition="0"><ofd:Glyphs>-1</ofd:Glyphs></ofd:CGTransform>"#,
            "Glyphs",
        ),
        (
            r#"<ofd:CGTransform CodePosition="0"/><ofd:CGTransform CodePosition="0"><ofd:Glyphs>2</ofd:Glyphs></ofd:CGTransform>"#,
            "Glyphs",
        ),
    ];
    for (maps, expected_field) in invalid {
        let object = format!(
            r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A😀B</ofd:TextCode>{maps}</ofd:TextObject>"#
        );
        let error = page_result(&object, &font_catalog(""), ResourceLimits::default()).unwrap_err();
        assert!(
            matches!(error, Error::InvalidPageObject { object_id: 2, field, .. } if field == expected_field),
            "{error:?}"
        );
    }
    let overlap = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">ABC</ofd:TextCode><ofd:CGTransform CodePosition="0" CodeCount="2"><ofd:Glyphs>1</ofd:Glyphs></ofd:CGTransform><ofd:CGTransform CodePosition="1"><ofd:Glyphs>2</ofd:Glyphs></ofd:CGTransform></ofd:TextObject>"#;
    let error = page_result(overlap, &font_catalog(""), ResourceLimits::default()).unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidPageObject {
            field: "CodePosition",
            ..
        }
    ));
}

#[test]
fn text_limits_have_exact_unicode_glyph_and_expansion_boundaries() {
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">中😀A</ofd:TextCode></ofd:TextObject>"#;
    for (exact, over) in [
        (
            ResourceLimits {
                max_text_characters_per_page: 3,
                ..ResourceLimits::default()
            },
            ResourceLimits {
                max_text_characters_per_page: 2,
                ..ResourceLimits::default()
            },
        ),
        (
            ResourceLimits {
                max_glyphs_per_page: 3,
                ..ResourceLimits::default()
            },
            ResourceLimits {
                max_glyphs_per_page: 2,
                ..ResourceLimits::default()
            },
        ),
        (
            ResourceLimits {
                max_text_expansion_entries: 7,
                ..ResourceLimits::default()
            },
            ResourceLimits {
                max_text_expansion_entries: 6,
                ..ResourceLimits::default()
            },
        ),
    ] {
        assert!(page_result(object, &font_catalog(""), exact).is_ok());
        assert!(matches!(
            page_result(object, &font_catalog(""), over),
            Err(Error::LimitExceeded(_))
        ));
    }
}

#[test]
fn text_run_and_glyph_map_nodes_have_exact_expansion_budget_boundaries() {
    let empty_runs = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0"/><ofd:TextCode/><ofd:TextCode/><ofd:TextCode/></ofd:TextObject>"#;
    for (limit, succeeds) in [(4, true), (3, false)] {
        let result = page_result(
            empty_runs,
            &font_catalog(""),
            ResourceLimits {
                max_text_expansion_entries: limit,
                ..ResourceLimits::default()
            },
        );
        assert_eq!(result.is_ok(), succeeds, "empty-run limit {limit}");
    }

    let mappings = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">ABCD</ofd:TextCode><ofd:CGTransform CodePosition="0"><ofd:Glyphs>10</ofd:Glyphs></ofd:CGTransform><ofd:CGTransform CodePosition="1"><ofd:Glyphs>11</ofd:Glyphs></ofd:CGTransform><ofd:CGTransform CodePosition="2"><ofd:Glyphs>12</ofd:Glyphs></ofd:CGTransform><ofd:CGTransform CodePosition="3"><ofd:Glyphs>13</ofd:Glyphs></ofd:CGTransform></ofd:TextObject>"#;
    for (limit, succeeds) in [(17, true), (16, false)] {
        let result = page_result(
            mappings,
            &font_catalog(""),
            ResourceLimits {
                max_text_expansion_entries: limit,
                ..ResourceLimits::default()
            },
        );
        assert_eq!(result.is_ok(), succeeds, "mapping limit {limit}");
    }
}

#[test]
fn text_run_nodes_exceeding_the_expansion_budget_fail_during_xml_preflight() {
    let empty_runs = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0"/><ofd:TextCode/><ofd:TextCode/></ofd:TextObject>"#;
    let error = page_result(
        empty_runs,
        &font_catalog(""),
        ResourceLimits {
            max_text_expansion_entries: 2,
            ..ResourceLimits::default()
        },
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::LimitExceeded(ref message) if message.contains("XML text expansion node count 3 exceeds limit 2")),
        "expected preflight text-node limit, got {error:?}"
    );
}

#[test]
fn cgtransform_replaces_character_glyphs_instead_of_double_counting_them() {
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">ABC</ofd:TextCode><ofd:CGTransform CodePosition="0" CodeCount="2" GlyphCount="1"><ofd:Glyphs>42</ofd:Glyphs></ofd:CGTransform></ofd:TextObject>"#;
    let exact = ResourceLimits {
        max_glyphs_per_page: 2,
        ..ResourceLimits::default()
    };
    assert!(page_result(object, &font_catalog(""), exact).is_ok());

    let one_over = ResourceLimits {
        max_glyphs_per_page: 1,
        ..ResourceLimits::default()
    };
    assert!(matches!(
        page_result(object, &font_catalog(""), one_over),
        Err(Error::LimitExceeded(_))
    ));
}

#[test]
fn cgtransform_overlap_index_preserves_source_order() {
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">ABC</ofd:TextCode><ofd:CGTransform CodePosition="2"><ofd:Glyphs>20</ofd:Glyphs></ofd:CGTransform><ofd:CGTransform CodePosition="0"><ofd:Glyphs>10</ofd:Glyphs></ofd:CGTransform></ofd:TextObject>"#;
    let page = package(object, &font_catalog(""), ResourceLimits::default())
        .page(0)
        .unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!("expected text object");
    };
    assert_eq!(
        text.glyph_maps()
            .iter()
            .map(|mapping| mapping.code_position())
            .collect::<Vec<_>>(),
        [2, 0]
    );
}

#[test]
fn unknown_text_children_fail_closed() {
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2"><ofd:TextCode X="0" Y="0">A</ofd:TextCode><ofd:Unknown/></ofd:TextObject>"#;
    assert!(
        matches!(page_result(object, &font_catalog(""), ResourceLimits::default()), Err(Error::InvalidStructure { message, .. }) if message.contains("Unknown"))
    );
}

#[test]
fn text_uses_drawparam_paint_then_local_color_and_style_precedence() {
    let resources = font_catalog(
        r#"<DrawParams><DrawParam ID="20" LineWidth="1" Join="Bevel"><FillColor Value="1 2 3"/><StrokeColor Value="4 5 6"/></DrawParam></DrawParams>"#,
    );
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2" DrawParam="20" Stroke="true" LineWidth="2"><ofd:FillColor Value="7 8 9"/><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#;
    let page = package(object, &resources, ResourceLimits::default())
        .page(0)
        .unwrap();
    let PageObject::Text(text) = &page.layers()[0].objects()[0] else {
        panic!()
    };
    assert_eq!(text.fill().unwrap().red, 7);
    assert_eq!(text.stroke().unwrap().red, 4);
    assert_eq!(text.stroke_style().line_width(), 2.0);
    assert_eq!(text.stroke_style().line_join(), LineJoin::Bevel);
}

#[test]
fn drawparam_unknown_relative_cycles_and_style_values_fail_lazily() {
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2" DrawParam="20"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#;
    let catalogs = [
        font_catalog(r#"<DrawParams><DrawParam ID="20" Relative="99"/></DrawParams>"#),
        font_catalog(
            r#"<DrawParams><DrawParam ID="20" Relative="21"/><DrawParam ID="21" Relative="20"/></DrawParams>"#,
        ),
        font_catalog(r#"<DrawParams><DrawParam ID="20" LineWidth="0"/></DrawParams>"#),
        font_catalog(r#"<DrawParams><DrawParam ID="20" Join="round"/></DrawParams>"#),
        font_catalog(r#"<DrawParams><DrawParam ID="20" Cap="Triangle"/></DrawParams>"#),
        font_catalog(r#"<DrawParams><DrawParam ID="20" DashOffset="-1"/></DrawParams>"#),
        font_catalog(r#"<DrawParams><DrawParam ID="20" DashPattern="1 0"/></DrawParams>"#),
        font_catalog(r#"<DrawParams><DrawParam ID="20" MiterLimit="NaN"/></DrawParams>"#),
    ];
    for (index, catalog) in catalogs.into_iter().enumerate() {
        let error = page_result(object, &catalog, ResourceLimits::default()).unwrap_err();
        let expected = match index {
            0 => {
                matches!(error, Error::InvalidPageObject { object_id: 2, field: "DrawParam", ref path, .. } if path.ends_with("Content.xml"))
            }
            1 => {
                matches!(error, Error::InvalidStructure { ref path, .. } if path.ends_with("Res.xml"))
            }
            _ => {
                matches!(error, Error::InvalidResource { ref path, .. } if path.ends_with("Res.xml"))
            }
        };
        assert!(expected, "{error:?}");
    }
}

#[test]
fn drawparam_missing_color_value_stays_a_resource_error() {
    let object = r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2" DrawParam="20"><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#;
    for (color, expected_field) in [
        ("<FillColor/>", "FillColor"),
        ("<StrokeColor/>", "StrokeColor"),
    ] {
        let catalog = font_catalog(&format!(
            "<DrawParams><DrawParam ID=\"20\">{color}</DrawParam></DrawParams>"
        ));
        let error = page_result(object, &catalog, ResourceLimits::default()).unwrap_err();
        assert!(
            matches!(error, Error::InvalidResource { object_id: Some(20), field, ref path, .. } if field == expected_field && path.ends_with("Res.xml")),
            "expected {expected_field}, got {error:?}"
        );
    }
}

#[test]
fn object_local_stroke_style_rejects_invalid_values() {
    for attribute in [
        "LineWidth=\"0\"",
        "Join=\"round\"",
        "Cap=\"Triangle\"",
        "DashOffset=\"-1\"",
        "DashPattern=\"\"",
        "DashPattern=\"1 inf\"",
        "MiterLimit=\"0\"",
    ] {
        let object = format!(
            r#"<ofd:TextObject ID="2" Boundary="0 0 9 9" Font="10" Size="2" {attribute}><ofd:TextCode X="0" Y="0">A</ofd:TextCode></ofd:TextObject>"#
        );
        assert!(matches!(
            page_result(&object, &font_catalog(""), ResourceLimits::default()),
            Err(Error::InvalidPageObject { object_id: 2, .. })
        ));
    }
}

#[test]
fn repeated_nested_template_text_accounting_is_cache_order_independent() {
    let document = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes><ofd:TemplatePage ID="50" BaseLoc="Templates/T.xml"/></ofd:CommonData><ofd:Pages><ofd:Page ID="1" BaseLoc="Pages/One.xml"/><ofd:Page ID="2" BaseLoc="Pages/Two.xml"/></ofd:Pages></ofd:Document>"#;
    let one = br#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Template TemplateID="50"/></ofd:Page>"#;
    let two = br#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Template TemplateID="50"/><ofd:Template TemplateID="50"/></ofd:Page>"#;
    let template = br#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="60"><ofd:PageBlock ID="61"><ofd:TextObject ID="62" Boundary="0 0 2 2" Font="10" Size="1"><ofd:TextCode X="0" Y="0">AB</ofd:TextCode></ofd:TextObject></ofd:PageBlock></ofd:Layer></ofd:Content></ofd:Page>"#;
    let catalog = br#"<Res><Fonts><Font ID="10" FontName="Fixture"/></Fonts></Res>"#;
    let bytes = support::ofd_with_document_and_entries(
        document,
        &[
            ("Doc_0/Pages/One.xml", one.as_slice()),
            ("Doc_0/Pages/Two.xml", two.as_slice()),
            ("Doc_0/Templates/T.xml", template.as_slice()),
            ("Doc_0/Res.xml", catalog.as_slice()),
        ],
    );

    let exact = Document::from_bytes(
        bytes.clone(),
        LoadOptions {
            limits: ResourceLimits {
                max_text_characters_per_page: 4,
                max_glyphs_per_page: 4,
                max_text_expansion_entries: 10,
                ..ResourceLimits::default()
            },
            ..LoadOptions::default()
        },
    )
    .unwrap();
    assert!(exact.page(0).is_ok());
    assert!(exact.page(1).is_ok());

    let expansion_over = LoadOptions {
        limits: ResourceLimits {
            max_text_expansion_entries: 9,
            ..ResourceLimits::default()
        },
        ..LoadOptions::default()
    };
    let warmed = Document::from_bytes(bytes.clone(), expansion_over.clone()).unwrap();
    assert!(warmed.page(0).is_ok());
    assert!(matches!(warmed.page(1), Err(Error::LimitExceeded(_))));
    let cold = Document::from_bytes(bytes.clone(), expansion_over).unwrap();
    assert!(matches!(cold.page(1), Err(Error::LimitExceeded(_))));

    let over_options = LoadOptions {
        limits: ResourceLimits {
            max_text_characters_per_page: 3,
            ..ResourceLimits::default()
        },
        ..LoadOptions::default()
    };
    let warmed = Document::from_bytes(bytes.clone(), over_options.clone()).unwrap();
    assert!(warmed.page(0).is_ok());
    assert!(matches!(warmed.page(1), Err(Error::LimitExceeded(_))));
    let cold = Document::from_bytes(bytes, over_options).unwrap();
    assert!(matches!(cold.page(1), Err(Error::LimitExceeded(_))));
}
