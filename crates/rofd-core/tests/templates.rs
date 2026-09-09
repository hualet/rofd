use std::io::{Cursor, Write};
use std::sync::{Arc, Barrier};

use rofd_core::{Document, Error, LayerSource, LayerType, LoadOptions, WarningCode};
use zip::{write::SimpleFileOptions, ZipWriter};

fn archive(document: &str, page: &str, templates: &[(&str, &str)]) -> Vec<u8> {
    archive_with_pages(document, &[("Doc_0/Pages/Page.xml", page)], templates)
}

fn archive_with_pages(
    document: &str,
    pages: &[(&str, &str)],
    templates: &[(&str, &str)],
) -> Vec<u8> {
    let ofd = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody>
  <ofd:DocInfo><ofd:DocID>templates</ofd:DocID></ofd:DocInfo>
  <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
</ofd:DocBody></ofd:OFD>"#;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in [("OFD.xml", ofd), ("Doc_0/Document.xml", document)]
        .into_iter()
        .chain(pages.iter().copied())
        .chain(templates.iter().copied())
    {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn document(common_templates: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    {common_templates}
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="900" BaseLoc="Pages/Page.xml"/></ofd:Pages>
</ofd:Document>"#
    )
}

fn page(area: Option<&str>, references: &str, layers: &str) -> String {
    let area = area
        .map(|value| format!("<ofd:Area><ofd:PhysicalBox>{value}</ofd:PhysicalBox></ofd:Area>"))
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">{area}{references}<ofd:Content>{layers}</ofd:Content></ofd:Page>"#
    )
}

fn template(references: &str, layers: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">{references}<ofd:Content>{layers}</ofd:Content></ofd:Page>"#
    )
}

fn layer(id: u64, kind: &str, object_id: u64) -> String {
    format!(
        r#"<ofd:Layer ID="{id}" Type="{kind}"><ofd:PathObject ID="{object_id}" Boundary="0 0 1 1"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer>"#
    )
}

fn path_layer(id: u64, commands: &str) -> String {
    format!(
        r#"<ofd:Layer ID="{id}"><ofd:PathObject ID="{}" Boundary="0 0 10 10"><ofd:AbbreviatedData>{commands}</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer>"#,
        id + 10_000
    )
}

