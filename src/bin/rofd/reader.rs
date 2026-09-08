//! GUI-independent search and raster policy for the desktop reader.
use rofd_core::{Page, Rect};
use serde::Serialize;

pub const MAX_MATCHES: usize = 1000;
pub const MAX_PIXELS: f64 = 8_000_000.0;
pub const PX_PER_MM: f64 = 96.0 / 25.4;

#[derive(Clone, Debug, Serialize)]
pub struct SearchHit {
    pub page: usize,
    pub snippet: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn search_page(page: &Page, query: &str, hits: &mut Vec<SearchHit>) -> bool {
    fn visit(
        objects: &[rofd_core::PageObject],
        page: &Page,
        query: &str,
        hits: &mut Vec<SearchHit>,
    ) -> bool {
        let mut has_text = false;
        for object in objects {
            match object {
                rofd_core::PageObject::Text(text) => {
                    let content: String = text.runs().iter().map(|run| run.text()).collect();
                    has_text |= !content.trim().is_empty();
                    let boundary = text.boundary();
                    let area = page.size();
                    for snippet in
                        match_snippets(&content, query, MAX_MATCHES.saturating_sub(hits.len()))
                    {
                        hits.push(SearchHit {
                            page: page.index(),
                            snippet,
                            x: (boundary.x - area.x) / area.width,
                            y: (boundary.y - area.y) / area.height,
                            width: boundary.width / area.width,
                            height: boundary.height / area.height,
                        });
                    }
                }
                rofd_core::PageObject::Group(group) => {
                    has_text |= visit(group.objects(), page, query, hits)
                }
                _ => {}
            }
            if hits.len() >= MAX_MATCHES {
                break;
            }
        }
        has_text
    }
    let mut has_text = false;
    for layer in page.layers() {
        has_text |= visit(layer.objects(), page, query, hits);
        if hits.len() >= MAX_MATCHES {
            break;
        }
    }
    has_text
}

pub fn match_snippets(text: &str, query: &str, limit: usize) -> Vec<String> {
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }
    // Lowercasing can expand a scalar (e.g. İ); retain a map to original scalar
    // positions so snippets remain valid Unicode and preserve original spelling.
    let chars: Vec<char> = text.chars().collect();
    let mut folded = String::new();
    let mut positions = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        for lower in ch.to_lowercase() {
            folded.push(lower);
            positions.extend(std::iter::repeat_n(i, lower.len_utf8()));
        }
    }
    let query = query.to_lowercase();
    folded
        .match_indices(&query)
        .take(limit)
        .map(|(offset, matched)| {
            let start = positions[offset].saturating_sub(30);
            let end = (positions[offset + matched.len() - 1] + 31)
                .min(chars.len())
                .min(start + 86);
            let mut snippet = String::new();
            if start > 0 {
                snippet.push('…');
            }
            snippet.extend(chars[start..end].iter());
            if end < chars.len() {
                snippet.push('…');
            }
            snippet
        })
        .collect()
}

pub fn raster_scale(size: Rect, requested: f64) -> (f64, bool) {
    let requested = if requested.is_finite() {
        requested.clamp(0.01, 16.)
    } else {
        1.
    };
    let w = size.width * PX_PER_MM;
    let h = size.height * PX_PER_MM;
    // Leave room for ceil() and restrict both dimensions, including very narrow pages.
    let scale = requested
        .min(((MAX_PIXELS - 32000.) / (w * h)).sqrt())
        .min(15999. / w)
        .min(15999. / h);
    (scale, scale < requested)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod support {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/rofd-core/tests/support/mod.rs"
        ));
    }

    #[test]
    fn searches_across_runs_inside_nested_objects_and_normalizes_region() {
        let xml = r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Area><ofd:PhysicalBox>10 20 200 100</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="1"><ofd:PageBlock ID="2"><ofd:TextObject ID="3" Boundary="30 40 60 10" Font="5" Size="4"><ofd:TextCode X="0" Y="4">连续</ofd:TextCode><ofd:TextCode X="8" Y="4">搜索</ofd:TextCode><ofd:TextCode X="16" Y="4">搜索</ofd:TextCode></ofd:TextObject></ofd:PageBlock></ofd:Layer></ofd:Content></ofd:Page>"#;
        let document = r#"<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016"><ofd:CommonData><ofd:MaxUnitID>5</ofd:MaxUnitID><ofd:PublicRes>PublicRes.xml</ofd:PublicRes><ofd:PageArea><ofd:PhysicalBox>0 0 200 100</ofd:PhysicalBox></ofd:PageArea></ofd:CommonData><ofd:Pages><ofd:Page ID="6" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages></ofd:Document>"#;
        let resources = br#"<ofd:Res xmlns:ofd="http://www.ofdspec.org/2016"><ofd:Fonts><ofd:Font ID="5" FontName="sans-serif"/></ofd:Fonts></ofd:Res>"#;
        let bytes = support::ofd_with_document_page_and_entries(
            document,
            xml,
            &[("Doc_0/PublicRes.xml", resources)],
        );
        let doc =
            rofd_core::Document::from_bytes(bytes, rofd_core::LoadOptions::default()).unwrap();
        let page = doc.page(0).unwrap();
        let mut hits = Vec::new();
        assert!(search_page(&page, "续搜", &mut hits));
        assert_eq!(hits.len(), 1);
        assert_eq!(
            (hits[0].x, hits[0].y, hits[0].width, hits[0].height),
            (0.1, 0.2, 0.3, 0.1)
        );
        hits.clear();
        search_page(&page, "搜索", &mut hits);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn finds_chinese_and_case_insensitive_unicode_without_byte_slicing_panics() {
        assert_eq!(match_snippets("中文 Rust 中文", "中文", 10).len(), 2);
        assert_eq!(match_snippets("ÄBC Rust", "äbc", 10).len(), 1);
        assert_eq!(match_snippets("Rust rust RUST", "rust", 2).len(), 2);
        assert!(match_snippets("anything", "", 10).is_empty());
        assert!(match_snippets("anything", "absent", 10).is_empty());
    }

    #[test]
    fn snippet_centers_late_match_and_keeps_unicode_boundaries() {
        let text = format!("{}目标{}", "前".repeat(200), "后".repeat(200));
        let snippets = match_snippets(&text, "目标", 10);
        assert_eq!(snippets.len(), 1);
        assert!(snippets[0].contains("目标"));
        assert!(snippets[0].chars().count() <= 90);
    }

    #[test]
    fn bounds_huge_rasters_and_rejects_nonfinite_scale() {
        let size = Rect {
            x: 0.,
            y: 0.,
            width: 210.,
            height: 297.,
        };
        assert_eq!(raster_scale(size, 1.), (1., false));
        let (scale, reduced) = raster_scale(size, 8.);
        assert!(reduced);
        assert!(size.width * size.height * PX_PER_MM.powi(2) * scale.powi(2) <= MAX_PIXELS);
        assert!(raster_scale(size, f64::NAN).0.is_finite());
        let narrow = Rect {
            width: 1e9,
            height: 0.01,
            ..size
        };
        assert!(narrow.width * PX_PER_MM * raster_scale(narrow, 4.).0 <= 16000.);
    }
}
