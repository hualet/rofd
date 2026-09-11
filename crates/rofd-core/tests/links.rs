mod support;

use rofd_core::{ActionEvent, ActionKind, Document, LoadOptions, Rect, Strictness, WarningCode};

fn action(region: &str, uri: &str) -> String {
    format!(r#"<Actions><Action Event="CLICK">{region}<URI URI="{uri}"/></Action></Actions>"#)
}

fn document(
    page: &str,
    navigation: &str,
    entries: &[(&str, &[u8])],
    strictness: Strictness,
) -> Document {
    let xml = format!(
        r#"<Document xmlns="http://www.ofdspec.org/2016"><CommonData><PageArea><PhysicalBox>5 7 210 297</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages>{navigation}</Document>"#
    );
    Document::from_bytes(
        support::ofd_with_document_page_and_entries(&xml, page, entries),
        LoadOptions {
            strictness,
            ..LoadOptions::default()
        },
    )
    .unwrap()
}

fn page(body: &str) -> String {
    format!(
        r#"<Page xmlns="http://www.ofdspec.org/2016"><Area><PhysicalBox>5 7 210 297</PhysicalBox></Area>{body}</Page>"#
    )
}

fn path(actions: &str, boundary: &str, ctm: &str) -> String {
    format!(
        r#"<PathObject ID="2" Boundary="{boundary}" CTM="{ctm}" Stroke="false" Fill="true"><AbbreviatedData>M 0 0 L 1 0 L 1 1 C</AbbreviatedData>{actions}</PathObject>"#
    )
}

fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn page_actions_and_object_fallback_keep_order_events_and_parent_boundary_coordinates() {
    let root = r#"<Actions><Action Event="PO"><URI URI="page"/></Action></Actions>"#;
    let object = path(
        &format!(
            r#"{}<Actions><Action Event="CUSTOM"><Sound/></Action></Actions>"#,
            action("", "object")
        ),
        "10 20 20 10",
        "20 0 0 20 0 0",
    );
    let doc = document(
        &page(&format!(
            "{root}<Content><Layer ID=\"1\">{object}</Layer></Content>"
        )),
        "",
        &[],
        Strictness::Strict,
    );
    let page = doc.page(0).unwrap();
    let links = page.links().unwrap();
    assert_eq!(links.len(), 3);
    assert_eq!(links[0].regions, [rectangle(5.0, 7.0, 210.0, 297.0)]);
    assert_eq!(links[0].actions[0].event, ActionEvent::PageOpen);
    assert_eq!(links[1].regions, [rectangle(10.0, 20.0, 20.0, 10.0)]);
    assert_eq!(
        links[2].actions[0].event,
        ActionEvent::Unknown("CUSTOM".into())
    );
    assert!(
        matches!(&links[2].actions[0].kind,ActionKind::Unknown{type_name} if type_name=="Sound")
    );
}

#[test]
fn explicit_multiple_areas_use_object_content_transform_and_remain_separate() {
    let region = r#"<Region><Area Start="0 0"><Line Point1="1 0"/><Line Point1="1 1"/><Line Point1="0 1"/></Area><Area Start="2 3"><Line Point1="4 3"/><Line Point1="4 5"/></Area></Region>"#;
    let object = path(&action(region, "explicit"), "10 20 20 10", "20 0 0 10 0 0");
    let doc = document(
        &page(&format!(
            "<Content><Layer ID=\"1\">{object}</Layer></Content>"
        )),
        "",
        &[],
        Strictness::Strict,
    );
    assert_eq!(
        doc.page(0).unwrap().links().unwrap()[0].regions,
        [
            rectangle(10.0, 20.0, 20.0, 10.0),
            rectangle(50.0, 50.0, 40.0, 20.0)
        ]
    );
}

#[test]
fn invalid_explicit_region_is_lazy_and_never_falls_back_to_owner_or_page() {
    for region in [
        "<Region/>",
        r#"<Region><Area Start="NaN 0"/></Region>"#,
        r#"<Region><Area Start="0 0"><CubicBezier Point1="1 1"/></Area></Region>"#,
        r#"<Region><Area Start="0 0"><Unknown Point1="1 1"/></Area></Region>"#,
    ] {
        let xml = page(&format!(
            "<Content><Layer ID=\"1\">{}</Layer></Content>",
            path(&action(region, "bad"), "10 20 20 10", "1 0 0 1 0 0")
        ));
        let strict = document(&xml, "", &[], Strictness::Strict);
        let loaded = strict.page(0).unwrap();
        assert!(loaded.links().is_err(), "{region}");
        let lenient = document(&xml, "", &[], Strictness::Lenient);
        assert!(lenient.page(0).unwrap().links().unwrap().is_empty());
        assert!(lenient
            .warnings()
            .iter()
            .any(|w| w.code == WarningCode::NavigationInvalid));
    }
}

#[test]
fn bad_outlines_do_not_block_page_links_or_named_bookmark_resolution() {
    let xml = page(
        r#"<Actions><Action Event="CLICK"><Goto><Bookmark Name="target"/></Goto></Action></Actions>"#,
    );
    let nav = r#"<Outlines><OutlineElem/></Outlines><Bookmarks><Bookmark Name="target"><Dest Type="Fit" PageID="42"/></Bookmark></Bookmarks>"#;
    let doc = document(&xml, nav, &[], Strictness::Strict);
    assert!(doc.outline().is_err());
    let page = doc.page(0).unwrap();
    let ActionKind::Goto {
        destination: Some(dest),
        ..
    } = &page.links().unwrap()[0].actions[0].kind
    else {
        panic!()
    };
    assert_eq!(dest.page_index, Some(0));
    assert!(doc.outline().is_err());
}

#[test]
fn page_link_cache_and_warnings_are_shared_across_concurrent_handles() {
    let doc = document(
        &page(r#"<Actions><Action Event="CLICK"><Sound/></Action></Actions>"#),
        "",
        &[],
        Strictness::Strict,
    );
    let page = doc.page(0).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let page = page.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                page.links().unwrap().as_ptr() as usize
            })
        })
        .collect();
    let pointers: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(pointers.iter().all(|p| *p == pointers[0]));
    assert_eq!(
        doc.page(0).unwrap().links().unwrap().as_ptr() as usize,
        pointers[0]
    );
    assert_eq!(doc.warnings().len(), 1);
}

