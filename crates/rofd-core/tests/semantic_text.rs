mod support;

use rofd_core::{
    Document, Error, LoadOptions, Rect, ResourceLimits, TextCharFlags, TextGeometryPrecision,
};

const FONT_CATALOG: &str = r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="10" FontName="Fixture"/></ofd:Fonts></ofd:Res>"#;

fn document_with_page(
    page_xml: &str,
    common: &str,
    tail: &str,
    entries: &[(&str, &[u8])],
) -> Document {
    document_with_page_options(page_xml, common, tail, entries, LoadOptions::default())
}

fn document_with_page_options(
    page_xml: &str,
    common: &str,
    tail: &str,
    entries: &[(&str, &[u8])],
    options: LoadOptions,
) -> Document {
    let document_xml = format!(
        r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData>
<ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
<ofd:PublicRes>Res.xml</ofd:PublicRes>{common}</ofd:CommonData>
<ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>{tail}</ofd:Document>"#
    );
    let entries = [("Doc_0/Res.xml", FONT_CATALOG.as_bytes())]
        .into_iter()
        .chain(entries.iter().copied())
        .collect::<Vec<_>>();
    Document::from_bytes(
        support::ofd_with_document_page_and_entries(&document_xml, page_xml, &entries),
        options,
    )
    .unwrap()
}

fn page_with_text(content: &str) -> rofd_core::Page {
    let page_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1">{content}</ofd:Layer></ofd:Content>
</ofd:Page>"#
    );
    document_with_page(&page_xml, "", "", &[]).page(0).unwrap()
}

#[test]
fn page_text_exposes_utf8_ranges_source_ids_and_conservative_geometry() {
    const SYNTHESIZED_SEPARATOR_BITS: u32 = TextCharFlags::SYNTHESIZED_SEPARATOR.bits();
    const HAS_SYNTHESIZED_SEPARATOR: bool =
        TextCharFlags::SYNTHESIZED_SEPARATOR.contains(TextCharFlags::SYNTHESIZED_SEPARATOR);

    assert_eq!(SYNTHESIZED_SEPARATOR_BITS, 1);
    const { assert!(HAS_SYNTHESIZED_SEPARATOR) };

    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="10 20 30 8" Font="10" Size="4">
  <ofd:TextCode X="1" Y="5" DeltaX="3 4">A中B</ofd:TextCode>
</ofd:TextObject>
<ofd:TextObject ID="3" Boundary="10 35 30 8" Font="10" Size="4">
  <ofd:TextCode X="1" Y="5">尾</ofd:TextCode>
</ofd:TextObject>"#,
    );

    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "A中B\n尾");

    let characters = text.characters();
    assert_eq!(characters.len(), 5);
    assert_eq!(characters[0].utf8_range(), 0..1);
    assert_eq!(characters[1].utf8_range(), 1..4);
    assert_eq!(characters[2].utf8_range(), 4..5);
    assert_eq!(characters[3].utf8_range(), 5..6);
    assert_eq!(characters[4].utf8_range(), 6..9);

    assert_eq!(characters[1].object_id(), Some(2));
    assert_eq!(
        characters[1].geometry_precision(),
        TextGeometryPrecision::Conservative
    );
    let middle_rect = characters[1].rect_mm().expect("middle character geometry");
    assert!(middle_rect.width > 0.0);
    assert!(middle_rect.height > 0.0);

    let separator = &characters[3];
    assert!(separator
        .flags()
        .contains(TextCharFlags::SYNTHESIZED_SEPARATOR));
    assert_eq!(separator.object_id(), None);
    assert_eq!(separator.rect_mm(), None);
}

fn text_object(id: u64, text: &str) -> String {
    format!(
        r#"<ofd:TextObject ID="{id}" Boundary="0 0 20 8" Font="10" Size="4"><ofd:TextCode X="1" Y="5">{text}</ofd:TextCode></ofd:TextObject>"#
    )
}

#[test]
fn semantic_text_recurses_in_effective_paint_order_and_maps_to_page_mm() {
    let page = page_with_text(
        r#"<ofd:PageBlock ID="2"><ofd:TextObject ID="3" Boundary="10 20 20 8" CTM="1 0 0 1 2 3" Font="10" Size="4"><ofd:TextCode X="1" Y="5" DeltaX="4">甲乙</ofd:TextCode></ofd:TextObject></ofd:PageBlock>"#,
    );
    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "甲乙");
    let rect = text.characters()[0].rect_mm().unwrap();
    assert_eq!((rect.x, rect.y), (13.0, 24.0));
    assert!(rect.width > 0.0 && rect.height > 0.0);
}

