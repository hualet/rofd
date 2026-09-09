mod support;

use rofd_core::{Document, LoadOptions, TextCharFlags, TextGeometryPrecision};

fn page_with_text(content: &str) -> rofd_core::Page {
    let page_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1">{content}</ofd:Layer></ofd:Content>
</ofd:Page>"#
    );
    Document::from_bytes(support::minimal_ofd(&page_xml), LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap()
}

#[test]
fn page_text_exposes_utf8_ranges_source_ids_and_conservative_geometry() {
    const SYNTHESIZED_SEPARATOR_BITS: u32 = TextCharFlags::SYNTHESIZED_SEPARATOR.bits();
    const HAS_SYNTHESIZED_SEPARATOR: bool =
        TextCharFlags::SYNTHESIZED_SEPARATOR.contains(TextCharFlags::SYNTHESIZED_SEPARATOR);

    assert_eq!(SYNTHESIZED_SEPARATOR_BITS, 1);
    assert!(HAS_SYNTHESIZED_SEPARATOR);

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