#[test]
fn non_link_annotation_actions_and_appearance_actions_are_exposed_after_page_objects() {
    let annots =
        br#"<Annotations><Page PageID="42"><FileLoc>PageAnnot.xml</FileLoc></Page></Annotations>"#;
    let appearance = format!(
        r#"<PageAnnot><Annot ID="10" Type="Path">{}<Appearance Boundary="30 40 50 60">{}</Appearance></Annot><Annot ID="12" Type="Link" Visible="false">{}<Appearance Boundary="1 1 5 5"/></Annot></PageAnnot>"#,
        action("", "annotation"),
        path(&action("", "appearance"), "2 3 4 5", "9 0 0 9 0 0"),
        action("", "hidden")
    );
    let doc = document(
        &page(&action("", "page")),
        "<Annotations>Annotations.xml</Annotations>",
        &[
            ("Doc_0/Annotations.xml", annots),
            ("Doc_0/PageAnnot.xml", appearance.as_bytes()),
        ],
        Strictness::Strict,
    );
    let page = doc.page(0).unwrap();
    let links = page.links().unwrap();
    assert_eq!(links.len(), 3);
    assert_eq!(links[1].regions, [rectangle(30.0, 40.0, 50.0, 60.0)]);
    assert_eq!(links[2].regions, [rectangle(32.0, 43.0, 4.0, 5.0)]);
}

#[test]
fn foreign_ancestors_and_skipped_wrappers_cannot_smuggle_or_shift_action_bindings() {
    for wrapper in ["foreign-layer", "foreign-group", "skipped-wrapper"] {
        let hidden = path(&action("", "smuggled"), "1 2 3 4", "1 0 0 1 0 0");
        let visible =
            path(&action("", "real"), "50 60 7 8", "1 0 0 1 0 0").replace("ID=\"2\"", "ID=\"99\"");
        let content = match wrapper {
            "foreign-layer" => format!(
                r#"<Content><e:Layer xmlns:e="urn:foreign" ID="1">{hidden}</e:Layer><Layer ID="8">{visible}</Layer></Content>"#
            ),
            "foreign-group" => format!(
                r#"<Content><Layer ID="1"><e:PageBlock xmlns:e="urn:foreign" ID="3">{hidden}</e:PageBlock>{visible}</Layer></Content>"#
            ),
            _ => format!(
                r#"<Content><Layer ID="1"><Unknown><PageBlock ID="3">{hidden}</PageBlock></Unknown>{visible}</Layer></Content>"#
            ),
        };
        let doc = document(&page(&content), "", &[], Strictness::Lenient);
        let loaded = doc.page(0).unwrap();
        let links = loaded.links().unwrap();
        assert_eq!(links.len(), 1, "{wrapper}");
        assert_eq!(links[0].regions, [rectangle(50.0, 60.0, 7.0, 8.0)]);
        assert!(
            matches!(&links[0].actions[0].kind,ActionKind::Uri{uri,..} if uri=="real"),
            "{wrapper}"
        );
    }
}

