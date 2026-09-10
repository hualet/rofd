mod support;

use rofd_core::{
    ActionEvent, ActionKind, DestinationMode, Document, LoadOptions, Strictness, WarningCode,
};

fn bytes(navigation: &str) -> Vec<u8> {
    support::ofd_with_entries(
        &format!(
            r#"<Document xmlns="http://www.ofdspec.org/2016" xmlns:x="urn:extension">
      <CommonData><PageArea><PhysicalBox>0 0 210 297</PhysicalBox></PageArea></CommonData>
      <Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/><Page ID="900" BaseLoc="Pages/Page_0/Content.xml"/><Page ID="7" BaseLoc="Pages/Page_0/Content.xml"/></Pages>
      {navigation}</Document>"#
        ),
        &[],
    )
}

fn open(navigation: &str, strictness: Strictness) -> Document {
    Document::from_bytes(
        bytes(navigation),
        LoadOptions {
            strictness,
            ..LoadOptions::default()
        },
    )
    .unwrap()
}

fn outline_action(body: &str) -> String {
    format!(
        r#"<Outlines><OutlineElem Title="Target"><Actions><Action Event="CLICK">{body}</Action></Actions></OutlineElem></Outlines>"#
    )
}

#[test]
fn empty_optional_outline_and_real_preorder_ignore_advisory_count() {
    assert!(open("", Strictness::Strict).outline().unwrap().is_empty());
    let doc = open(
        r#"<Outlines><OutlineElem Title="章节" Count="999999999999999999999" Expanded="false"><OutlineElem Title="A"><OutlineElem Title="B"/></OutlineElem><OutlineElem Title="C"/></OutlineElem><OutlineElem Title="" Count="0"/></Outlines>"#,
        Strictness::Strict,
    );
    let nodes = doc.outline().unwrap();
    assert_eq!(
        nodes.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(),
        ["章节", "A", "B", "C", ""]
    );
    assert_eq!(
        nodes
            .iter()
            .map(|n| (n.parent, n.first_child, n.next_sibling))
            .collect::<Vec<_>>(),
        [
            (None, Some(1), Some(4)),
            (Some(0), Some(2), Some(3)),
            (Some(1), None, None),
            (Some(0), None, None),
            (None, None, None)
        ]
    );
    assert!(!nodes[0].expanded);
    assert!(nodes[1..]
        .iter()
        .all(|n| n.expanded && n.actions.is_empty()));
}