#[test]
fn repeated_page_handles_share_one_published_semantic_index() {
    let page = page_with_text(&text_object(2, "cache"));
    let clone = page.clone();
    assert!(std::ptr::eq(page.text().unwrap(), clone.text().unwrap()));
    assert_eq!(page.text().unwrap().as_str(), "cache");
}

#[test]
fn concurrent_cold_page_handles_publish_one_complete_index() {
    let page = page_with_text(&text_object(2, "并发 cache"));
    let barrier = std::sync::Barrier::new(8);
    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                let page = &page;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    let text = page.text().unwrap();
                    assert_eq!(text.as_str(), "并发 cache");
                    assert_eq!(text.characters().len(), 8);
                    text
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            assert!(std::ptr::eq(handle.join().unwrap(), page.text().unwrap()));
        }
    });
}

#[test]
fn explicit_zero_negative_and_vertical_deltas_preserve_source_run_positions() {
    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="0 0 30 20" Font="10" Size="4">
<ofd:TextCode X="10" Y="5" DeltaX="0 -3 2" DeltaY="2 -1 0">ABC</ofd:TextCode>
<ofd:TextCode X="2" Y="10">DE</ofd:TextCode></ofd:TextObject>"#,
    );
    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "ABCDE");
    let rects = text
        .characters()
        .iter()
        .map(|ch| ch.rect_mm().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rects,
        vec![
            Rect {
                x: 10.0,
                y: 1.0,
                width: 4.0,
                height: 6.0
            },
            Rect {
                x: 7.0,
                y: 2.0,
                width: 4.0,
                height: 5.0
            },
            Rect {
                x: 7.0,
                y: 2.0,
                width: 4.0,
                height: 4.0
            },
            Rect {
                x: 2.0,
                y: 6.0,
                width: 4.0,
                height: 4.0
            },
            Rect {
                x: 6.0,
                y: 6.0,
                width: 4.0,
                height: 4.0
            },
        ]
    );
}

#[test]
fn source_whitespace_is_flagged_and_empty_objects_do_not_add_separators() {
    let page = page_with_text(
        &[
            text_object(2, ""),
            text_object(3, "A &#x9;&#xA;中"),
            text_object(4, ""),
            text_object(5, "B"),
            text_object(6, ""),
        ]
        .join(""),
    );
    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "A \t\n中\nB");
    for (index, ch) in text.characters().iter().enumerate() {
        assert_eq!(
            ch.flags().contains(TextCharFlags::WHITESPACE),
            (1..=3).contains(&index)
        );
        assert_eq!(
            ch.flags().contains(TextCharFlags::SYNTHESIZED_SEPARATOR),
            index == 5
        );
        assert_eq!(ch.geometry_precision(), TextGeometryPrecision::Conservative);
    }
}

fn composite_page(transform: &str, inner_transform: &str) -> rofd_core::Page {
    let catalog = format!(
        r#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CompositeGraphicUnits>
<ofd:CompositeGraphicUnit ID="20" Width="20" Height="20"><ofd:Content ID="21">
<ofd:CompositeObject ID="22" ResourceID="30" Boundary="3 4 20 20" CTM="{inner_transform}"/>
</ofd:Content></ofd:CompositeGraphicUnit>
<ofd:CompositeGraphicUnit ID="30" Width="20" Height="20"><ofd:Content ID="31">
<ofd:PageBlock ID="32"><ofd:TextObject ID="33" Boundary="5 6 20 8" CTM="1 0 0 1 2 3" Font="10" Size="4"><ofd:TextCode X="1" Y="5">X</ofd:TextCode></ofd:TextObject></ofd:PageBlock>
</ofd:Content></ofd:CompositeGraphicUnit></ofd:CompositeGraphicUnits></ofd:Res>"#
    );
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="1">
<ofd:CompositeObject ID="2" ResourceID="20" Boundary="10 20 40 40" CTM="{transform}"/>
<ofd:TextObject ID="3" Boundary="0 0 20 8" CTM="1 0 0 0 0 0" Font="10" Size="4"><ofd:TextCode X="1" Y="5">hidden</ofd:TextCode></ofd:TextObject>
{}</ofd:Layer></ofd:Content></ofd:Page>"#,
        text_object(4, "tail")
    );
    document_with_page(
        &page,
        "<ofd:DocumentRes>Composite.xml</ofd:DocumentRes>",
        "",
        &[("Doc_0/Composite.xml", catalog.as_bytes())],
    )
    .page(0)
    .unwrap()
}