#[test]
fn bezier_defaults_move_and_close_have_conservative_bounds() {
    let region = r#"<Region><Area Start="0 0"><Move Point1="2 3"/><QuadraticBezier Point1="-4 8" Point2="6 1"/><CubicBezier Point3="10 4"/><Close/><CubicBezier Point1="1 7" Point3="3 4"/></Area></Region>"#;
    let doc = document(
        &page(&action(region, "curves")),
        "",
        &[],
        Strictness::Strict,
    );
    assert_eq!(
        doc.page(0).unwrap().links().unwrap()[0].regions,
        [rectangle(-4.0, 0.0, 14.0, 8.0)]
    );
}

#[test]
fn arc_radii_normalization_correction_and_standard_degenerate_arrays() {
    for (radii, expected) in [
        ("1", rectangle(0.0, -5.0, 10.0, 10.0)),
        ("1 2 999", rectangle(0.0, -10.0, 10.0, 20.0)),
        ("-1 -2", rectangle(0.0, -10.0, 10.0, 20.0)),
        ("", rectangle(0.0, 0.0, 10.0, 4.0)),
        ("0 5", rectangle(0.0, 0.0, 10.0, 4.0)),
    ] {
        let region = format!(
            r#"<Region><Area Start="0 0"><Arc EllipseSize="{radii}" RotationAngle="360" LargeArc="0" SweepDirection="true" EndPoint="10 0"/><Line Point1="10 4"/><Close/></Area></Region>"#
        );
        let doc = document(&page(&action(&region, "arc")), "", &[], Strictness::Strict);
        assert_eq!(
            doc.page(0).unwrap().links().unwrap()[0].regions,
            [expected],
            "{radii}"
        );
    }
}

#[test]
fn arc_endpoint_equal_to_current_is_invalid_even_with_zero_radii() {
    for radii in ["0", "1 2"] {
        let region = format!(
            r#"<Region><Area Start="0 0"><Move Point1="2 3"/><Line Point1="4 5"/><Close/><Arc EllipseSize="{radii}" RotationAngle="0" LargeArc="false" SweepDirection="true" EndPoint="2 3"/></Area></Region>"#
        );
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let doc = document(&page(&action(&region, "invalid")), "", &[], strictness);
            let page = doc.page(0).unwrap();
            if strictness == Strictness::Strict {
                assert!(page.links().is_err());
            } else {
                assert!(page.links().unwrap().is_empty());
            }
        }
    }
}

#[test]
fn page_and_template_root_actions_precede_their_effective_content_including_empty_templates() {
    let xml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 100 100</PhysicalBox></PageArea><TemplatePage ID="100" BaseLoc="BG.xml"/><TemplatePage ID="101" BaseLoc="Empty.xml"/><TemplatePage ID="102" BaseLoc="FG.xml" ZOrder="Foreground"/></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    let background = page(&format!(
        "{}<Content><Layer ID=\"10\">{}</Layer></Content>",
        action("", "bg-root"),
        path(&action("", "bg-object"), "1 2 3 4", "1 0 0 1 0 0")
    ));
    let empty = page(&action("", "empty-root"));
    let foreground = page(&format!(
        "{}<Content><Layer ID=\"20\">{}</Layer></Content>",
        action("", "fg-root"),
        path(&action("", "fg-object"), "1 2 3 4", "1 0 0 1 0 0")
    ));
    let page = page(&format!(
        r#"{}<Template TemplateID="100"/><Template TemplateID="101"/><Template TemplateID="102"/><Content><Layer ID="1">{}</Layer></Content>"#,
        action("", "page-root"),
        path(&action("", "page-object"), "1 2 3 4", "1 0 0 1 0 0")
    ));
    let doc = Document::from_bytes(
        support::ofd_with_document_page_and_entries(
            xml,
            &page,
            &[
                ("Doc_0/BG.xml", background.as_bytes()),
                ("Doc_0/Empty.xml", empty.as_bytes()),
                ("Doc_0/FG.xml", foreground.as_bytes()),
            ],
        ),
        LoadOptions::default(),
    )
    .unwrap();
    let page = doc.page(0).unwrap();
    let uris: Vec<_> = page
        .links()
        .unwrap()
        .iter()
        .map(|link| match &link.actions[0].kind {
            ActionKind::Uri { uri, .. } => uri.as_str(),
            _ => panic!(),
        })
        .collect();
    assert_eq!(
        uris,
        [
            "page-root",
            "bg-root",
            "bg-object",
            "empty-root",
            "page-object",
            "fg-root",
            "fg-object"
        ]
    );
}