#[test]
fn destination_modes_preserve_optional_decimal_coordinates_and_raw_zoom() {
    let modes = [
        ("XYZ", DestinationMode::Xyz),
        ("Fit", DestinationMode::Fit),
        ("FitH", DestinationMode::FitH),
        ("FitV", DestinationMode::FitV),
        ("FitR", DestinationMode::FitR),
    ];
    for (name, mode) in modes {
        let doc = open(
            &outline_action(&format!(
                r#"<Goto><Dest Type="{name}" PageID="900" Left="1.25" Top="-2.5" Right="8.75" Bottom="9" Zoom="0"/></Goto>"#
            )),
            Strictness::Strict,
        );
        let ActionKind::Goto {
            destination: Some(dest),
            bookmark: None,
        } = &doc.outline().unwrap()[0].actions[0].kind
        else {
            panic!("expected destination")
        };
        assert_eq!((dest.page_id, dest.page_index), (900, Some(1)));
        assert_eq!(dest.mode, mode);
        assert_eq!(
            (dest.left, dest.top, dest.right, dest.bottom, dest.zoom),
            (Some(1.25), Some(-2.5), Some(8.75), Some(9.0), Some(0.0))
        );
    }
    let doc = open(
        &outline_action(r#"<Goto><Dest Type="XYZ" PageID="7"/></Goto>"#),
        Strictness::Strict,
    );
    let ActionKind::Goto {
        destination: Some(dest),
        ..
    } = &doc.outline().unwrap()[0].actions[0].kind
    else {
        panic!()
    };
    assert_eq!(dest.page_index, Some(2));
    assert_eq!(
        (dest.left, dest.top, dest.right, dest.bottom, dest.zoom),
        (None, None, None, None, None)
    );
}

#[test]
fn ordered_actions_keep_inert_uris_attachments_unknown_events_and_types() {
    let doc = open(
        r#"<Outlines><OutlineElem Title="Actions"><Actions>
      <Action Event="DO"><URI URI="relative/path" Base="https://example.test/"/></Action>
      <Action Event="PO"><URI URI="file:///etc/passwd"/></Action>
      <Action Event="CLICK"><GotoA AttachID="attachment-string-id"/></Action>
      <Action Event="CLICK"><GotoA AttachID="other" NewWindow="false"/></Action>
      <Action Event="CUSTOM"><Sound/></Action><Action Event="CLICK"><x:Goto><Dest Type="Fit" PageID="42"/></x:Goto></Action>
    </Actions></OutlineElem></Outlines>"#,
        Strictness::Strict,
    );
    let actions = &doc.outline().unwrap()[0].actions;
    assert_eq!(actions.len(), 6);
    assert_eq!(actions[0].event, ActionEvent::DocumentOpen);
    assert_eq!(actions[1].event, ActionEvent::PageOpen);
    assert_eq!(actions[2].event, ActionEvent::Click);
    assert_eq!(actions[4].event, ActionEvent::Unknown("CUSTOM".into()));
    assert_eq!(
        actions[0].kind,
        ActionKind::Uri {
            uri: "relative/path".into(),
            base: Some("https://example.test/".into())
        }
    );
    assert_eq!(
        actions[1].kind,
        ActionKind::Uri {
            uri: "file:///etc/passwd".into(),
            base: None
        }
    );
    assert_eq!(
        actions[2].kind,
        ActionKind::Attachment {
            attachment_id: "attachment-string-id".into(),
            new_window: true
        }
    );
    assert_eq!(
        actions[3].kind,
        ActionKind::Attachment {
            attachment_id: "other".into(),
            new_window: false
        }
    );
    assert_eq!(
        actions[4].kind,
        ActionKind::Unknown {
            type_name: "Sound".into()
        }
    );
    assert_eq!(
        actions[5].kind,
        ActionKind::Unknown {
            type_name: "{urn:extension}Goto".into()
        }
    );
    assert_eq!(
        doc.warnings()
            .iter()
            .filter(|w| w.code == WarningCode::NavigationUnsupported)
            .count(),
        3
    );
}

#[test]
fn forward_named_bookmark_resolves_noncontiguous_page_id() {
    let nav = format!(
        r#"{}<Bookmarks><Bookmark Name="later"><Dest Type="FitH" PageID="7" Top="12.25"/></Bookmark></Bookmarks>"#,
        outline_action(r#"<Goto><Bookmark Name="later"/></Goto>"#)
    );
    let doc = open(&nav, Strictness::Strict);
    let ActionKind::Goto {
        destination: Some(dest),
        bookmark: Some(name),
    } = &doc.outline().unwrap()[0].actions[0].kind
    else {
        panic!()
    };
    assert_eq!(name, "later");
    assert_eq!(
        (dest.page_id, dest.page_index, dest.top),
        (7, Some(2), Some(12.25))
    );
}

#[test]
fn malformed_navigation_is_lazy_and_lenient_destinations_never_become_page_zero() {
    for body in [
        r#"<Goto><Dest Type="XYZ" PageID="42" Left="NaN"/></Goto>"#,
        r#"<Goto><Dest Type="XYZ" PageID="42" Zoom="inf"/></Goto>"#,
        r#"<Goto><Dest Type="Fit"/></Goto>"#,
        r#"<Goto><Bookmark Name="missing"/></Goto>"#,
    ] {
        let strict = open(&outline_action(body), Strictness::Strict);
        strict.page(0).unwrap();
        assert!(strict.outline().is_err(), "{body}");
        let doc = open(&outline_action(body), Strictness::Lenient);
        assert!(doc.warnings().is_empty());
        assert!(matches!(
            doc.outline().unwrap()[0].actions[0].kind,
            ActionKind::Goto {
                destination: None,
                ..
            }
        ));
        assert!(doc
            .warnings()
            .iter()
            .any(|w| w.code == WarningCode::NavigationInvalid));
    }
    let nav = outline_action(r#"<Goto><Dest Type="Fit" PageID="999"/></Goto>"#);
    assert!(open(&nav, Strictness::Strict).outline().is_err());
    let doc = open(&nav, Strictness::Lenient);
    let ActionKind::Goto {
        destination: Some(dest),
        ..
    } = &doc.outline().unwrap()[0].actions[0].kind
    else {
        panic!()
    };
    assert_eq!((dest.page_id, dest.page_index), (999, None));
}

#[test]
fn duplicate_bookmark_names_are_ambiguous_not_first_wins() {
    let nav = format!(
        r#"{}<Bookmarks><Bookmark Name="same"><Dest Type="Fit" PageID="42"/></Bookmark><Bookmark Name="same"><Dest Type="Fit" PageID="7"/></Bookmark></Bookmarks>"#,
        outline_action(r#"<Goto><Bookmark Name="same"/></Goto>"#)
    );
    assert!(open(&nav, Strictness::Strict).outline().is_err());
    let doc = open(&nav, Strictness::Lenient);
    assert!(
        matches!(&doc.outline().unwrap()[0].actions[0].kind,ActionKind::Goto { destination:None,bookmark:Some(name) } if name == "same")
    );
}

#[test]
fn namespace_and_direct_child_boundaries_prevent_extension_tag_confusion() {
    let doc = open(
        r#"<x:Outlines><OutlineElem Title="foreign root"/></x:Outlines><Outlines><x:OutlineElem Title="foreign node"/><x:Wrapper><OutlineElem Title="nested extension"/></x:Wrapper><OutlineElem Title="real"><x:Actions><Action Event="CLICK"><URI URI="ignored"/></Action></x:Actions><Actions><Action Event="CLICK"><x:URI URI="not-native"/></Action></Actions></OutlineElem></Outlines>"#,
        Strictness::Strict,
    );
    let nodes = doc.outline().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].title, "real");
    assert_eq!(nodes[0].actions.len(), 1);
    assert_eq!(
        nodes[0].actions[0].kind,
        ActionKind::Unknown {
            type_name: "{urn:extension}URI".into()
        }
    );
    let xml = r#"<Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages><Outlines><OutlineElem Title="plain"/></Outlines></Document>"#;
    let plain =
        Document::from_bytes(support::ofd_with_entries(xml, &[]), LoadOptions::default()).unwrap();
    assert_eq!(plain.outline().unwrap()[0].title, "plain");
}

#[test]
fn numeric_child_compatibility_is_explicit_and_attributes_have_priority() {
    let doc = open(
        &outline_action(
            r#"<Goto><Dest Type="XYZ" PageID="900" Left="1.25"><PageID>42</PageID><Left>NaN</Left><Top>2.75</Top><Zoom>0</Zoom></Dest></Goto>"#,
        ),
        Strictness::Strict,
    );
    let ActionKind::Goto {
        destination: Some(dest),
        ..
    } = &doc.outline().unwrap()[0].actions[0].kind
    else {
        panic!()
    };
    assert_eq!(
        (dest.page_id, dest.left, dest.top, dest.zoom),
        (900, Some(1.25), Some(2.75), Some(0.0))
    );
    assert!(doc
        .warnings()
        .iter()
        .any(|w| w.code == WarningCode::NavigationCompatibility));
    let fallback = open(
        &outline_action(r#"<Goto><Dest Type="Fit"><PageID>7</PageID></Dest></Goto>"#),
        Strictness::Strict,
    );
    let ActionKind::Goto {
        destination: Some(dest),
        ..
    } = &fallback.outline().unwrap()[0].actions[0].kind
    else {
        panic!()
    };
    assert_eq!(dest.page_index, Some(2));
}

#[test]
fn navigation_limits_are_lazy_and_ignore_unrelated_document_elements() {
    let nav = r#"<Outlines><OutlineElem Title="root" Count="999999999"/></Outlines>"#;
    for (limit, ok) in [(1, false), (2, true)] {
        let mut options = LoadOptions::default();
        options.limits.max_page_objects = limit;
        let doc = Document::from_bytes(bytes(nav), options).unwrap();
        assert_eq!(doc.outline().is_ok(), ok);
    }
    let mut options = LoadOptions::default();
    options.limits.max_page_block_depth = 2;
    let doc=Document::from_bytes(bytes(r#"<Outlines><OutlineElem Title="a"><OutlineElem Title="b"><OutlineElem Title="c"/></OutlineElem></OutlineElem></Outlines>"#),options).unwrap();
    assert!(matches!(
        doc.outline(),
        Err(rofd_core::Error::LimitExceeded(_))
    ));
    let mut options = LoadOptions::default();
    options.limits.max_page_objects = 6;
    assert!(Document::from_bytes(
        bytes(&outline_action(
            r#"<Goto><Dest Type="Fit" PageID="42"/></Goto>"#
        )),
        options.clone()
    )
    .unwrap()
    .outline()
    .is_ok());
    let doc=Document::from_bytes(bytes(&format!(r#"{}<Bookmarks><Bookmark Name="x"><Dest Type="Fit" PageID="42"/></Bookmark></Bookmarks>"#,outline_action(r#"<Goto><Dest Type="Fit" PageID="42"/></Goto>"#))),options).unwrap();
    assert!(matches!(
        doc.outline(),
        Err(rofd_core::Error::LimitExceeded(_))
    ));
}

#[test]
fn cloned_concurrent_queries_share_cache_and_commit_warnings_once() {
    let doc = open(&outline_action("<Sound/>"), Strictness::Strict);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let doc = doc.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                doc.outline().unwrap().as_ptr() as usize
            })
        })
        .collect();
    let pointers: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(pointers.iter().all(|p| *p == pointers[0]));
    assert_eq!(doc.outline().unwrap().as_ptr() as usize, pointers[0]);
    assert_eq!(doc.warnings().len(), 1);
    assert_eq!(doc.warnings()[0].code, WarningCode::NavigationUnsupported);
}

#[test]
fn malformed_structures_fail_lazily_and_lenient_parsing_keeps_later_entries() {
    for nav in [
        r#"<Outlines><OutlineElem/></Outlines>"#.to_owned(),
        r#"<Outlines><OutlineElem Title="x" Expanded="perhaps"/></Outlines>"#.to_owned(),
        r#"<Outlines><OutlineElem Title="x"><Actions><Action><URI URI="x"/></Action></Actions></OutlineElem></Outlines>"#.to_owned(),
        outline_action("<URI/>"),
        outline_action("<GotoA/>"),
        outline_action(r#"<GotoA AttachID="id" NewWindow="perhaps"/>"#),
        outline_action("<Goto/>"),
        outline_action(r#"<Goto><Dest PageID="42"/></Goto>"#),
        outline_action(r#"<Goto><Dest Type="Fit"><PageID>42</PageID><PageID>7</PageID></Dest></Goto>"#),
        outline_action(r#"<Goto><Dest Type="Fit"><PageID><x:Value>42</x:Value></PageID></Dest></Goto>"#),
        outline_action(r#"<Goto><Dest Type="Fit" PageID="42"/><Bookmark Name="x"/></Goto>"#),
        outline_action(r#"<URI URI="x"/><URI URI="y"/>"#),
        r#"<Bookmarks><Bookmark><Dest Type="Fit" PageID="42"/></Bookmark></Bookmarks>"#.to_owned(),
    ] {
        let doc = open(&nav, Strictness::Strict);
        assert!(doc.outline().is_err(), "{nav}");
        assert!(doc.warnings().is_empty(), "failed strict query must not commit warnings");
        doc.page(0).unwrap();
        let doc = open(&nav, Strictness::Lenient);
        assert!(doc.outline().is_ok(), "{nav}");
        assert!(doc.warnings().iter().any(|w| w.code == WarningCode::NavigationInvalid), "{nav}");
    }
    let doc = open(
        r#"<Outlines><OutlineElem><Actions><Action Event="CLICK"><Goto><Dest Type="XYZ" PageID="42" Left="bad"/></Goto></Action><Action Event="CLICK"><URI URI="still-available"/></Action></Actions><OutlineElem Title="child"/></OutlineElem><OutlineElem Title="sibling"/></Outlines>"#,
        Strictness::Lenient,
    );
    let nodes = doc.outline().unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].title, "");
    assert_eq!(
        (nodes[0].first_child, nodes[0].next_sibling),
        (Some(1), Some(2))
    );
    assert!(matches!(
        nodes[0].actions[0].kind,
        ActionKind::Goto {
            destination: None,
            ..
        }
    ));
    assert!(
        matches!(&nodes[0].actions[1].kind,ActionKind::Uri{uri,..} if uri == "still-available")
    );
}

#[test]
fn unknown_modes_and_namespace_scoped_coordinates_are_not_reinterpreted() {
    for strictness in [Strictness::Strict, Strictness::Lenient] {
        let doc = open(
            &outline_action(
                r#"<Goto><Dest Type="CUSTOM" PageID="42" x:Left="99"><x:Top>80</x:Top><x:Wrapper><Left>70</Left></x:Wrapper></Dest></Goto>"#,
            ),
            strictness,
        );
        let ActionKind::Goto {
            destination: Some(dest),
            ..
        } = &doc.outline().unwrap()[0].actions[0].kind
        else {
            panic!()
        };
        assert_eq!(dest.mode, DestinationMode::Unknown("CUSTOM".into()));
        assert_eq!((dest.left, dest.top), (None, None));
        assert_eq!(doc.warnings().len(), 1);
        assert_eq!(doc.warnings()[0].code, WarningCode::NavigationUnsupported);
    }
}

#[test]
fn xml_depth_stays_a_global_open_time_limit_and_unknown_selected_nodes_are_budgeted() {
    let nav = outline_action(r#"<Goto><Dest Type="Fit" PageID="42"/></Goto>"#);
    let mut options = LoadOptions::default();
    options.limits.max_xml_depth = 5;
    assert!(matches!(
        Document::from_bytes(bytes(&nav), options),
        Err(rofd_core::Error::LimitExceeded(_))
    ));
    let mut options = LoadOptions::default();
    options.limits.max_page_objects = 3;
    let doc=Document::from_bytes(bytes(r#"<Outlines><OutlineElem Title="safe"/><x:Wrapper><x:Extension/></x:Wrapper></Outlines>"#),options).unwrap();
    assert!(matches!(
        doc.outline(),
        Err(rofd_core::Error::LimitExceeded(_))
    ));
    let mut options = LoadOptions::default();
    options.limits.max_page_objects = 0;
    assert!(Document::from_bytes(bytes(""), options)
        .unwrap()
        .outline()
        .unwrap()
        .is_empty());
}

#[test]
fn real_ofdrw_compat_fixture_retains_all_outline_destinations() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ofdrw-compat/fixtures/converter/z.ofd");
    let doc = Document::open(path, LoadOptions::default()).unwrap();
    let nodes = doc.outline().unwrap();
    assert_eq!(nodes.len(), 5);
    assert!(nodes[0].title.starts_with("11.4字型变换"));
    for (index, node) in nodes.iter().enumerate() {
        assert_eq!(node.parent, None);
        assert_eq!(
            node.next_sibling,
            (index + 1 < nodes.len()).then_some(index + 1)
        );
        let ActionKind::Goto {
            destination: Some(dest),
            bookmark: None,
        } = &node.actions[0].kind
        else {
            panic!()
        };
        assert_eq!(dest.page_id, [10090, 10090, 10091, 10092, 10093][index]);
        let page = doc.page(dest.page_index.unwrap()).unwrap();
        assert_eq!(page.object_id(), dest.page_id);
    }
    let warnings = doc.warnings();
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.code == WarningCode::NavigationCompatibility)
            .count(),
        1
    );
    assert_eq!(doc.clone().outline().unwrap().as_ptr(), nodes.as_ptr());
    assert_eq!(doc.warnings(), warnings);
    let strict = Document::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/ofdrw-compat/fixtures/converter/z.ofd"),
        LoadOptions {
            strictness: Strictness::Strict,
            ..LoadOptions::default()
        },
    )
    .unwrap();
    assert_eq!(strict.outline().unwrap().len(), 5);
}

#[test]
fn repeated_bookmark_modes_cannot_amplify_beyond_the_navigation_string_budget() {
    let mode = "X".repeat(16 * 1024);
    let actions = r#"<Action Event="CLICK"><Goto><Bookmark Name="b"/></Goto></Action>"#.repeat(128);
    let nav = format!(
        r#"<Outlines><OutlineElem Title="links"><Actions>{actions}</Actions></OutlineElem></Outlines><Bookmarks><Bookmark Name="b"><Dest Type="{mode}" PageID="42"/></Bookmark></Bookmarks>"#
    );
    for strictness in [Strictness::Strict, Strictness::Lenient] {
        let mut options = LoadOptions {
            strictness,
            ..LoadOptions::default()
        };
        options.limits.max_entry_size = 64 * 1024;
        let doc = Document::from_bytes(bytes(&nav), options).unwrap();
        for _ in 0..3 {
            assert!(matches!(
                doc.outline(),
                Err(rofd_core::Error::LimitExceeded(_))
            ));
            assert!(
                doc.warnings().is_empty(),
                "failed queries must not publish warnings"
            );
        }
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let clone = doc.clone();
                scope.spawn(move || {
                    assert!(matches!(
                        clone.outline(),
                        Err(rofd_core::Error::LimitExceeded(_))
                    ));
                    assert!(clone.warnings().is_empty());
                });
            }
        });
        doc.page(0).unwrap();
    }
}

#[test]
fn escaped_unknown_event_warnings_share_the_expanded_string_budget() {
    let event = "\u{7f}".repeat(4096);
    let nav = format!(
        r#"<Outlines><OutlineElem Title="x"><Actions><Action Event="{event}"><Sound/></Action></Actions></OutlineElem></Outlines>"#
    );
    for strictness in [Strictness::Strict, Strictness::Lenient] {
        let mut options = LoadOptions {
            strictness,
            ..LoadOptions::default()
        };
        // The encoded XML fits; copied event, warning messages and paths are
        // independently charged after XML entity decoding and debug escaping.
        options.limits.max_entry_size = 8 * 1024;
        let doc = Document::from_bytes(bytes(&nav), options).unwrap();
        assert!(matches!(
            doc.outline(),
            Err(rofd_core::Error::LimitExceeded(_))
        ));
        assert!(doc.warnings().is_empty());
    }
}

#[test]
fn ignored_extension_names_cannot_amplify_a_shared_namespace_in_the_xml_arena() {
    let namespace = format!("urn:{}", "n".repeat(2048));
    for repeated in ["<e:Ignored/>", r#"<Ignored e:attribute="value"/>"#] {
        let nav = format!(
            r#"<Outlines xmlns:e="{namespace}">{}</Outlines>"#,
            repeated.repeat(64)
        );
        for strictness in [Strictness::Strict, Strictness::Lenient] {
            let mut options = LoadOptions {
                strictness,
                ..LoadOptions::default()
            };
            options.limits.max_entry_size = 16 * 1024;
            let doc = Document::from_bytes(bytes(&nav), options).unwrap();
            assert!(matches!(
                doc.outline(),
                Err(rofd_core::Error::LimitExceeded(_))
            ));
            assert!(doc.warnings().is_empty());
        }
    }
}

#[test]
fn strict_diagnostic_escaping_cannot_bypass_or_downgrade_the_string_limit() {
    let invalid_id = "\u{7f}".repeat(4096);
    let nav = outline_action(&format!(
        r#"<Goto><Dest Type="Fit" PageID="{invalid_id}"/></Goto>"#
    ));
    for strictness in [Strictness::Strict, Strictness::Lenient] {
        let mut options = LoadOptions {
            strictness,
            ..LoadOptions::default()
        };
        options.limits.max_entry_size = 8 * 1024;
        let doc = Document::from_bytes(bytes(&nav), options).unwrap();
        assert!(matches!(
            doc.outline(),
            Err(rofd_core::Error::LimitExceeded(_))
        ));
        assert!(doc.warnings().is_empty());
    }
}

#[test]
fn expanded_entity_text_and_attribute_values_are_charged_to_the_arena() {
    let references = "&expanded;".repeat(16);
    for payload in [
        format!("<Ignored>{references}</Ignored>"),
        format!(r#"<Ignored Value="{references}"/>"#),
    ] {
        let xml = format!(
            r#"<!DOCTYPE Document [<!ENTITY expanded "{}">]><Document><CommonData><PageArea><PhysicalBox>0 0 10 10</PhysicalBox></PageArea></CommonData><Pages><Page ID="42" BaseLoc="Pages/Page_0/Content.xml"/></Pages><Outlines>{payload}</Outlines></Document>"#,
            "E".repeat(1024)
        );
        let mut options = LoadOptions::default();
        options.limits.max_entry_size = 4 * 1024;
        let doc = Document::from_bytes(support::ofd_with_entries(&xml, &[]), options).unwrap();
        assert!(matches!(
            doc.outline(),
            Err(rofd_core::Error::LimitExceeded(_))
        ));
        assert!(doc.warnings().is_empty());
    }
}