#[test]
fn nested_composites_compose_child_then_local_then_parent_transforms() {
    let page = composite_page("2 0 0 3 1 2", "0 1 -1 0 7 8");
    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "X\ntail");
    assert_eq!(text.characters()[0].object_id(), Some(33));
    assert_eq!(
        text.characters()[0].rect_mm(),
        Some(Rect {
            x: 3.0,
            y: 82.0,
            width: 8.0,
            height: 12.0
        })
    );
}

#[test]
fn singular_composites_and_text_are_omitted_without_separators() {
    for (outer, inner) in [
        ("1 0 0 0 0 0", "1 0 0 1 0 0"),
        ("1 0 0 1 0 0", "0 0 0 1 0 0"),
    ] {
        assert_eq!(
            composite_page(outer, inner).text().unwrap().as_str(),
            "tail"
        );
    }
}

#[test]
fn effective_template_and_layer_order_precedes_visible_annotation_appearances() {
    let layer = |id: u64, kind, text| {
        format!(
            r#"<ofd:Layer ID="{id}" Type="{kind}">{}</ofd:Layer>"#,
            text_object(id + 100, text)
        )
    };
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Template TemplateID="20"/><ofd:Template TemplateID="30"/><ofd:Content>{}{}{}{}</ofd:Content></ofd:Page>"#,
        layer(1, "Foreground", "front"),
        layer(2, "Body", "body1"),
        layer(3, "Background", "back"),
        layer(4, "Body", "body2")
    );
    let background = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content>{}</ofd:Content></ofd:Page>"#,
        layer(5, "Body", "template back")
    );
    let foreground = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content>{}</ofd:Content></ofd:Page>"#,
        layer(6, "Body", "template front")
    );
    let annotations = r#"<ofd:Annotations xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Page PageID="900">
<ofd:FileLoc>Annots/Page_0/Annotation.xml</ofd:FileLoc>
</ofd:Page></ofd:Annotations>"#;
    let page_annotations = format!(
        r#"<ofd:PageAnnot xmlns:ofd="http://www.ofdspec.org/2016">
<ofd:Annot ID="40" Type="Stamp"><ofd:Appearance Boundary="100 200 30 20"><ofd:PageBlock ID="41">{}</ofd:PageBlock></ofd:Appearance></ofd:Annot>
<ofd:Annot ID="50" Type="Stamp" Visible="false"><ofd:Appearance Boundary="0 0 30 20">{}</ofd:Appearance></ofd:Annot>
<ofd:Annot ID="60" Type="Stamp"><ofd:Appearance Boundary="0 0 30 20">{}</ofd:Appearance></ofd:Annot>
</ofd:PageAnnot>"#,
        text_object(42, "annotation1"),
        text_object(51, "invisible"),
        text_object(61, "annotation2")
    );
    let document = document_with_page(
        &page,
        r#"<ofd:TemplatePage ID="20" BaseLoc="Back.xml"/><ofd:TemplatePage ID="30" BaseLoc="Front.xml" ZOrder="Foreground"/>"#,
        "<ofd:Annotations>Annotations.xml</ofd:Annotations>",
        &[
            ("Doc_0/Back.xml", background.as_bytes()),
            ("Doc_0/Front.xml", foreground.as_bytes()),
            ("Doc_0/Annotations.xml", annotations.as_bytes()),
            (
                "Doc_0/Annots/Page_0/Annotation.xml",
                page_annotations.as_bytes(),
            ),
        ],
    );
    let page = document.page(0).unwrap();
    let text = page.text().unwrap();
    assert_eq!(
        text.as_str(),
        "template back\nback\nbody1\nbody2\nfront\ntemplate front\nannotation1\nannotation2"
    );
    let annotation = text
        .characters()
        .iter()
        .find(|ch| ch.object_id() == Some(42))
        .unwrap();
    assert_eq!(
        annotation.rect_mm(),
        Some(Rect {
            x: 101.0,
            y: 201.0,
            width: 4.0,
            height: 4.0
        })
    );
    assert!(document
        .warnings()
        .iter()
        .all(|warning| warning.code != rofd_core::WarningCode::AnnotationSkipped));
}