#[test]
fn nested_composite_resource_actions_reuse_the_existing_parent_transform_chain() {
    let xml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 100 100</PhysicalBox></PageArea><DocumentRes>Res.xml</DocumentRes></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    let region = r#"<Region><Area Start="0 0"><Line Point1="1 0"/><Line Point1="1 1"/><Close/></Area></Region>"#;
    let inner = path(
        &format!(
            "{}{}",
            action("", "inner-fallback"),
            action(region, "inner-region")
        ),
        "1 2 4 5",
        "2 0 0 2 0 0",
    );
    let resource = format!(
        r#"<Res><CompositeGraphicUnits><CompositeGraphicUnit ID="10" Width="10" Height="10"><Content>{}<CompositeObject ID="3" ResourceID="20" Boundary="4 5 6 7" CTM="3 0 0 4 0 0">{}</CompositeObject></Content></CompositeGraphicUnit><CompositeGraphicUnit ID="20" Width="10" Height="10"><Content>{inner}</Content></CompositeGraphicUnit></CompositeGraphicUnits></Res>"#,
        action(region, "resource-root"),
        action("", "nested")
    );
    let page = page(&format!(
        r#"<Content><Layer ID="1"><CompositeObject ID="1" ResourceID="10" Boundary="10 20 30 40" CTM="2 0 0 3 0 0">{}</CompositeObject></Layer></Content>"#,
        action("", "outer")
    ));
    let doc = Document::from_bytes(
        support::ofd_with_document_page_and_entries(
            xml,
            &page,
            &[("Doc_0/Res.xml", resource.as_bytes())],
        ),
        LoadOptions::default(),
    )
    .unwrap();
    let page = doc.page(0).unwrap();
    let links = page.links().unwrap();
    assert_eq!(links.len(), 5);
    assert_eq!(links[0].regions, [rectangle(10.0, 20.0, 30.0, 40.0)]);
    assert_eq!(links[1].regions, [rectangle(10.0, 20.0, 2.0, 3.0)]);
    assert_eq!(links[2].regions, [rectangle(18.0, 35.0, 12.0, 21.0)]);
    assert_eq!(links[3].regions, [rectangle(24.0, 59.0, 24.0, 60.0)]);
    assert_eq!(links[4].regions, [rectangle(24.0, 59.0, 12.0, 24.0)]);
}

#[test]
fn image_boundary_fallback_is_not_scaled_twice_and_does_not_decode_image_bytes() {
    let xml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 100 100</PhysicalBox></PageArea><DocumentRes>Res.xml</DocumentRes></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    let resource=br#"<Res><MultiMedias><MultiMedia ID="10" Type="Image" Format="PNG"><MediaFile>deliberately-missing.png</MediaFile></MultiMedia></MultiMedias></Res>"#;
    let page = page(&format!(
        r#"<Content><Layer ID="1"><ImageObject ID="2" Boundary="10 20 20 30" CTM="20 0 0 30 0 0" ResourceID="10">{}</ImageObject></Layer></Content>"#,
        action("", "image")
    ));
    let doc = Document::from_bytes(
        support::ofd_with_document_page_and_entries(xml, &page, &[("Doc_0/Res.xml", resource)]),
        LoadOptions::default(),
    )
    .unwrap();
    assert_eq!(
        doc.page(0).unwrap().links().unwrap()[0].regions,
        [rectangle(10.0, 20.0, 20.0, 30.0)]
    );
}

