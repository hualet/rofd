use std::io::{Cursor, Write};

use rofd_core::{
    Color, Document, FillRule, LayerSource, LoadOptions, PathData, Point, Transform,
    UnsupportedObjectKind,
};
use rofd_render::{ClipPath, Command, DisplayList};
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
    <ofd:DocumentRes>Res.xml</ofd:DocumentRes>
    <ofd:MaxUnitID>100</ofd:MaxUnitID>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="100" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#,
        ),
        ("Doc_0/Pages/Page_0/Content.xml", page_xml),
        (
            "Doc_0/Res.xml",
            r#"<Res><Fonts><Font ID="900" FontName="Fixture"/></Fonts><MultiMedias><MultiMedia ID="901" Type="Image" Format="PNG"><MediaFile>unused.png</MediaFile></MultiMedia></MultiMedias></Res>"#,
        ),
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
fn template_effective_layer_order_and_sources_flow_into_display_commands_and_diagnostics() {
    let entries = [
        (
            "OFD.xml",
            r#"<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016"><ofd:DocBody><ofd:DocInfo><ofd:DocID>templates</ofd:DocID></ofd:DocInfo><ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>"#,
        ),
        (
            "Doc_0/Document.xml",
            r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:PageArea><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:PageArea><ofd:DocumentRes>Res.xml</ofd:DocumentRes><ofd:TemplatePage ID="10" BaseLoc="Templates/Back.xml"/><ofd:TemplatePage ID="20" BaseLoc="Templates/Front.xml" ZOrder="Foreground"/></ofd:CommonData><ofd:Pages><ofd:Page ID="100" BaseLoc="Pages/Page.xml"/></ofd:Pages></ofd:Document>"#,
        ),
        (
            "Doc_0/Pages/Page.xml",
            r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>0 0 20 20</ofd:PhysicalBox></ofd:Area><ofd:Template TemplateID="10"/><ofd:Template TemplateID="20"/><ofd:Content><ofd:Layer ID="100"><ofd:PathObject ID="101" Boundary="0 0 10 10"><ofd:AbbreviatedData>M 3 0</ofd:AbbreviatedData></ofd:PathObject><ofd:PageBlock ID="102"><ofd:TextObject ID="103" Boundary="0 0 1 1" Font="900" Size="1"><ofd:TextCode X="0" Y="0">T</ofd:TextCode></ofd:TextObject></ofd:PageBlock></ofd:Layer></ofd:Content></ofd:Page>"#,
        ),
        (
            "Doc_0/Templates/Back.xml",
            r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="10"><ofd:PathObject ID="11" Boundary="0 0 10 10"><ofd:AbbreviatedData>M 1 0</ofd:AbbreviatedData></ofd:PathObject><ofd:PageBlock ID="13"><ofd:TextObject ID="12" Boundary="0 0 1 1" Font="900" Size="1"><ofd:TextCode X="0" Y="0">T</ofd:TextCode></ofd:TextObject></ofd:PageBlock></ofd:Layer></ofd:Content></ofd:Page>"#,
        ),
        (
            "Doc_0/Templates/Front.xml",
            r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Content><ofd:Layer ID="20"><ofd:PathObject ID="21" Boundary="0 0 10 10"><ofd:AbbreviatedData>M 5 0</ofd:AbbreviatedData></ofd:PathObject></ofd:Layer></ofd:Content></ofd:Page>"#,
        ),
        (
            "Doc_0/Res.xml",
            r#"<Res><Fonts><Font ID="900" FontName="Fixture"/></Fonts></Res>"#,
        ),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let page = Document::from_bytes(bytes, LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap();

    assert_eq!(page.layers()[0].source(), LayerSource::Template(10));
    assert_eq!(page.layers()[1].source(), LayerSource::Page);
    assert_eq!(page.layers()[2].source(), LayerSource::Template(20));
    let display = DisplayList::from_page(&page).unwrap();
    let x_coordinates = drawn_paths(&display)
        .iter()
        .map(|path| match path.commands()[0] {
            rofd_core::PathCommand::MoveTo(point) => point.x(),
            _ => panic!("expected MoveTo"),
        })
        .collect::<Vec<_>>();
    assert_eq!(x_coordinates, vec![1.0, 3.0, 5.0]);
    assert_eq!(display.diagnostics().len(), 2);
    assert_eq!(display.diagnostics()[0].object_id(), 12);
    assert_eq!(display.diagnostics()[0].source(), LayerSource::Template(10));
    assert_eq!(display.diagnostics()[1].object_id(), 103);
    assert_eq!(display.diagnostics()[1].source(), LayerSource::Page);
}

#[test]
fn lowers_path_to_exact_backend_neutral_command_sequence() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="-2 3 4 5" CTM="1 0.5 0 1 6 7" Stroke="true" Fill="true"
  LineWidth="1.25" Rule="Even-Odd">
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
            Command::ConcatTransform(Transform::new(1.0, 0.5, 0.0, 1.0, 4.0, 10.0).unwrap()),
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
            Command::SetFillRule(FillRule::EvenOdd),
            Command::SetLineWidth(1.25),
            Command::DrawPath(PathData::parse("M 1 2 L 3 4 C").unwrap()),
            Command::Restore,
        ]
    );
    assert!(display_list.diagnostics().is_empty());
}