#[test]
fn overflowing_coordinates_fail_repeatedly_without_partial_cache_or_poisoning() {
    for attributes in [
        r#"CTM="1e308 0 0 1 0 0""#,
        r#"Boundary="1e308 0 20 8" CTM="1 0 0 1 1e308 0""#,
    ] {
        let boundary = if attributes.starts_with("Boundary") {
            ""
        } else {
            r#"Boundary="0 0 20 8""#
        };
        let page = page_with_text(&format!(
            r#"{}<ofd:TextObject ID="3" {boundary} {attributes} Font="10" Size="4"><ofd:TextCode X="10" Y="5">overflow</ofd:TextCode></ofd:TextObject>"#,
            text_object(2, "valid prefix")
        ));
        for handle in [&page, &page.clone(), &page] {
            assert!(matches!(handle.text(), Err(Error::InvalidValue { .. })));
        }
    }
}

#[test]
fn collapsed_character_geometry_fails_repeatedly_without_cache_publication() {
    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="0 0 20 8" Font="10" Size="1">
<ofd:TextCode X="10000000000000000" Y="10000000000000000" DeltaX="0">X</ofd:TextCode>
</ofd:TextObject>"#,
    );
    let clone = page.clone();

    for handle in [&page, &clone, &page] {
        let error = handle.text().expect_err("collapsed geometry must fail");
        assert!(
            matches!(
                error,
                Error::InvalidValue {
                    field: "semantic text geometry",
                    ..
                }
            ),
            "{error:?}"
        );
    }
}

fn page_with_aggregate_semantic_text(limits: ResourceLimits) -> rofd_core::Page {
    let page_xml = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="1">{}</ofd:Layer></ofd:Content></ofd:Page>"#,
        text_object(2, "A")
    );
    let annotations = r#"<ofd:Annotations xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Page PageID="900"><ofd:FileLoc>Annots/Page_0/Annotation.xml</ofd:FileLoc></ofd:Page></ofd:Annotations>"#;
    let page_annotations = format!(
        r#"<ofd:PageAnnot xmlns:ofd="http://www.ofdspec.org/2016">
<ofd:Annot ID="10" Type="Stamp"><ofd:Appearance Boundary="0 10 20 8">{}</ofd:Appearance></ofd:Annot>
<ofd:Annot ID="20" Type="Stamp"><ofd:Appearance Boundary="0 20 20 8">{}</ofd:Appearance></ofd:Annot>
<ofd:Annot ID="30" Type="Stamp" Visible="false"><ofd:Appearance Boundary="0 30 20 8">{}</ofd:Appearance></ofd:Annot>
</ofd:PageAnnot>"#,
        text_object(11, "B"),
        text_object(21, "C"),
        text_object(31, "D")
    );
    document_with_page_options(
        &page_xml,
        "",
        "<ofd:Annotations>Annotations.xml</ofd:Annotations>",
        &[
            ("Doc_0/Annotations.xml", annotations.as_bytes()),
            (
                "Doc_0/Annots/Page_0/Annotation.xml",
                page_annotations.as_bytes(),
            ),
        ],
        LoadOptions {
            limits,
            ..LoadOptions::default()
        },
    )
    .page(0)
    .unwrap()
}

#[test]
fn semantic_source_scalar_budget_is_aggregate_across_visible_page_content() {
    let exact_limits = ResourceLimits {
        max_text_characters_per_page: 3,
        ..ResourceLimits::default()
    };
    assert_eq!(
        page_with_aggregate_semantic_text(exact_limits)
            .text()
            .unwrap()
            .as_str(),
        "A\nB\nC"
    );

    let exceeded_limits = ResourceLimits {
        max_text_characters_per_page: 2,
        ..ResourceLimits::default()
    };
    let page = page_with_aggregate_semantic_text(exceeded_limits);
    let clone = page.clone();
    for handle in [&page, &clone, &page] {
        assert!(matches!(
            handle.text(),
            Err(Error::LimitExceeded(message))
                if message == "semantic page source character limit exceeded: 3 > 2"
        ));
    }
}

#[test]
fn semantic_metadata_budget_counts_synthesized_separators_at_exact_boundary() {
    let exact_limits = ResourceLimits {
        max_text_characters_per_page: 3,
        max_text_expansion_entries: 5,
        ..ResourceLimits::default()
    };
    assert_eq!(
        page_with_aggregate_semantic_text(exact_limits)
            .text()
            .unwrap()
            .characters()
            .len(),
        5
    );

    let exceeded_limits = ResourceLimits {
        max_text_characters_per_page: 3,
        max_text_expansion_entries: 4,
        ..ResourceLimits::default()
    };
    let page = page_with_aggregate_semantic_text(exceeded_limits);
    let clone = page.clone();
    for handle in [&page, &clone, &page] {
        assert!(matches!(
            handle.text(),
            Err(Error::LimitExceeded(message))
                if message == "semantic text metadata entry limit exceeded: 5 > 4"
        ));
    }
}