#[test]
fn bookmark_warning_commit_is_order_independent_and_failed_links_publish_nothing() {
    let nav =
        r#"<Bookmarks><Bookmark Name="b"><Dest Type="CUSTOM" PageID="42"/></Bookmark></Bookmarks>"#;
    let valid =
        r#"<Actions><Action Event="CLICK"><Goto><Bookmark Name="b"/></Goto></Action></Actions>"#;
    let invalid = action("<Region/>", "bad");
    let doc = document(
        &page(&format!("{valid}{invalid}")),
        nav,
        &[],
        Strictness::Strict,
    );
    let loaded = doc.page(0).unwrap();
    for _ in 0..3 {
        assert!(loaded.links().is_err());
        assert!(doc.warnings().is_empty());
    }
    assert!(doc.outline().unwrap().is_empty());
    assert_eq!(doc.warnings().len(), 1);
    for outline_first in [false, true] {
        let doc = document(&page(valid), nav, &[], Strictness::Strict);
        if outline_first {
            doc.outline().unwrap();
        }
        doc.page(0).unwrap().links().unwrap();
        doc.outline().unwrap();
        assert_eq!(doc.warnings().len(), 1);
    }
    let doc = document(
        &page(&action("", "no-bookmark")),
        "<Bookmarks><Bookmark/></Bookmarks>",
        &[],
        Strictness::Strict,
    );
    assert!(doc.page(0).unwrap().links().is_ok());
    assert!(doc.outline().is_err());
}

#[test]
fn page_link_node_and_command_budgets_are_cumulative_and_fail_without_publication() {
    let region = r#"<Region><Area Start="0 0"><Line Point1="1 0"/><Line Point1="1 1"/><Close/></Area></Region>"#;
    let xml = page(&format!(
        "{}{}",
        action(region, "first"),
        action(region, "second")
    ));
    let docxml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    for (commands, nodes, ok) in [(6, 16, true), (5, 16, false), (6, 15, false)] {
        let mut options = LoadOptions::default();
        options.limits.max_path_commands = commands;
        options.limits.max_page_objects = nodes;
        let doc = Document::from_bytes(
            support::ofd_with_document_page_and_entries(docxml, &xml, &[]),
            options,
        )
        .unwrap();
        let loaded = doc.page(0).unwrap();
        for _ in 0..2 {
            let result = loaded.links();
            if ok {
                assert_eq!(result.unwrap().len(), 2);
            } else {
                assert!(matches!(result, Err(rofd_core::Error::LimitExceeded(_))));
                assert!(doc.warnings().is_empty());
            }
        }
    }
}

#[test]
fn all_real_compatibility_pages_can_query_links_without_changing_page_loading() {
    for name in ["z.ofd", "发票示例.ofd", "testImageNotFound.ofd"] {
        let doc = Document::open(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/ofdrw-compat/fixtures/converter")
                .join(name),
            LoadOptions::default(),
        )
        .unwrap();
        for index in 0..doc.page_count() {
            let page = doc.page(index).unwrap();
            page.links().unwrap();
        }
    }
}

#[test]
fn missing_owner_boundary_uses_physical_page_instead_of_a_synthetic_empty_box() {
    let object = path(&action("", "unbounded"), "1 2 3 4", "1 0 0 1 0 0")
        .replace(" Boundary=\"1 2 3 4\"", "");
    let annots=br#"<Annotations><Page PageID="42"><Annot ID="10" Type="Link"><Actions><Action Event="CLICK"><URI URI="no-appearance"/></Action></Actions></Annot></Page></Annotations>"#;
    let doc = document(
        &page(&format!(
            r#"<Content><Layer ID="1">{object}</Layer></Content>"#
        )),
        "<Annotations>Annotations.xml</Annotations>",
        &[("Doc_0/Annotations.xml", annots)],
        Strictness::Lenient,
    );
    let loaded = doc.page(0).unwrap();
    assert_eq!(loaded.links().unwrap().len(), 2);
    assert!(loaded
        .links()
        .unwrap()
        .iter()
        .all(|link| link.regions == [rectangle(5.0, 7.0, 210.0, 297.0)]));
}