#[test]
fn identity_object_transform_leaves_boundary_translation_as_the_effective_transform() {
    let page = open_page(&format!(
        "<ofd:Content><ofd:Layer ID=\"1\">{}</ofd:Layer></ofd:Content>",
        path(2, "M 0 0")
    ));

    let display_list = DisplayList::from_page(&page).unwrap();

    assert_eq!(display_list.commands().len(), 8);
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
    assert_eq!(
        display_list.commands()[4],
        Command::SetFillRule(FillRule::NonZero)
    );
    assert_eq!(display_list.commands()[5], Command::SetLineWidth(0.353));
    assert!(matches!(display_list.commands()[6], Command::DrawPath(_)));
    assert_eq!(display_list.commands()[7], Command::Restore);
}

#[test]
fn precomposed_transform_applies_non_commuting_ctm_before_boundary_translation() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="10 20 4 5" CTM="2 1 0.5 3 4 5">
  <ofd:AbbreviatedData>M 2 3</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );

    let display_list = DisplayList::from_page(&page).unwrap();
    let Command::ConcatTransform(transform) = display_list.commands()[1] else {
        panic!("expected a precomposed object-to-page transform");
    };

    assert_eq!(
        transform.apply(Point::new(2.0, 3.0).unwrap()).unwrap(),
        Point::new(19.5, 36.0).unwrap()
    );
    assert_eq!(
        display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::ConcatTransform(_)))
            .count(),
        1
    );
}

#[test]
fn preserves_even_odd_fill_rule() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject
  ID="2" Boundary="0 0 4 5" Rule="Even-Odd">
  <ofd:AbbreviatedData>M 0 0 L 1 1</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );

    let display_list = DisplayList::from_page(&page).unwrap();

    assert!(display_list
        .commands()
        .contains(&Command::SetFillRule(FillRule::EvenOdd)));
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

    assert_eq!(display_list.commands().len(), 4 * 8);
    assert_eq!(
        drawn_paths(&display_list),
        ["M 2 0", "M 4 0", "M 6 0", "M 7 0"]
            .map(|data| PathData::parse(data).unwrap())
            .iter()
            .collect::<Vec<_>>()
    );
}

#[test]
fn direct_page_layers_follow_effective_category_order() {
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
        ["M 4 0", "M 2 0"]
            .map(|data| PathData::parse(data).unwrap())
            .iter()
            .collect::<Vec<_>>()
    );
}