fn template_decl(id: &str, base: &str, z_order: Option<&str>) -> String {
    let z_order = z_order
        .map(|value| format!(r#" ZOrder="{value}""#))
        .unwrap_or_default();
    format!(r#"<ofd:TemplatePage ID="{id}" BaseLoc="{base}"{z_order}/>"#)
}

fn template_ref(id: &str, z_order: Option<&str>) -> String {
    let z_order = z_order
        .map(|value| format!(r#" ZOrder="{value}""#))
        .unwrap_or_default();
    format!(r#"<ofd:Template TemplateID="{id}"{z_order}/>"#)
}

fn sources(page: &rofd_core::Page) -> Vec<(u64, LayerType, LayerSource)> {
    page.layers()
        .iter()
        .map(|layer| (layer.object_id(), layer.kind(), layer.source()))
        .collect()
}

#[test]
fn merges_default_and_overridden_templates_around_stably_grouped_page_layers() {
    let declarations = [
        template_decl("10", "Templates/Background.xml", None),
        template_decl("20", "Templates/Foreground.xml", Some("Foreground")),
        template_decl("30", "Templates/Override.xml", Some("Foreground")),
    ]
    .join("");
    let references = [
        template_ref("10", None),
        template_ref("20", None),
        template_ref("30", Some("Background")),
    ]
    .join("");
    let direct = [
        layer(901, "Foreground", 1901),
        layer(902, "Body", 1902),
        layer(903, "Background", 1903),
        layer(904, "Body", 1904),
        layer(905, "Background", 1905),
    ]
    .join("");
    let bytes = archive(
        &document(&declarations),
        &page(Some("10 20 30 40"), &references, &direct),
        &[
            (
                "Doc_0/Templates/Background.xml",
                &template("", &layer(101, "Body", 1101)),
            ),
            (
                "Doc_0/Templates/Foreground.xml",
                &template("", &layer(201, "Body", 1201)),
            ),
            (
                "Doc_0/Templates/Override.xml",
                &template("", &layer(301, "Body", 1301)),
            ),
        ],
    );

    let document_handle = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document_handle.page(0).unwrap();

    assert_eq!(page.size().x, 10.0);
    assert_eq!(page.size().y, 20.0);
    assert_eq!(page.size().width, 30.0);
    assert_eq!(page.size().height, 40.0);
    assert_eq!(
        sources(&page),
        vec![
            (101, LayerType::Body, LayerSource::Template(10)),
            (301, LayerType::Body, LayerSource::Template(30)),
            (903, LayerType::Background, LayerSource::Page),
            (905, LayerType::Background, LayerSource::Page),
            (902, LayerType::Body, LayerSource::Page),
            (904, LayerType::Body, LayerSource::Page),
            (901, LayerType::Foreground, LayerSource::Page),
            (201, LayerType::Body, LayerSource::Template(20)),
        ]
    );
}

#[test]
fn nested_templates_use_their_own_effective_order_and_preserve_source_order() {
    let declarations = [
        template_decl("10", "Templates/Parent.xml", None),
        template_decl("20", "Templates/NestedBack.xml", Some("Background")),
        template_decl("30", "Templates/NestedFront.xml", Some("Foreground")),
    ]
    .join("");
    let parent_refs = [template_ref("30", None), template_ref("20", None)].join("");
    let parent_layers = [
        layer(101, "Foreground", 1101),
        layer(102, "Background", 1102),
        layer(103, "Body", 1103),
    ]
    .join("");
    let bytes = archive(
        &document(&declarations),
        &page(Some("0 0 20 20"), &template_ref("10", None), ""),
        &[
            (
                "Doc_0/Templates/Parent.xml",
                &template(&parent_refs, &parent_layers),
            ),
            (
                "Doc_0/Templates/NestedBack.xml",
                &template("", &layer(201, "Body", 1201)),
            ),
            (
                "Doc_0/Templates/NestedFront.xml",
                &template("", &layer(301, "Body", 1301)),
            ),
        ],
    );

    let page = Document::from_bytes(bytes, LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap();

    assert_eq!(
        sources(&page),
        vec![
            (201, LayerType::Body, LayerSource::Template(20)),
            (102, LayerType::Background, LayerSource::Template(10)),
            (103, LayerType::Body, LayerSource::Template(10)),
            (101, LayerType::Foreground, LayerSource::Template(10)),
            (301, LayerType::Body, LayerSource::Template(30)),
        ]
    );
}

#[test]
fn rejects_unknown_duplicate_and_invalid_template_metadata_with_context() {
    let duplicate = document(
        &[
            template_decl("10", "Templates/A.xml", None),
            template_decl("10", "Templates/B.xml", None),
        ]
        .join(""),
    );
    let bytes = archive(&duplicate, &page(Some("0 0 20 20"), "", ""), &[]);
    assert!(matches!(
        Document::from_bytes(bytes, LoadOptions::default()),
        Err(Error::InvalidStructure { ref path, ref message })
            if path == "Doc_0/Document.xml" && message.contains("duplicate template ID 10")
    ));

    for (declaration_z, reference_z, expected_path) in [
        (Some("Body"), None, "Doc_0/Document.xml"),
        (None, Some("background"), "Doc_0/Pages/Page.xml"),
    ] {
        let declaration = template_decl("10", "Templates/A.xml", declaration_z);
        let bytes = archive(
            &document(&declaration),
            &page(Some("0 0 20 20"), &template_ref("10", reference_z), ""),
            &[("Doc_0/Templates/A.xml", &template("", ""))],
        );
        let result = Document::from_bytes(bytes, LoadOptions::default())
            .and_then(|document| document.page(0));
        assert!(matches!(
            result,
            Err(Error::InvalidValue { field: "template ZOrder", ref path, .. })
                if path.as_deref() == Some(expected_path)
        ));
    }

    let bytes = archive(
        &document(""),
        &page(Some("0 0 20 20"), &template_ref("999", None), ""),
        &[],
    );
    assert!(matches!(
        Document::from_bytes(bytes, LoadOptions::default()).and_then(|document| document.page(0)),
        Err(Error::InvalidStructure { ref path, ref message })
            if path == "Doc_0/Pages/Page.xml" && message.contains("unknown template ID 999")
    ));
}

#[test]
fn rejects_invalid_template_ids_and_unsafe_relative_base_locations() {
    for id in ["0", "not-an-id"] {
        let bytes = archive(
            &document(&template_decl(id, "Templates/A.xml", None)),
            &page(Some("0 0 20 20"), "", ""),
            &[],
        );
        assert!(matches!(
            Document::from_bytes(bytes, LoadOptions::default()),
            Err(Error::InvalidValue { field: "template ID", ref path, .. })
                if path.as_deref() == Some("Doc_0/Document.xml")
        ));
    }

    for id in ["0", "not-an-id"] {
        let bytes = archive(
            &document(&template_decl("10", "Templates/A.xml", None)),
            &page(Some("0 0 20 20"), &template_ref(id, None), ""),
            &[("Doc_0/Templates/A.xml", &template("", ""))],
        );
        assert!(matches!(
            Document::from_bytes(bytes, LoadOptions::default())
                .and_then(|document| document.page(0)),
            Err(Error::InvalidValue { field: "template ID", ref path, .. })
                if path.as_deref() == Some("Doc_0/Pages/Page.xml")
        ));
    }

    let bytes = archive(
        &document(&template_decl("10", "../../outside.xml", None)),
        &page(Some("0 0 20 20"), "", ""),
        &[],
    );
    assert!(matches!(
        Document::from_bytes(bytes, LoadOptions::default()),
        Err(Error::InvalidValue {
            field: "package path",
            ..
        })
    ));
}

#[test]
fn resolves_normalized_template_base_locations_relative_to_document_xml() {
    let bytes = archive(
        &document(&template_decl("10", "./Templates/Nested/../A.xml", None)),
        &page(Some("0 0 20 20"), &template_ref("10", None), ""),
        &[(
            "Doc_0/Templates/A.xml",
            &template("", &layer(101, "Body", 1101)),
        )],
    );

    let page = Document::from_bytes(bytes, LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap();
    assert_eq!(
        sources(&page),
        vec![(101, LayerType::Body, LayerSource::Template(10))]
    );
}

#[test]
fn detects_direct_and_indirect_template_cycles_with_a_useful_chain() {
    for (root, entries, expected) in [
        (
            "10",
            vec![(
                "Doc_0/Templates/A.xml",
                template(&template_ref("10", None), ""),
            )],
            "10 -> 10",
        ),
        (
            "10",
            vec![
                (
                    "Doc_0/Templates/A.xml",
                    template(&template_ref("20", None), ""),
                ),
                (
                    "Doc_0/Templates/B.xml",
                    template(&template_ref("10", None), ""),
                ),
            ],
            "10 -> 20 -> 10",
        ),
    ] {
        let declarations = entries
            .iter()
            .enumerate()
            .map(|(index, (path, _))| {
                template_decl(
                    if index == 0 { "10" } else { "20" },
                    path.strip_prefix("Doc_0/").unwrap(),
                    None,
                )
            })
            .collect::<String>();
        let owned = entries
            .iter()
            .map(|(path, xml)| (*path, xml.as_str()))
            .collect::<Vec<_>>();
        let bytes = archive(
            &document(&declarations),
            &page(Some("0 0 20 20"), &template_ref(root, None), ""),
            &owned,
        );
        assert!(matches!(
            Document::from_bytes(bytes, LoadOptions::default()).and_then(|document| document.page(0)),
            Err(Error::InvalidStructure { ref message, .. }) if message.contains(expected)
        ));
    }
}

#[test]
fn repeated_references_expand_and_count_deterministically() {
    let declarations = template_decl("10", "Templates/A.xml", None);
    let references = [template_ref("10", None), template_ref("10", None)].join("");
    let template_xml = template("", &layer(101, "Body", 1101));
    let bytes = archive(
        &document(&declarations),
        &page(Some("0 0 20 20"), &references, ""),
        &[("Doc_0/Templates/A.xml", &template_xml)],
    );
    let page = Document::from_bytes(bytes.clone(), LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap();
    assert_eq!(sources(&page).len(), 2);
    assert_eq!(sources(&page)[0], sources(&page)[1]);

    let mut exact = LoadOptions::default();
    exact.limits.max_page_objects = 6;
    assert_eq!(
        Document::from_bytes(bytes.clone(), exact)
            .unwrap()
            .page(0)
            .unwrap()
            .layers()
            .len(),
        2
    );

    let mut options = LoadOptions::default();
    options.limits.max_page_objects = 5;
    assert!(matches!(
        Document::from_bytes(bytes, options).and_then(|document| document.page(0)),
        Err(Error::LimitExceeded(ref message)) if message.contains("effective page object count 6")
    ));
}

#[test]
fn cached_templates_still_enforce_the_current_expansion_depth() {
    let declarations = [
        template_decl("10", "Templates/A.xml", None),
        template_decl("20", "Templates/B.xml", None),
        template_decl("30", "Templates/C.xml", None),
    ]
    .join("");
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    {declarations}
  </ofd:CommonData>
  <ofd:Pages>
    <ofd:Page ID="900" BaseLoc="Pages/Shallow.xml"/>
    <ofd:Page ID="901" BaseLoc="Pages/Deep.xml"/>
  </ofd:Pages>
</ofd:Document>"#
    );
    let shallow = page(Some("0 0 20 20"), &template_ref("10", None), "");
    let deep = page(Some("0 0 20 20"), &template_ref("30", None), "");
    let a = template(&template_ref("20", None), "");
    let b = template("", &layer(201, "Body", 1201));
    let c = template(&template_ref("10", None), "");
    let bytes = archive_with_pages(
        &document_xml,
        &[
            ("Doc_0/Pages/Shallow.xml", &shallow),
            ("Doc_0/Pages/Deep.xml", &deep),
        ],
        &[
            ("Doc_0/Templates/A.xml", &a),
            ("Doc_0/Templates/B.xml", &b),
            ("Doc_0/Templates/C.xml", &c),
        ],
    );

    let mut limited = LoadOptions::default();
    limited.limits.max_page_block_depth = 2;
    let document_handle = Document::from_bytes(bytes.clone(), limited).unwrap();
    document_handle.page(0).unwrap();
    assert!(matches!(
        document_handle.page(1),
        Err(Error::LimitExceeded(ref message))
            if message.contains("template reference depth 3")
    ));

    let mut exact = LoadOptions::default();
    exact.limits.max_page_block_depth = 3;
    let document_handle = Document::from_bytes(bytes, exact).unwrap();
    document_handle.page(0).unwrap();
    assert_eq!(document_handle.page(1).unwrap().layers().len(), 1);
}

#[test]
fn effective_expansion_enforces_path_command_and_template_depth_limits() {
    let declarations = [
        template_decl("10", "Templates/A.xml", None),
        template_decl("20", "Templates/B.xml", None),
    ]
    .join("");
    let a = template(&template_ref("20", None), &path_layer(101, "M 0 0 L 1 1"));
    let b = template("", &path_layer(201, "M 0 0 L 2 2"));
    let bytes = archive(
        &document(&declarations),
        &page(Some("0 0 20 20"), &template_ref("10", None), ""),
        &[("Doc_0/Templates/A.xml", &a), ("Doc_0/Templates/B.xml", &b)],
    );

    let mut path_limited = LoadOptions::default();
    path_limited.limits.max_path_commands = 3;
    assert!(matches!(
        Document::from_bytes(bytes.clone(), path_limited).and_then(|document| document.page(0)),
        Err(Error::LimitExceeded(ref message)) if message.contains("effective page path command count 4")
    ));

    let mut exact_objects = LoadOptions::default();
    exact_objects.limits.max_page_objects = 6;
    assert_eq!(
        Document::from_bytes(bytes.clone(), exact_objects)
            .unwrap()
            .page(0)
            .unwrap()
            .layers()
            .len(),
        2
    );

    let mut object_limited = LoadOptions::default();
    object_limited.limits.max_page_objects = 5;
    assert!(matches!(
        Document::from_bytes(bytes.clone(), object_limited)
            .and_then(|document| document.page(0)),
        Err(Error::LimitExceeded(ref message)) if message.contains("effective page object count 6")
    ));

    let mut depth_limited = LoadOptions::default();
    depth_limited.limits.max_page_block_depth = 1;
    assert!(matches!(
        Document::from_bytes(bytes, depth_limited).and_then(|document| document.page(0)),
        Err(Error::LimitExceeded(ref message)) if message.contains("template reference depth 2")
    ));
}

#[test]
fn malformed_or_individually_over_limit_templates_fail_without_page_warnings() {
    let declarations = template_decl("10", "Templates/A.xml", None);
    let missing_area_page = page(None, &template_ref("10", None), "");
    let malformed = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content>"#;
    let bytes = archive(
        &document(&declarations),
        &missing_area_page,
        &[("Doc_0/Templates/A.xml", malformed)],
    );
    let document_handle = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    assert!(document_handle.page(0).is_err());
    assert!(document_handle.warnings().is_empty());

    let oversized = template(
        "",
        &[layer(101, "Body", 1101), layer(102, "Body", 1102)].join(""),
    );
    let bytes = archive(
        &document(&declarations),
        &page(Some("0 0 20 20"), &template_ref("10", None), ""),
        &[("Doc_0/Templates/A.xml", &oversized)],
    );
    let mut options = LoadOptions::default();
    options.limits.max_page_objects = 3;
    assert!(matches!(
        Document::from_bytes(bytes, options).and_then(|document| document.page(0)),
        Err(Error::LimitExceeded(_))
    ));
}

#[test]
fn template_area_never_changes_real_page_size_or_publishes_fallback_warning() {
    let declarations = template_decl("10", "Templates/A.xml", None);
    let template_with_area = page(Some("50 60 70 80"), "", &layer(101, "Body", 1101));
    let bytes = archive(
        &document(&declarations),
        &page(None, &template_ref("10", None), ""),
        &[("Doc_0/Templates/A.xml", &template_with_area)],
    );
    let document_handle = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document_handle.page(0).unwrap();
    assert_eq!(page.size().width, 210.0);
    assert_eq!(page.size().height, 297.0);
    assert_eq!(document_handle.warnings().len(), 1);
    assert_eq!(
        document_handle.warnings()[0].code,
        WarningCode::PageAreaFallback
    );
    assert_eq!(document_handle.warnings()[0].path, "Doc_0/Pages/Page.xml");
}

#[test]
fn failed_template_initialization_is_retryable_without_partial_publication() {
    let declarations = template_decl("10", "Templates/A.xml", None);
    let bytes = archive(
        &document(&declarations),
        &page(None, &template_ref("10", None), ""),
        &[(
            "Doc_0/Templates/A.xml",
            r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content>"#,
        )],
    );
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    assert!(document.page(0).is_err());
    assert!(document.page(0).is_err());
    assert!(document.warnings().is_empty());
}

#[test]
fn unreferenced_missing_and_malformed_templates_remain_lazy() {
    let declarations = [
        template_decl("10", "Templates/Missing.xml", None),
        template_decl("20", "Templates/Malformed.xml", None),
    ]
    .join("");
    let bytes = archive(
        &document(&declarations),
        &page(Some("0 0 20 20"), "", &layer(901, "Body", 1901)),
        &[(
            "Doc_0/Templates/Malformed.xml",
            r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content>"#,
        )],
    );

    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    assert_eq!(sources(&document.page(0).unwrap()).len(), 1);
}

#[test]
fn invalid_values_in_template_content_report_the_template_path() {
    let declarations = template_decl("10", "Templates/A.xml", None);
    let invalid_template = template("", &layer(101, "body", 1101));
    let bytes = archive(
        &document(&declarations),
        &page(Some("0 0 20 20"), &template_ref("10", None), ""),
        &[("Doc_0/Templates/A.xml", &invalid_template)],
    );

    assert!(matches!(
        Document::from_bytes(bytes, LoadOptions::default()).and_then(|document| document.page(0)),
        Err(Error::InvalidValue {
            field: "layer type",
            ref path,
            ..
        }) if path.as_deref() == Some("Doc_0/Templates/A.xml")
    ));
}

#[test]
fn concurrent_pages_share_only_complete_template_cache_results() {
    let declarations = template_decl("10", "Templates/A.xml", None);
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    {declarations}
  </ofd:CommonData>
  <ofd:Pages>
    <ofd:Page ID="900" BaseLoc="Pages/First.xml"/>
    <ofd:Page ID="901" BaseLoc="Pages/Second.xml"/>
  </ofd:Pages>
</ofd:Document>"#
    );
    let page_xml = page(Some("0 0 20 20"), &template_ref("10", None), "");
    let template_xml = template("", &layer(101, "Body", 1101));
    let bytes = archive_with_pages(
        &document_xml,
        &[
            ("Doc_0/Pages/First.xml", &page_xml),
            ("Doc_0/Pages/Second.xml", &page_xml),
        ],
        &[("Doc_0/Templates/A.xml", &template_xml)],
    );
    let document_handle = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|index| {
            let document_handle = document_handle.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                sources(&document_handle.page(index).unwrap())
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    for handle in handles {
        assert_eq!(
            handle.join().unwrap(),
            vec![(101, LayerType::Body, LayerSource::Template(10))]
        );
    }
    assert!(document_handle.warnings().is_empty());

    let malformed = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content>"#;
    let bytes = archive_with_pages(
        &document_xml,
        &[
            ("Doc_0/Pages/First.xml", &page_xml),
            ("Doc_0/Pages/Second.xml", &page_xml),
        ],
        &[("Doc_0/Templates/A.xml", malformed)],
    );
    let document_handle = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|index| {
            let document_handle = document_handle.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                document_handle.page(index).is_err()
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    assert!(handles.into_iter().all(|handle| handle.join().unwrap()));
    assert!(document_handle.page(0).is_err());
    assert!(document_handle.warnings().is_empty());
}

#[test]
fn non_contiguous_template_page_declarations_all_register() {
    // ofdrw's reader/path_unstd.ofd splits its TemplatePage declarations
    // around other CommonData children.
    let declarations = [
        template_decl("10", "Templates/A.xml", None),
        "<ofd:MaxUnitID>99</ofd:MaxUnitID>".to_owned(),
        template_decl("20", "Templates/B.xml", None),
    ]
    .join("");
    let references = [template_ref("10", None), template_ref("20", None)].join("");
    let bytes = archive(
        &document(&declarations),
        &page(None, &references, &layer(901, "Body", 1901)),
        &[
            (
                "Doc_0/Templates/A.xml",
                &template("", &layer(101, "Body", 1101)),
            ),
            (
                "Doc_0/Templates/B.xml",
                &template("", &layer(102, "Body", 1102)),
            ),
        ],
    );
    let document = Document::from_bytes(bytes, LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    let ids: Vec<u64> = page
        .layers()
        .iter()
        .map(|layer| layer.object_id())
        .collect();
    assert_eq!(ids, [101, 102, 901]);
}