#[test]
fn lenient_transform_overflow_skips_only_the_affected_explicit_region_or_subtree() {
    let xml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 100 100</PhysicalBox></PageArea><DocumentRes>Res.xml</DocumentRes></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    let region =
        r#"<Region><Area Start="0 0"><Line Point1="1 0"/><Line Point1="1 1"/></Area></Region>"#;
    let leaf = path(&action(region, "bad"), "0 0 1 1", "1e308 0 0 1 0 0");
    let nested =
        r#"<CompositeObject ID="3" Boundary="0 0 1 1" CTM="1e308 0 0 1 0 0" ResourceID="20"/>"#;
    for content in [leaf, nested.into()] {
        let resource = format!(
            r#"<Res><CompositeGraphicUnits><CompositeGraphicUnit ID="10" Width="1" Height="1"><Content>{content}</Content></CompositeGraphicUnit><CompositeGraphicUnit ID="20" Width="1" Height="1"><Content>{}</Content></CompositeGraphicUnit></CompositeGraphicUnits></Res>"#,
            path(&action("", "also-bad"), "0 0 1 1", "1 0 0 1 0 0")
        );
        let page = page(&format!(
            r#"<Content><Layer ID="1"><CompositeObject ID="4" Boundary="0 0 1 1" CTM="1e308 0 0 1 0 0" ResourceID="10"/>{}</Layer></Content>"#,
            path(&action("", "good"), "2 3 4 5", "1 0 0 1 0 0").replace("ID=\"2\"", "ID=\"99\"")
        ));
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let doc = Document::from_bytes(
                support::ofd_with_document_page_and_entries(
                    xml,
                    &page,
                    &[("Doc_0/Res.xml", resource.as_bytes())],
                ),
                LoadOptions {
                    strictness,
                    ..LoadOptions::default()
                },
            )
            .unwrap();
            let loaded = doc.page(0).unwrap();
            if strictness == Strictness::Strict {
                assert!(loaded.links().is_err());
            } else {
                let links = loaded.links().unwrap();
                assert_eq!(links.len(), 1);
                assert!(matches!(&links[0].actions[0].kind,ActionKind::Uri{uri,..} if uri=="good"));
                assert!(doc
                    .warnings()
                    .iter()
                    .any(|w| w.code == WarningCode::NavigationInvalid));
            }
        }
    }
}

#[test]
fn empty_link_queries_do_not_charge_internal_owner_bookkeeping_as_extra_page_objects() {
    let docxml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    for (body, limit) in [
        (String::new(), 0),
        (
            format!(
                r#"<Content><Layer ID="1">{}</Layer></Content>"#,
                path("", "0 0 1 1", "1 0 0 1 0 0")
            ),
            2,
        ),
    ] {
        let mut options = LoadOptions::default();
        options.limits.max_page_objects = limit;
        let doc = Document::from_bytes(
            support::ofd_with_document_page_and_entries(docxml, &page(&body), &[]),
            options,
        )
        .unwrap();
        assert!(doc.page(0).unwrap().links().unwrap().is_empty());
    }
}

#[test]
fn repeated_template_actions_share_capture_but_consume_effective_query_budgets() {
    let docxml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea><TemplatePage ID="10" BaseLoc="Template.xml"/></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    let region = r#"<Region><Area Start="0 0"><Line Point1="1 0"/><Line Point1="1 1"/><Close/></Area></Region>"#;
    let repeated = page(&r#"<Template TemplateID="10"/>"#.repeat(8));
    for (regions, uri, entries, commands, bytes, succeeds) in [
        ("", "ok".to_owned(), 24, 24, 4096, true),
        ("", "ok".to_owned(), 23, 24, 4096, false),
        (region, "ok".to_owned(), 64, 24, 4096, true),
        (region, "ok".to_owned(), 64, 23, 4096, false),
        ("", "x".repeat(1024), 24, 24, 4096, false),
    ] {
        let template = page(&action(regions, &uri));
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut options = LoadOptions {
                strictness,
                ..LoadOptions::default()
            };
            options.limits.max_page_objects = entries;
            options.limits.max_path_commands = commands;
            options.limits.max_entry_size = bytes;
            let doc = Document::from_bytes(
                support::ofd_with_document_page_and_entries(
                    docxml,
                    &repeated,
                    &[("Doc_0/Template.xml", template.as_bytes())],
                ),
                options,
            )
            .unwrap();
            let loaded = doc.page(0).unwrap();
            for _ in 0..2 {
                if succeeds {
                    assert_eq!(loaded.links().unwrap().len(), 8);
                } else {
                    assert!(matches!(
                        loaded.links(),
                        Err(rofd_core::Error::LimitExceeded(_))
                    ));
                }
                assert!(doc.warnings().is_empty());
            }
        }
    }
}

