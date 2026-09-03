mod support;

use std::path::PathBuf;

use rofd_core::{
    Color, Document, Error, FillRule, LayerType, LoadOptions, PageObject, Rect, ResourceLimits,
    Transform, UnsupportedObjectKind,
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
fn enabled_fill_without_color_defaults_to_black_with_object_alpha() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="0 0 1 1" Stroke="false" Fill="true" Alpha="128">
  <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let PageObject::Path(path) = &page.layers()[0].objects()[0] else {
        panic!("expected a path object");
    };

    assert_eq!(
        path.fill(),
        Some(Color {
            alpha: 128,
            ..Color::BLACK
        })
    );
}

#[test]
fn nested_page_blocks_preserve_exact_source_order() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1">
  <ofd:TextObject ID="2"/>
  <ofd:PageBlock ID="3">
    <ofd:ImageObject ID="4"/>
    <ofd:PageBlock ID="5"><ofd:CompositeObject ID="6"/></ofd:PageBlock>
    <ofd:TextObject ID="7"/>
  </ofd:PageBlock>
  <ofd:ImageObject ID="8"/>
</ofd:Layer></ofd:Content>"#,
    )
    .unwrap();

    let objects = page.layers()[0].objects();
    assert!(matches!(objects[0], PageObject::Unsupported(_)));
    let PageObject::Group(group) = &objects[1] else {
        panic!("expected a page group");
    };
    assert_eq!(group.object_id(), 3);
    assert_eq!(group.objects().len(), 3);
    assert!(matches!(group.objects()[0], PageObject::Unsupported(_)));
    let PageObject::Group(nested) = &group.objects()[1] else {
        panic!("expected a nested page group");
    };
    assert_eq!(nested.object_id(), 5);
    assert_eq!(nested.objects()[0].object_id(), 6);
    assert_eq!(group.objects()[2].object_id(), 7);
    assert_eq!(objects[2].object_id(), 8);
}

#[test]
fn known_unsupported_objects_remain_explicit() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1">
  <ofd:TextObject ID="2"><ofd:TextCode>hello</ofd:TextCode></ofd:TextObject>
  <ofd:ImageObject ID="3" ResourceID="9"/>
  <ofd:CompositeObject ID="4" ResourceID="10"/>
</ofd:Layer></ofd:Content>"#,
    )
    .unwrap();
    let objects = page.layers()[0].objects();
    for (index, (id, kind)) in [
        (2, UnsupportedObjectKind::Text),
        (3, UnsupportedObjectKind::Image),
        (4, UnsupportedObjectKind::Composite),
    ]
    .into_iter()
    .enumerate()
    {
        let PageObject::Unsupported(object) = &objects[index] else {
            panic!("expected an unsupported object");
        };
        assert_eq!(object.object_id(), id);
        assert_eq!(object.kind(), kind);
    }
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
            Error::InvalidValue {
                field: "line width",
                ..
            }
        ));
    }
}

#[test]
fn rejects_invalid_stroke_and_fill_attributes() {
    for (attribute, field) in [("Stroke=\"yes\"", "stroke"), ("Fill=\"1\"", "fill")] {
        let content = format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 1 1" {attribute}><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#
        );
        let error = open_page(&content).unwrap_err();
        assert!(
            matches!(error, Error::InvalidValue { field: actual, .. } if actual == field),
            "expected invalid {field}, got {error:?}"
        );
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
            "Boundary=\"0 0 -1 1\"",
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
        assert!(
            matches!(error, Error::InvalidValue { field: actual, .. } if actual == field),
            "expected invalid {field}, got {error:?}"
        );
    }
}

#[test]
fn rejects_duplicate_ids_anywhere_on_a_page() {
    let error = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:TextObject ID="3"/></ofd:PageBlock><ofd:PathObject ID="3" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::InvalidStructure { message, .. } if message.contains("duplicate object ID 3"))
    );
}

#[test]
fn counts_layers_groups_and_leaves_against_page_object_limit() {
    let limits = ResourceLimits {
        max_page_objects: 2,
        ..ResourceLimits::default()
    };
    let error = open_page_with_limits(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:TextObject ID="3"/></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
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
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:TextObject ID="3"/></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
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
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:PageBlock ID="3"><ofd:TextObject ID="4"/></ofd:PageBlock></ofd:PageBlock></ofd:Layer></ofd:Content>"#,
        limits,
    )
    .unwrap();

    let PageObject::Group(outer) = &page.layers()[0].objects()[0] else {
        panic!("expected outer page group");
    };
    assert!(matches!(outer.objects()[0], PageObject::Group(_)));
}

#[test]
fn rejects_unknown_graphic_units_instead_of_discarding_them() {
    let error = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:VideoObject ID="2"/></ofd:Layer></ofd:Content>"#,
    )
    .unwrap_err();
    assert!(
        matches!(error, Error::InvalidStructure { message, .. } if message.contains("VideoObject"))
    );
}

#[test]
fn repository_fixture_exposes_paths_and_unsupported_nodes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../learning/test.ofd");
    let document = Document::open(path, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();

    fn counts(objects: &[PageObject]) -> (usize, usize) {
        objects
            .iter()
            .fold((0, 0), |(paths, unsupported), object| match object {
                PageObject::Path(_) => (paths + 1, unsupported),
                PageObject::Unsupported(_) => (paths, unsupported + 1),
                PageObject::Group(group) => {
                    let nested = counts(group.objects());
                    (paths + nested.0, unsupported + nested.1)
                }
            })
    }

    let (paths, unsupported) = page.layers().iter().fold((0, 0), |total, layer| {
        let layer_counts = counts(layer.objects());
        (total.0 + layer_counts.0, total.1 + layer_counts.1)
    });
    assert!(paths > 0);
    assert!(unsupported > 0);
}
