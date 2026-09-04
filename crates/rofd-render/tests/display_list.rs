use std::io::{Cursor, Write};

use rofd_core::{
    Color, Document, FillRule, LoadOptions, PathData, Transform, UnsupportedObjectKind,
};
use rofd_render::{Command, DisplayList};
use zip::{write::SimpleFileOptions, ZipWriter};

fn minimal_ofd(page_xml: &str) -> Vec<u8> {
    let entries = [
        (
            "OFD.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">
  <ofd:DocBody>
    <ofd:DocInfo><ofd:DocID>display-list-fixture</ofd:DocID></ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
  </ofd:DocBody>
</ofd:OFD>"#,
        ),
        (
            "Doc_0/Document.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    <ofd:MaxUnitID>100</ofd:MaxUnitID>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="100" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#,
        ),
        ("Doc_0/Pages/Page_0/Content.xml", page_xml),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn open_page(content: &str) -> rofd_core::Page {
    let page_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  {content}
</ofd:Page>"#
    );
    Document::from_bytes(minimal_ofd(&page_xml), LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap()
}

fn path(id: u64, data: &str) -> String {
    format!(
        r#"<ofd:PathObject ID="{id}" Boundary="0 0 10 10">
  <ofd:AbbreviatedData>{data}</ofd:AbbreviatedData>
</ofd:PathObject>"#
    )
}

fn drawn_paths(display_list: &DisplayList) -> Vec<&PathData> {
    display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::DrawPath(path) => Some(path),
            _ => None,
        })
        .collect()
}

#[test]
fn lowers_path_to_exact_backend_neutral_command_sequence() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="-2 3 4 5" CTM="1 0.5 0 1 6 7" Stroke="true" Fill="true"
  LineWidth="1.25">
  <ofd:StrokeColor Value="10 20 30" Alpha="40"/>
  <ofd:FillColor Value="50 60 70" Alpha="80"/>
  <ofd:AbbreviatedData>M 1 2 L 3 4 C</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );

    let display_list = DisplayList::from_page(&page).unwrap();

    assert_eq!(
        display_list.commands(),
        [
            Command::Save,
            Command::ConcatTransform(Transform::new(1.0, 0.0, 0.0, 1.0, -2.0, 3.0).unwrap()),
            Command::ConcatTransform(Transform::new(1.0, 0.5, 0.0, 1.0, 6.0, 7.0).unwrap()),
            Command::SetStroke(Some(Color {
                red: 10,
                green: 20,
                blue: 30,
                alpha: 40,
            })),
            Command::SetFill(Some(Color {
                red: 50,
                green: 60,
                blue: 70,
                alpha: 80,
            })),
            Command::SetLineWidth(1.25),
            Command::DrawPath(PathData::parse("M 1 2 L 3 4 C").unwrap()),
            Command::Restore,
        ]
    );
    assert!(display_list.diagnostics().is_empty());
}

#[test]
fn always_emits_boundary_translation_but_omits_identity_object_transform() {
    let page = open_page(&format!(
        "<ofd:Content><ofd:Layer ID=\"1\">{}</ofd:Layer></ofd:Content>",
        path(2, "M 0 0")
    ));

    let display_list = DisplayList::from_page(&page).unwrap();

    assert_eq!(display_list.commands().len(), 7);
    assert_eq!(display_list.commands()[0], Command::Save);
    assert_eq!(
        display_list.commands()[1],
        Command::ConcatTransform(Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap())
    );
    assert_eq!(
        display_list.commands()[2],
        Command::SetStroke(Some(Color::BLACK))
    );
    assert_eq!(display_list.commands()[3], Command::SetFill(None));
    assert_eq!(display_list.commands()[4], Command::SetLineWidth(0.353));
    assert!(matches!(display_list.commands()[5], Command::DrawPath(_)));
    assert_eq!(display_list.commands()[6], Command::Restore);
}

#[test]
fn recursively_flattens_nested_groups_in_source_order_without_group_state() {
    let page = open_page(&format!(
        r#"<ofd:Content><ofd:Layer ID="1">
  {}
  <ofd:PageBlock ID="3">
    {}
    <ofd:PageBlock ID="5">{}</ofd:PageBlock>
  </ofd:PageBlock>
  {}
</ofd:Layer></ofd:Content>"#,
        path(2, "M 2 0"),
        path(4, "M 4 0"),
        path(6, "M 6 0"),
        path(7, "M 7 0"),
    ));

    let display_list = DisplayList::from_page(&page).unwrap();

    assert_eq!(display_list.commands().len(), 4 * 7);
    assert_eq!(
        drawn_paths(&display_list),
        ["M 2 0", "M 4 0", "M 6 0", "M 7 0"]
            .map(|data| PathData::parse(data).unwrap())
            .iter()
            .collect::<Vec<_>>()
    );
}

#[test]
fn preserves_current_layer_source_order() {
    let page = open_page(&format!(
        r#"<ofd:Content>
  <ofd:Layer ID="1" Type="Foreground">{}</ofd:Layer>
  <ofd:Layer ID="3" Type="Background">{}</ofd:Layer>
</ofd:Content>"#,
        path(2, "M 2 0"),
        path(4, "M 4 0"),
    ));

    let display_list = DisplayList::from_page(&page).unwrap();

    assert_eq!(
        drawn_paths(&display_list),
        ["M 2 0", "M 4 0"]
            .map(|data| PathData::parse(data).unwrap())
            .iter()
            .collect::<Vec<_>>()
    );
}

#[test]
fn unsupported_nodes_produce_diagnostics_and_no_drawing_commands() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1">
  <ofd:TextObject ID="2"/>
  <ofd:PageBlock ID="3">
    <ofd:ImageObject ID="4"/>
    <ofd:CompositeObject ID="5"/>
  </ofd:PageBlock>
</ofd:Layer></ofd:Content>"#,
    );

    let display_list = DisplayList::from_page(&page).unwrap();

    assert!(display_list.commands().is_empty());
    assert_eq!(display_list.diagnostics().len(), 3);
    for (diagnostic, (object_id, kind)) in display_list.diagnostics().iter().zip([
        (2, UnsupportedObjectKind::Text),
        (4, UnsupportedObjectKind::Image),
        (5, UnsupportedObjectKind::Composite),
    ]) {
        assert_eq!(diagnostic.object_id(), object_id);
        assert_eq!(diagnostic.kind(), kind);
        assert!(!diagnostic.message().is_empty());
    }
}

#[test]
fn empty_page_produces_an_empty_display_list() {
    let page = open_page("");

    let display_list = DisplayList::from_page(&page).unwrap();

    assert!(display_list.commands().is_empty());
    assert!(display_list.diagnostics().is_empty());
}

#[test]
fn clip_path_variant_has_the_stable_public_shape_but_is_not_emitted_yet() {
    let clip = Command::ClipPath {
        path: PathData::parse("M 0 0 L 1 1").unwrap(),
        rule: FillRule::EvenOdd,
    };
    assert!(matches!(
        clip,
        Command::ClipPath {
            rule: FillRule::EvenOdd,
            ..
        }
    ));
}