#[test]
fn unsupported_nodes_produce_diagnostics_and_no_drawing_commands() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1">
  <ofd:TextObject ID="2" Boundary="0 0 1 1" Font="900" Size="1"><ofd:TextCode X="0" Y="0">T</ofd:TextCode></ofd:TextObject>
  <ofd:PageBlock ID="3">
    <ofd:ImageObject ID="4" Boundary="0 0 1 1" ResourceID="901"/>
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
fn clip_path_variant_represents_one_union_operand() {
    let clip = Command::ClipPath {
        paths: vec![ClipPath::new(
            Transform::IDENTITY,
            PathData::parse("M 0 0 L 1 1").unwrap(),
        )],
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

#[test]
fn lowers_clip_transforms_exactly_with_and_without_the_object_ctm() {
    for (trans_flag, expected) in [
        ("false", Point::new(79.0, 221.0).unwrap()),
        ("true", Point::new(72.5, 247.0).unwrap()),
    ] {
        let page = open_page(&format!(
            r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="100 200 30 40" CTM="2 1 0.5 3 4 5">
  <ofd:Clips TransFlag="{trans_flag}"><ofd:Clip><ofd:Area CTM="0 1 -1 0 7 8">
    <ofd:Path Boundary="10 20 30 40" CTM="2 0 0 3 1 2" Fill="true" Stroke="false">
      <ofd:AbbreviatedData>M 1 2</ofd:AbbreviatedData>
    </ofd:Path>
  </ofd:Area></ofd:Clip></ofd:Clips>
  <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#
        ));

        let display_list = DisplayList::from_page(&page).unwrap();
        let Command::ClipPath { paths, .. } = &display_list.commands()[1] else {
            panic!("expected clip command before drawing transform");
        };
        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths[0]
                .transform()
                .apply(Point::new(1.0, 2.0).unwrap())
                .unwrap(),
            expected,
            "TransFlag={trans_flag}"
        );
    }
}

#[test]
fn unions_areas_per_clip_and_intersects_clips_in_source_order_before_drawing() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10">
    <ofd:Clips>
    <ofd:Clip>
      <ofd:Area CTM="1 0 0 1 2 0"><ofd:Path Boundary="0 0 4 4" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 4 0 L 4 4 L 0 4 C</ofd:AbbreviatedData></ofd:Path></ofd:Area>
      <ofd:Area CTM="1 0 0 1 4 0"><ofd:Path Boundary="0 0 4 4" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0 L 0 4 L 4 4 L 4 0 C</ofd:AbbreviatedData></ofd:Path></ofd:Area>
    </ofd:Clip>
    <ofd:Clip><ofd:Area><ofd:Path Boundary="0 0 2 2" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 3 0</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip>
  </ofd:Clips>
  <ofd:AbbreviatedData>M 4 0</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );

    let display_list = DisplayList::from_page(&page).unwrap();
    assert_eq!(display_list.commands()[0], Command::Save);
    let Command::ClipPath { paths: first, rule } = &display_list.commands()[1] else {
        panic!("expected first intersection operand");
    };
    assert_eq!(*rule, FillRule::NonZero);
    assert_eq!(first.len(), 2);
    assert_eq!(
        first[0].path(),
        &PathData::parse("M 0 0 L 4 0 L 4 4 L 0 4 C").unwrap()
    );
    assert_eq!(
        first[1].path(),
        &PathData::parse("M 0 0 L 0 4 L 4 4 L 4 0 C").unwrap()
    );
    assert_eq!(
        first[0].transform(),
        Transform::new(1.0, 0.0, 0.0, 1.0, 2.0, 0.0).unwrap()
    );
    assert_eq!(
        first[1].transform(),
        Transform::new(1.0, 0.0, 0.0, 1.0, 4.0, 0.0).unwrap()
    );
    let Command::ClipPath { paths: second, .. } = &display_list.commands()[2] else {
        panic!("expected second intersection operand");
    };
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].path(), &PathData::parse("M 3 0").unwrap());
    assert!(matches!(
        display_list.commands()[3],
        Command::ConcatTransform(_)
    ));
    assert!(matches!(display_list.commands()[8], Command::DrawPath(_)));
    assert_eq!(display_list.commands()[9], Command::Restore);
}

#[test]
fn reports_clip_transform_overflow_as_an_invalid_model() {
    let page = open_page(
        r#"<ofd:Content><ofd:Layer ID="1"><ofd:PathObject ID="2" Boundary="0 0 10 10">
  <ofd:Clips><ofd:Clip><ofd:Area CTM="1e308 0 0 1 0 0"><ofd:Path Boundary="0 0 1 1" CTM="1e308 0 0 1 0 0" Fill="true" Stroke="false"><ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData></ofd:Path></ofd:Area></ofd:Clip></ofd:Clips>
  <ofd:AbbreviatedData>M 0 0</ofd:AbbreviatedData>
</ofd:PathObject></ofd:Layer></ofd:Content>"#,
    );

    let error = DisplayList::from_page(&page).unwrap_err();
    assert!(matches!(
        error,
        rofd_render::Error::InvalidModel {
            field: "clip transform",
            ..
        }
    ));
}