#[test]
fn unused_resource_owners_do_not_spend_the_effective_page_link_budget() {
    let docxml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea><DocumentRes>Res.xml</DocumentRes></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages></Document>"#;
    let mut units = String::new();
    for id in 10..40 {
        units.push_str(&format!(r#"<CompositeGraphicUnit ID="{id}" Width="1" Height="1"><Content/></CompositeGraphicUnit>"#));
    }
    let resource = format!("<Res><CompositeGraphicUnits>{units}</CompositeGraphicUnits></Res>");
    let xml = page(
        r#"<Content><Layer ID="1"><CompositeObject ID="2" ResourceID="39" Boundary="0 0 1 1"/></Layer></Content>"#,
    );
    let mut options = LoadOptions::default();
    options.limits.max_page_objects = 2;
    let doc = Document::from_bytes(
        support::ofd_with_document_page_and_entries(
            docxml,
            &xml,
            &[("Doc_0/Res.xml", resource.as_bytes())],
        ),
        options,
    )
    .unwrap();
    assert!(doc.page(0).unwrap().links().unwrap().is_empty());
}

#[test]
fn annotation_resource_limits_remain_fatal_for_links_in_both_cache_orders() {
    let annotations =
        br#"<Annotations><Page PageID="42"><FileLoc>PageAnnot.xml</FileLoc></Page></Annotations>"#;
    let appearance = format!(
        r#"<PageAnnot><Annot ID="10" Type="Link"><Appearance Boundary="0 0 10 10">{}</Appearance></Annot></PageAnnot>"#,
        path(&action("", "limited"), "0 0 1 1", "1 0 0 1 0 0")
    );
    let docxml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages><Annotations>Annotations.xml</Annotations></Document>"#;
    let deep_entry = format!(
        "<Annotations>{}{}</Annotations>",
        "<Extension>".repeat(8),
        "</Extension>".repeat(8)
    );
    let deep_file = format!(
        "<PageAnnot>{}{}</PageAnnot>",
        "<Extension>".repeat(8),
        "</Extension>".repeat(8)
    );
    for (annotations, appearance, command_limit, depth_limit) in [
        (annotations.as_slice(), appearance.as_bytes(), 1, 256),
        (deep_entry.as_bytes(), appearance.as_bytes(), 250_000, 6),
        (annotations.as_slice(), deep_file.as_bytes(), 250_000, 6),
    ] {
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            for public_first in [false, true] {
                let mut options = LoadOptions {
                    strictness,
                    ..LoadOptions::default()
                };
                options.limits.max_path_commands = command_limit;
                options.limits.max_xml_depth = depth_limit;
                let doc = Document::from_bytes(
                    support::ofd_with_document_page_and_entries(
                        docxml,
                        &page(""),
                        &[
                            ("Doc_0/Annotations.xml", annotations),
                            ("Doc_0/PageAnnot.xml", appearance),
                        ],
                    ),
                    options,
                )
                .unwrap();
                let loaded = doc.page(0).unwrap();
                if public_first {
                    assert!(doc.page_annotations().unwrap().is_empty());
                }
                for _ in 0..3 {
                    assert!(matches!(
                        loaded.links(),
                        Err(rofd_core::Error::LimitExceeded(_))
                    ));
                    assert!(doc.page_annotations().unwrap().is_empty());
                    assert_eq!(doc.warnings().len(), 1);
                    assert_eq!(doc.warnings()[0].code, WarningCode::AnnotationSkipped);
                }
            }
        }
    }
}

#[test]
fn historical_owner_ancestry_is_warned_even_when_action_elements_are_unqualified() {
    for xml in [
        format!(
            r#"<old:Page xmlns:old="http://www.ofdspec.org"><Area><PhysicalBox>0 0 10 10</PhysicalBox></Area>{}</old:Page>"#,
            action("", "page")
        ),
        format!(
            r#"<Page><Area><PhysicalBox>0 0 10 10</PhysicalBox></Area><Content><old:Layer xmlns:old="http://www.ofdspec.org" ID="1">{}</old:Layer></Content></Page>"#,
            action("", "layer")
        ),
    ] {
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let doc = document(&xml, "", &[], strictness);
            let loaded = doc.page(0).unwrap();
            for _ in 0..3 {
                assert_eq!(loaded.links().unwrap().len(), 1);
            }
            assert_eq!(doc.warnings().len(), 1);
            assert_eq!(doc.warnings()[0].code, WarningCode::NavigationCompatibility);
        }
    }
}
